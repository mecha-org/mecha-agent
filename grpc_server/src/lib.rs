use crate::agent::identity_service_server::IdentityServiceServer;
use crate::agent::messaging_service_server::MessagingServiceServer;
use crate::agent::provisioning_service_server::ProvisioningServiceServer;
use crate::agent::settings_service_server::SettingsServiceServer;
use crate::errors::{AgentServerError, AgentServerErrorCodes};
use crate::services::identity::IdentityServiceHandler;
use crate::services::messaging::MessagingServiceHandler;
use crate::services::provisioning::ProvisioningServiceHandler;
use crate::services::settings::SettingsServiceHandler;
use crate::services::telemetry::LogsAgent;
use crate::services::telemetry::TelemetryServiceHandler;
use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use tokio::sync::mpsc;
use tonic::transport::Server;
use tracing::info;
pub mod errors;
pub mod services;

pub mod agent {
    tonic::include_proto!("provisioning");
    tonic::include_proto!("settings");
    tonic::include_proto!("identity");
    tonic::include_proto!("messaging");
    // tonic::include_proto!("opentelemetry.proto.collector.metrics.v1");
    // tonic::include_proto!("opentelemetry.proto.collector.trace.v1");
    tonic::include_proto!("opentelemetry.proto.collector.logs.v1");
}

// use agent::{
//     metrics_service_server::{MetricsService, MetricsServiceServer},
//     ExportMetricsServiceRequest, ExportMetricsServiceResponse,
// };

use agent::logs_service_server::LogsServiceServer;
// use agent::{
//     trace_service_server::{TraceService, TraceServiceServer},
//     ExportTraceServiceRequest, ExportTraceServiceResponse,
// };

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
        .server
        .url
        .unwrap_or(String::from("127.0.0.1"))
        .parse()
        .unwrap();
    let port: u16 = settings.server.port as u16;

    let addr: SocketAddr = (ip, port).into();

    info!(
        task = "init_grpc_server",
        result = "success",
        "agent server listening on {} [grpc]",
        addr
    );

    let provisioning_service_handler = ProvisioningServiceHandler::new(opt.provisioning_tx.clone());
    let identity_service_handler = IdentityServiceHandler::new(opt.identity_tx);
    let messaging_service_handler = MessagingServiceHandler::new(opt.messaging_tx);
    let settings_service_handler =
        SettingsServiceHandler::new(opt.settings_tx, opt.provisioning_tx);
    let telemetry_service_handler = TelemetryServiceHandler::new(opt.telemetry_tx);
    let logs_handler = LogsAgent {
        telemetry_service_handler: telemetry_service_handler,
    };

    match Server::builder()
        .add_service(ProvisioningServiceServer::new(provisioning_service_handler))
        .add_service(IdentityServiceServer::new(identity_service_handler))
        .add_service(MessagingServiceServer::new(messaging_service_handler))
        .add_service(SettingsServiceServer::new(settings_service_handler))
        .add_service(LogsServiceServer::new(logs_handler))
        // .add_service(MetricsServiceServer::new(metric_server))
        // .add_service(TraceServiceServer::new(trace_server))
        .serve(addr)
        .await
    {
        Ok(s) => s,
        Err(e) => bail!(AgentServerError::new(
            AgentServerErrorCodes::InitGRPCServerError,
            format!("error initializing grpc server - {}", e),
            true
        )),
    };
    Ok(())
}
