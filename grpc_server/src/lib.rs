use crate::agent::identity_service_server::IdentityServiceServer;
use crate::agent::messaging_service_server::MessagingServiceServer;
use crate::agent::provisioning_service_server::ProvisioningServiceServer;
use crate::agent::settings_service_server::SettingsServiceServer;
use crate::errors::{AgentServerError, AgentServerErrorCodes};
use crate::services::identity::IdentityServiceHandler;
use crate::services::messaging::MessagingServiceHandler;
use crate::services::provisioning::ProvisioningServiceHandler;
use crate::services::settings::SettingsServiceHandler;
use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer,
};
use sentry_tracing::EventFilter;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use tokio::sync::mpsc;
use tonic::transport::Server;
use tracing::info;
use tracing_appender::non_blocking;
use tracing_appender::rolling::never;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::EnvFilter;
pub mod errors;
pub mod services;

pub mod agent {
    tonic::include_proto!("provisioning");
    tonic::include_proto!("settings");
    tonic::include_proto!("identity");
    tonic::include_proto!("messaging");
    // tonic::include_proto!("opentelemetry.proto.collector.metrics.v1");
    // tonic::include_proto!("opentelemetry.proto.collector.trace.v1");
    // tonic::include_proto!("opentelemetry.proto.collector.logs.v1");
}

pub struct GrpcServerOptions {
    pub provisioning_tx: mpsc::Sender<provisioning::handler::ProvisioningMessage>,
    pub identity_tx: mpsc::Sender<identity::handler::IdentityMessage>,
    pub messaging_tx: mpsc::Sender<messaging::handler::MessagingMessage>,
    pub settings_tx: mpsc::Sender<settings::handler::SettingMessage>,
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

    match Server::builder()
        .add_service(ProvisioningServiceServer::new(provisioning_service_handler))
        .add_service(IdentityServiceServer::new(identity_service_handler))
        .add_service(MessagingServiceServer::new(messaging_service_handler))
        .add_service(SettingsServiceServer::new(settings_service_handler))
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

pub async fn set_tracing() -> Result<()> {
    let settings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };
    // enable the sentry exception reporting if enabled in settings and a DSN path is specified
    if settings.clone().sentry.enabled && settings.clone().sentry.dsn.is_some() {
        let sentry_path = settings.clone().sentry.dsn.unwrap();

        let _guard = sentry::init((
            sentry_path,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                trim_backtraces: true,
                ..Default::default()
            },
        ));
    }

    let path = Path::new(settings.logging.path.as_str());
    let directory = path.parent().unwrap();
    let file_name = path.file_name().unwrap();
    let file_appender = never(directory, file_name);
    let (non_blocking_writer, _guard) = non_blocking(file_appender);
    // Set optional layer for logging to a file
    let layer = if settings.logging.enabled && !settings.logging.path.is_empty() {
        Some(
            Layer::new()
                .with_writer(non_blocking_writer)
                .with_ansi(false),
        )
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry()
        .with(layer)
        .with(EnvFilter::new(settings.logging.level.as_str()))
        .with(sentry_tracing::layer().event_filter(|_| EventFilter::Ignore))
        .with(build_loglevel_filter_layer()) //temp for terminal log
        .with(build_logger_text()) //temp for terminal log
        .with(build_otel_layer().unwrap()); // trace collection layer

    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => (),
        Err(e) => bail!(e),
    };

    tracing::info!(
        //sample log
        task = "tracing_setup",
        result = "success",
        "tracing set up",
    );
    Ok(())
}
