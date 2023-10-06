use anyhow::{bail, Result};
use heartbeat::service::Heatbeat;
use identity::service::Identity;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer,
};
use provisioning::service::Provisioning;
use sentry_tracing::{self, EventFilter};
use settings::AgentSettings;
use std::{thread, time};
use telemetry::service::TelemetryService;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

pub mod errors;
pub mod services;

pub mod agent {
    tonic::include_proto!("provisioning");
}

pub mod metrics {
    tonic::include_proto!("opentelemetry.proto.collector.metrics.v1");
}

pub mod trace {
    tonic::include_proto!("opentelemetry.proto.collector.trace.v1");
}

pub mod logs {
    tonic::include_proto!("opentelemetry.proto.collector.logs.v1");
}

use logs::logs_service_server::LogsServiceServer;
use metrics::metrics_service_server::MetricsServiceServer;
use trace::trace_service_server::TraceServiceServer;

use crate::agent::provisioning_service_server::ProvisioningServiceServer;
use crate::errors::{AgentServerError, AgentServerErrorCodes};
use crate::services::provisioning::ProvisioningServiceHandler;
use crate::services::telemetry::{
    TelemetryLogsHandler, TelemetryMetricsHandler, TelemetryTraceHandler,
};

async fn init_grpc_server() -> Result<()> {
    // TODO: pass settings from main()
    let server_settings = match settings::read_settings_yml() {
        Ok(v) => v.server,
        Err(_e) => AgentSettings::default().server,
    };
    let addr = format!(
        "{}:{}",
        server_settings.url.unwrap_or(String::from("127.0.0.1")),
        server_settings.port
    )
    .parse()
    .unwrap();
    let provisioning_service = ProvisioningServiceHandler::default();
    let trace_service = TelemetryTraceHandler::default();
    let log_service = TelemetryLogsHandler::default();
    let metrics_service = TelemetryMetricsHandler::default();

    info!(
        task = "init_grpc_server",
        result = "success",
        "agent server listening on {} [grpc]",
        addr
    );

    match Server::builder()
        .add_service(ProvisioningServiceServer::new(provisioning_service))
        .add_service(MetricsServiceServer::new(metrics_service))
        .add_service(LogsServiceServer::new(log_service))
        .add_service(TraceServiceServer::new(trace_service))
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

async fn init_provisioning_service() -> Result<bool> {
    println!("init_provisioning_service");
    let agent_settings = match settings::read_settings_yml() {
        Ok(v) => v,
        Err(_e) => AgentSettings::default(),
    };

    // initiate heartbeat client
    let provisioning_service = Provisioning::new(agent_settings.provisioning.clone());
    let code_result = provisioning_service.generate_code();
    match code_result {
        Ok(code) => println!("code: {}", code),
        Err(e) => bail!(e),
    };

    Ok(true)
}
async fn init_heartbeat_client() -> Result<bool> {
    println!("init_heartbeat_client");
    let agent_settings = match settings::read_settings_yml() {
        Ok(v) => v,
        Err(_e) => AgentSettings::default(),
    };

    println!("agent_settings: completed");
    // return none if system messaging is disabled
    if !agent_settings.messaging.system.enabled {
        info!(
            target = "init_heartbeat_client",
            "system messaging client is disabled"
        );
        return Ok(false);
    }

    // initiate heartbeat client
    let heartbeat_client = Heatbeat::new(agent_settings.clone());
    let _ = heartbeat_client.start().await;

    Ok(true)
}

async fn init_telemtry() -> Result<Option<TelemetryService>> {
    let telemetry_settings = match settings::read_settings_yml() {
        Ok(v) => v.telemetry,
        Err(_e) => AgentSettings::default().telemetry,
    };

    if !telemetry_settings.enabled {
        info!(
            target = "init_telemetry_otel_collector_service",
            "Telemetry collection is disabled"
        );
    }

    let telemetry_service = TelemetryService::new(telemetry_settings).await;

    tokio::task::spawn({
        let telemetry_service = telemetry_service.clone();
        async move {
            telemetry_service.start_telemetry().await;
            Ok::<(), anyhow::Error>(())
        }
    });
    Ok(Some(telemetry_service))
}

#[tokio::main]
async fn main() -> Result<()> {
    let settings = match settings::read_settings_yml() {
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

    // TODO: logging to an output file
    // start the tracing service
    let subscriber = tracing_subscriber::registry()
        .with(sentry_tracing::layer().event_filter(|_| EventFilter::Ignore))
        // .with(build_loglevel_filter_layer()) //temp for terminal log
        // .with(build_logger_text()) //temp for terminal log
        .with(build_otel_layer().unwrap()); // trace collection layer
    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing::info!(
        //sample log
        task = "tracing_setup",
        result = "success",
        "tracing set up",
    );

    //step1: check if provisioning is complete
    let identity_client = Identity::new(settings.clone());
    let is_provisioned = match identity_client.is_device_provisioned() {
        Ok(v) => v,
        Err(e) => bail!(e),
    };

    //step2: if not complete, start GRPC and the provisioning service
    if !is_provisioned {
        match init_provisioning_service().await {
            Ok(_) => (),
            Err(e) => bail!(e),
        };
    } else {
        //step3: if complete, start the heartbeat service
        match init_heartbeat_client().await {
            Ok(_) => (),
            Err(e) => bail!(e),
        };
    }

    // `init the telemetryService
    match init_telemtry().await {
        Ok(_) => (),
        Err(e) => bail!(e),
    };
    //init the GRPC server
    match init_grpc_server().await {
        Ok(_) => (),
        Err(e) => bail!(e),
    };

    Ok(())
}
