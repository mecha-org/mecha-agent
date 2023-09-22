use anyhow::{bail, Result};
use futures::StreamExt;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer,
};
use messaging::service::{Messaging, MessagingScope};
use messaging::Bytes;
use sentry_tracing::{self, EventFilter};
use settings::AgentSettings;
use std::{thread, time};
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

pub mod errors;
pub mod services;

pub mod agent {
    tonic::include_proto!("provisioning");
}

use crate::agent::provisioning_service_server::ProvisioningServiceServer;
use crate::errors::{AgentServerError, AgentServerErrorCodes};
use crate::services::provisioning::ProvisioningServiceHandler;

async fn init_grpc_server() -> Result<()> {
    // TODO: pass settings from main()
    let server_settings = match settings::read_settings_yml() {
        Ok(v) => v.server,
        Err(e) => AgentSettings::default().server,
    };
    let addr = format!(
        "{}:{}",
        server_settings.url.unwrap_or(String::from("127.0.0.1")),
        server_settings.port
    )
    .parse()
    .unwrap();
    let provisioning_service = ProvisioningServiceHandler::default();

    info!(
        task = "init_grpc_server",
        result = "success",
        "agent server listening on {} [grpc]",
        addr
    );

    match Server::builder()
        .add_service(ProvisioningServiceServer::new(provisioning_service))
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

async fn init_system_messaging_client() -> Result<Option<Messaging>> {
    let messaging_settings = match settings::read_settings_yml() {
        Ok(v) => v.messaging,
        Err(_e) => AgentSettings::default().messaging,
    };

    // return none if system messaging is disabled
    if !messaging_settings.system.enabled {
        info!(
            target = "init_system_messaging_client",
            "system messaging client is disabled"
        );
        return Ok(None);
    }

    let mut messaging_client = Messaging::new(MessagingScope::System, true);
    let _ = match messaging_client.connect().await {
        Ok(s) => s,
        Err(e) => bail!(AgentServerError::new(
            AgentServerErrorCodes::InitMessagingClientError,
            format!("error initializing messaging client - {}", e),
            true
        )),
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
            }
            Ok::<(), anyhow::Error>(())
        }
    });

    // publish message
    thread::sleep(time::Duration::from_secs(5));
    let is_published = messaging_client.publish("foo", Bytes::from("bar1")).await?;
    println!("Message published - {}", is_published);

    Ok(Some(messaging_client))
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

    match init_grpc_server().await {
        Ok(_) => (),
        Err(e) => bail!(e),
    };

    Ok(())
}
