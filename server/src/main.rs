use std::{thread, time};
use anyhow::{bail, Result};
use futures::StreamExt;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer,
};
use messaging::Bytes;
use messaging::service::{Messaging, MessagingScope};
use sentry_tracing::{self, EventFilter};
use settings::AgentSettings;
use telemetry::service::TelemetryService;
use tracing::info;
use tonic::transport::Server;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

pub mod services;
pub mod errors;

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

use metrics::metrics_service_server::MetricsServiceServer;
use logs::logs_service_server::LogsServiceServer;
use trace::trace_service_server::TraceServiceServer;

use crate::agent::provisioning_service_server::ProvisioningServiceServer;
use crate::errors::{AgentServerError, AgentServerErrorCodes};
use crate::services::provisioning::ProvisioningServiceHandler;
use crate::services::telemetry::{TelemetryTraceHandler, TelemetryLogsHandler, TelemetryMetricsHandler};

async fn init_grpc_server() -> Result<()> {
    // TODO: pass settings from main()
    let server_settings = match settings::read_settings_yml() {
        Ok(v) => v.server,
        Err(_e) =>  AgentSettings::default().server,
    };
    let addr = format!("{}:{}", server_settings.url.unwrap_or(String::from("127.0.0.1")), server_settings.port)
        .parse()
        .unwrap();
    let provisioning_service = ProvisioningServiceHandler::default();
    let trace_service = TelemetryTraceHandler::default();
    let log_service = TelemetryLogsHandler::default();
    let metrics_service = TelemetryMetricsHandler::default();



    info!(
        task = "init_grpc_server",
        result = "success",
        "agent server listening on {} [grpc]", addr);

    match Server::builder()
        .add_service(ProvisioningServiceServer::new(provisioning_service))
        .add_service(MetricsServiceServer::new(metrics_service))
        .add_service(LogsServiceServer::new(log_service))
        .add_service(TraceServiceServer::new(trace_service))
        .serve(addr)
        .await {
            Ok(s) => s,
            Err(e) => bail!(AgentServerError::new(
                AgentServerErrorCodes::InitGRPCServerError,
                format!("error initializing grpc server - {}", e),
                true
            )),
        };
    Ok(())
}

async fn init_system_messaging_client() -> Result<Option<Messaging>> {
    let messaging_settings = match settings::read_settings_yml() {
        Ok(v) => v.messaging,
        Err(e) =>  AgentSettings::default().messaging,
    };

    // return none if system messaging is disabled
    if !messaging_settings.system.enabled {
        info!(target="init_system_messaging_client", "system messaging client is disabled");
        return Ok(None);
    }

    let mut messaging_client = Messaging::new(MessagingScope::System, true);
    let _ = match messaging_client.connect().await {
        Ok(s) => s,
        Err(_) => false, // TODO: dont stop the agent but add re-connection with exponential backoff
    };

    // subscribe
    tokio::task::spawn({
        let messaging_client = messaging_client.clone();
        async move {
            // subscribe to messages
            let mut subscriber = messaging_client.subscribe("foo".into()).await?;

            println!("Awaiting messages on foo");
            while let Some(message) = subscriber.next().await {
                println!("Received message {message:?}");
                println!("Received message header {:?}", message.headers);
            }
            Ok::<(), anyhow::Error>(())
        }
    });

 

    // // publish message
    // thread::sleep(time::Duration::from_secs(5));
    // let is_published = messaging_client.publish("foo", Bytes::from("bar1")).await?;
    // println!("Message published - {}", is_published);

    Ok(Some(messaging_client))
}

async fn init_telemtry() -> Result<Option<TelemetryService>> {
    let telemetry_settings = match settings::read_settings_yml() {
        Ok(v) => v.telemetry,
        Err(_e) =>  AgentSettings::default().telemetry,
    };

    // return none if system messaging is disabled
    if !telemetry_settings.enabled {
        info!(target="init_telemetry_otel_collector_service", "Telemetry collection is disabled");
        return Ok(None);
    }

    let mut telemetry_service = TelemetryService::new(telemetry_settings);

    // subscribe
    tokio::task::spawn({
        let telemetry_service = telemetry_service.clone();
        async move {
            // subscribe to messages
            let _ = match telemetry_service.init() {
                Ok(res) => res,
                Err(_e) => "Failed to start otel-collector".to_string()
            };
            
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

    // setup sentry reporting
    // enable the sentry exception reporting if enabled in settings and a DSN path is specified
    if settings.sentry.enabled && settings.sentry.dsn.is_some() {
        let sentry_path = settings.sentry.dsn.unwrap();
    
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
        .with(build_loglevel_filter_layer()) //temp for terminal log
        .with(build_logger_text()) //temp for terminal log
        .with(build_otel_layer().unwrap()); // trace collection layer
    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing::info!(
        //sample log
        task = "tracing_setup",
        result = "success",
        "tracing set up",
    );

    // start the agent services
    match init_system_messaging_client().await {
        Ok(_) => (),
        Err(e) => bail!(e),
    };

    match init_telemtry().await {
        Ok(_) => (),
        Err(e) => bail!(e),
    }

    match init_grpc_server().await {
        Ok(_) => (),
        Err(e) => bail!(e),
    };

    Ok(())
}
