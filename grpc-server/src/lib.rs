use crate::agent::identity_service_server::IdentityServiceServer;
use crate::agent::messaging_service_server::MessagingServiceServer;
use crate::agent::provisioning_service_server::ProvisioningServiceServer;
use crate::agent::settings_service_server::SettingsServiceServer;
use crate::errors::{AgentServerError, AgentServerErrorCodes};
use crate::services::identity::IdentityServiceHandler;
use crate::services::messaging::MessagingServiceHandler;
use crate::services::provisioning::ProvisioningServiceHandler;
use crate::services::settings::SettingsServiceHandler;
use crate::services::telemetry::TelemetryServiceHandler;
use crate::services::telemetry::{LogsAgent, MetricsAgent};
use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use tokio::sync::mpsc;
use tonic::transport::Server;
use tracing::{error, info};
pub mod errors;
pub mod services;

pub mod agent {
    tonic::include_proto!("provisioning");
    tonic::include_proto!("settings");
    tonic::include_proto!("identity");
    tonic::include_proto!("messaging");
}

pub mod metrics {
    tonic::include_proto!("opentelemetry.proto.collector.metrics.v1");
}
pub mod logs {
    tonic::include_proto!("opentelemetry.proto.collector.logs.v1");
}

use metrics::metrics_service_server::MetricsServiceServer;

use logs::logs_service_server::LogsServiceServer;

#[derive(Debug, Deserialize, Serialize)]
pub struct EncodeData {
    encoded: Vec<u8>,
    user_type: String,
}
pub struct GrpcServerOptions {
    pub provisioning_tx: mpsc::Sender<provisioning::handler::ProvisioningMessage>,
    pub identity_tx: mpsc::Sender<identity::handler::IdentityMessage>,
    pub messaging_tx: mpsc::Sender<messaging::handler::MessagingMessage>,
    pub settings_tx: mpsc::Sender<settings::handler::SettingMessage>,
    pub telemetry_tx: mpsc::Sender<telemetry::handler::TelemetryMessage>,
}

pub async fn start_grpc_service(opt: GrpcServerOptions) -> Result<()> {
    // TODO: pass settings from main()
    let settings = match read_settings_yml() {
        Ok(v) => v,
        Err(_e) => AgentSettings::default(),
    };

    // Construct socket address
    let ip: IpAddr = settings
        .grpc
        .addr
        .parse()
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    let port: u16 = settings.grpc.port as u16;

    let addr: SocketAddr = (ip, port).into();

    let provisioning_service_handler = ProvisioningServiceHandler::new(opt.provisioning_tx.clone());
    let identity_service_handler = IdentityServiceHandler::new(opt.identity_tx);
    let messaging_service_handler = MessagingServiceHandler::new(opt.messaging_tx);
    let settings_service_handler =
        SettingsServiceHandler::new(opt.settings_tx, opt.provisioning_tx);
    let telemetry_service_handler = TelemetryServiceHandler::new(opt.telemetry_tx);
    let logs_handler = LogsAgent {
        telemetry_service_handler: telemetry_service_handler.clone(),
    };
    let metrics_handler = MetricsAgent {
        telemetry_service_handler: telemetry_service_handler.clone(),
    };

    match Server::builder()
        .add_service(ProvisioningServiceServer::new(provisioning_service_handler))
        .add_service(IdentityServiceServer::new(identity_service_handler))
        .add_service(MessagingServiceServer::new(messaging_service_handler))
        .add_service(SettingsServiceServer::new(settings_service_handler))
        .add_service(LogsServiceServer::new(logs_handler))
        .add_service(MetricsServiceServer::new(metrics_handler))
        .serve(addr)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = "start_grpc_service",
                package = env!("CARGO_PKG_NAME"),
                "error initializing grpc server - {}",
                e
            );
            bail!(AgentServerError::new(
                AgentServerErrorCodes::InitGRPCServerError,
                format!("error initializing grpc server - {}", e),
            ))
        }
    };
    info!(
        func = "start_grpc_service",
        package = env!("CARGO_PKG_NAME"),
        "grpc server started on - {}",
        addr
    );
    Ok(())
}
