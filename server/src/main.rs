use anyhow::{bail, Result};
// use futures::executor::block_on;
// use futures::join;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer,
};
use sentry_tracing::{self, EventFilter};
use tracing::info;
// use std::thread;
use tonic::transport::Server;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

pub mod settings;
pub mod services;

pub mod agent {
    tonic::include_proto!("provisioning");
}

use crate::agent::provisioning_service_server::ProvisioningServiceServer;
use crate::services::provisioning::ProvisioningServiceHandler;
use crate::settings::AgentSettings;

pub async fn init_server() -> Result<(), String> {
    // TODO: pass settings from main()
    let server_settings = match settings::read_settings_yml() {
        Ok(v) => v.server,
        Err(_e) =>  AgentSettings::default().server
    };
    let addr = format!("{}:{}", server_settings.url.unwrap_or(String::from("127.0.0.1")), server_settings.port)
        .parse()
        .unwrap();
    let provisioning_service = ProvisioningServiceHandler::default();

    info!(
        task = "init_server",
        result = "success",
        "agent server listening on {} [grpc]", addr);

    let _ = Server::builder()
        .add_service(ProvisioningServiceServer::new(provisioning_service))
        .serve(addr)
        .await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let settings = match settings::read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };

    // Setting up the Sentry Reporter
    // Enable the sentry exception reporting if enabled in settings and a DSN path is specified
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

    match init_server().await {
        Ok(_) => (),
        Err(e) => bail!(e),
    };

    // // Spawn the grpc server
    // let t1 = thread::spawn(init_server);
    // // block_on(async { join!(t1.join().unwrap()) });
    Ok(())
}
