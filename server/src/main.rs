use std::time::Duration;

use anyhow::{bail, Result};
use device_settings::services::DeviceSettings;
use heartbeat::service::Heatbeat;
use identity::service::Identity;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_otel_layer;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer,
};
use provisioning::service::Provisioning;
use sentry_tracing::{self, EventFilter};
use settings::AgentSettings;
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
#[tokio::main]
async fn main() -> Result<()> {
    let settings = match settings::read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };

    // setup sentry reporting
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
        match init_heartbeat_service().await {
            Ok(_) => (),
            Err(e) => bail!(e),
        };
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
                match init_device_settings_service().await {
                    Ok(_) => (),
                    Err(e) => bail!(e),
                };
            }
        }
        Ok(())
    });

    //init the GRPC server
    match init_grpc_server().await {
        Ok(_) => (),
        Err(e) => bail!(e),
    };

    Ok(())
}
