use std::time::Duration;

use anyhow::{bail, Result};
use device_settings::services::DeviceSettings;
use heartbeat::service::Heatbeat;
use identity::service::Identity;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer,
};
use messaging::service::{Messaging, MessagingScope};
use networking::service::Networking;
use provisioning::service::Provisioning;
use sentry_tracing::{self, EventFilter};
use settings::AgentSettings;
use telemetry::errors::{TelemetryError, TelemetryErrorCodes};
use telemetry::service::TelemetryService;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

pub mod errors;
pub mod services;

pub mod agent {
    tonic::include_proto!("provisioning");
    tonic::include_proto!("device_settings");
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

use crate::agent::device_setting_service_server::DeviceSettingServiceServer;
use crate::agent::provisioning_service_server::ProvisioningServiceServer;
use crate::errors::{AgentServerError, AgentServerErrorCodes};
use crate::services::device_settings::DeviceSettingServiceHandler;
use crate::services::provisioning::ProvisioningServiceHandler;
use crate::services::telemetry::{
    TelemetryLogsHandler, TelemetryMetricsHandler, TelemetryTraceHandler,
};
use logs::logs_service_server::LogsServiceServer;
use metrics::metrics_service_server::MetricsServiceServer;
use trace::trace_service_server::TraceServiceServer;

async fn init_grpc_server() -> Result<()> {
    // TODO: pass settings from main()
    let settings = match settings::read_settings_yml() {
        Ok(v) => v,
        Err(_e) => AgentSettings::default(),
    };

    //initiate messaging service and publish a message
    let mut messaging_client =
        Messaging::new(MessagingScope::System, settings.messaging.system.enabled);
    let _ = match messaging_client.connect().await {
        Ok(s) => s,
        Err(e) => bail!(TelemetryError::new(
            TelemetryErrorCodes::InitMessagingClientError,
            format!("error initializing messaging client - {}", e),
            true
        )),
    };

    let addr = format!(
        "{}:{}",
        settings.server.url.unwrap_or(String::from("127.0.0.1")),
        settings.server.port
    )
    .parse()
    .unwrap();
    let provisioning_service = ProvisioningServiceHandler::default();
    let trace_service = TelemetryTraceHandler {
        messaging_client: messaging_client.clone(),
    };
    let log_service = TelemetryLogsHandler {
        messaging_client: messaging_client.clone(),
    };
    let metrics_service = TelemetryMetricsHandler {
        messaging_client: messaging_client.clone(),
    };
    let device_settings_service = DeviceSettingServiceHandler::default();

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
        .add_service(DeviceSettingServiceServer::new(device_settings_service))
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
async fn init_heartbeat_service() -> Result<bool> {
    let agent_settings = match settings::read_settings_yml() {
        Ok(v) => v,
        Err(_e) => AgentSettings::default(),
    };

    // return none if system messaging is disabled
    if !agent_settings.messaging.system.enabled {
        info!(
            target = "init_heartbeat_service",
            "system messaging client is disabled"
        );
        return Ok(false);
    }

    // initiate heartbeat client
    let heartbeat_client = Heatbeat::new(agent_settings.clone());
    let _ = heartbeat_client.start().await;

    Ok(true)
}

async fn init_telemetry() -> Result<bool> {
    let agent_settings = match settings::read_settings_yml() {
        Ok(v) => v,
        Err(_e) => AgentSettings::default(),
    };

    if !agent_settings.telemetry.enabled {
        info!(
            target = "init_telemetry_otel_collector_service",
            "Telemetry collection is disabled"
        );
    }

    let _ = TelemetryService::telemetry_init(agent_settings);
    info!(
        target = "init_telemetry_otel_collector_service",
        "telemetry services started"
    );
    Ok(true)
}

async fn init_device_settings_service() -> Result<bool> {
    let agent_settings = match settings::read_settings_yml() {
        Ok(v) => v,
        Err(_e) => AgentSettings::default(),
    };

    // initiate heartbeat client
    let device_settings_service = DeviceSettings::new(agent_settings.clone());
    let _ = device_settings_service.start().await;

    Ok(true)
}

async fn init_networking_service() -> Result<bool> {
    let agent_settings = match settings::read_settings_yml() {
        Ok(v) => v,
        Err(_e) => AgentSettings::default(),
    };

    // return false if networking is disabled
    if !agent_settings.networking.enabled {
        info!(
            target = "init_networking_service",
            "networking service is disabled"
        );
        return Ok(false);
    }

    // initiate networking service
    let networking_service = Networking::new(agent_settings.clone());
    let _ = networking_service.start().await;

    Ok(true)
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
    // let filter = Targets::new().with_target("networking", LevelFilter::TRACE);

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

    // Start the gRPC server in its own task
    let start_grpc = tokio::spawn(async move {
        if let Err(e) = init_grpc_server().await {
            eprintln!("Error initializing GRPC server: {:?}", e);
        } else {
            println!("GRPC server started successfully!");
        }
    });

    let start_services = tokio::spawn(async move {
        if let Err(e) = start_services(settings).await {
            eprintln!("Error initializing services: {:?}", e);
        } else {
            println!("Services started successfully!");
        }
    });
    tokio::join!(start_grpc, start_services);
    Ok(())
}

async fn start_services(settings: AgentSettings) -> Result<()> {
    //step1: check if provisioning is complete
    let identity_client = Identity::new(settings.clone());
    let mut is_provisioned = match identity_client.is_device_provisioned() {
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
        //start networking service on its own
        let _ = tokio::spawn(async move {
            if let Err(e) = init_networking_service().await {
                eprintln!("error initializing networking service: {:?}", e);
            } else {
                println!("networking service started successfully!");
            }
        });
        match init_heartbeat_service().await {
            Ok(_) => (),
            Err(e) => bail!(e),
        };
        match init_telemetry().await {
            Ok(_) => (),
            Err(e) => bail!(e),
        }
        match init_device_settings_service().await {
            Ok(_) => (),
            Err(e) => bail!(e),
        };
    }

    let _result = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        while !is_provisioned {
            interval.tick().await; // This should go first.
            is_provisioned = match identity_client.is_device_provisioned() {
                Ok(v) => v,
                Err(e) => bail!(e),
            };
            if is_provisioned {
                match init_heartbeat_service().await {
                    Ok(_) => (),
                    Err(e) => bail!(e),
                };
                match init_telemetry().await {
                    Ok(_) => (),
                    Err(e) => bail!(e),
                }
                match init_device_settings_service().await {
                    Ok(_) => (),
                    Err(e) => bail!(e),
                };
            }
        }
        Ok(())
    });
    Ok(())
}
