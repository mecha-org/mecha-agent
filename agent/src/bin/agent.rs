use std::{
    net::{IpAddr, SocketAddr},
    path::Path,
};
use tracing_subscriber::prelude::*;

use agent_settings::{read_settings_yml, AgentSettings, GrpcSettings, LoggingSettings};
use anyhow::{bail, Result};
use init_tracing_opentelemetry::tracing_subscriber_ext::{build_logger_text, build_otel_layer};

use mecha_agent::{
    errors::{AgentError, AgentErrorCodes},
    init::init_handlers,
};
use opentelemetry::global;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use telemetry::config::init_logs_config;
use tracing_appender::{non_blocking, rolling::never};
use tracing_subscriber::{fmt::Layer, prelude::__tracing_subscriber_SubscriberExt, EnvFilter};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
#[tokio::main]
async fn main() -> Result<()> {
    let settings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(e) => {
            println!("error reading settings.yml: {}", e);
            AgentSettings::default()
        }
    };

    println!("settings: {:?}", settings);
    // Configure the global logger to use our opentelemetry logger
    let socket_addr = get_exporter_endpoint(&settings.grpc);
    let otlp_collector_endpoint = format!("http://{}", socket_addr);
    match init_logs_config(otlp_collector_endpoint) {
        Ok(_) => (),
        Err(e) => {
            bail!(AgentError::new(
                AgentErrorCodes::InitLoggerError,
                format!("error initiating logger: {:?}", e)
            ));
        }
    }

    // Set global logger provider to export logs to GRPC
    let logger_provider = opentelemetry::global::logger_provider();
    let tracing_bridge_layer = OpenTelemetryTracingBridge::new(&logger_provider);
    global::set_logger_provider(logger_provider);

    // Set layer for logging to a file if it enabled in settings file and correct path is provided
    // let write_to_file_layer_with_settings_filter = build_write_to_file_layer(&settings.logging);
    let path = Path::new(settings.logging.path.as_str());
    let directory = path.parent().unwrap();
    let file_name = path.file_name().unwrap();
    println!("directory: {:?}, file_name: {:?}", directory, file_name);
    let file_appender = never(directory, file_name);
    let (non_blocking_writer, _guard) = non_blocking(file_appender);
    let write_to_file_layer = if settings.logging.enabled && !settings.logging.path.is_empty() {
        let env_filter = EnvFilter::new(settings.logging.level.as_str());
        Some(
            Layer::new()
                .with_writer(non_blocking_writer)
                .with_ansi(false)
                .with_filter(env_filter),
        )
    } else {
        None
    };

    // Set layer for logging to console if RUST_LOG provided in env file
    let console_logs_layer = if EnvFilter::try_from_default_env().is_ok() {
        // Set up console logging
        let console_log_level = EnvFilter::from_default_env();
        Some(build_logger_text().with_filter(console_log_level))
    } else {
        None
    };

    println!("logs level: {:?}", settings.logging.level.as_str());
    let subscriber = tracing_subscriber::registry()
        .with(tracing_bridge_layer)
        .with(write_to_file_layer)
        .with(console_logs_layer)
        .with(build_otel_layer().unwrap());
    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => (),
        Err(e) => bail!(AgentError::new(
            AgentErrorCodes::InitLoggerError,
            format!("error setting global default logger: {:?}", e),
        )),
    };
    tracing::info!(
        //sample log
        func = "set_tracing",
        package = env!("CARGO_PKG_NAME"),
        result = "success",
        "tracing set up",
    );
    let _ = init_handlers(settings, &socket_addr.to_string()).await;
    Ok(())
}

//TODO: discuss this with shoaib regarding lifespan
fn _build_write_to_file_layer(
    logging: &LoggingSettings,
) -> Option<
    tracing_subscriber::filter::Filtered<
        Layer<
            tracing_subscriber::layer::Layered<
                OpenTelemetryTracingBridge<global::GlobalLoggerProvider, global::logs::BoxedLogger>,
                tracing_subscriber::Registry,
            >,
            tracing_subscriber::fmt::format::DefaultFields,
            tracing_subscriber::fmt::format::Format,
            non_blocking::NonBlocking,
        >,
        EnvFilter,
        tracing_subscriber::layer::Layered<
            OpenTelemetryTracingBridge<global::GlobalLoggerProvider, global::logs::BoxedLogger>,
            tracing_subscriber::Registry,
        >,
    >,
> {
    // Set layer for logging to a file if it enabled in settings file and correct path is provided
    // TODO: if you put writer inside if block, it will be dropped after block ends
    let path = Path::new(logging.path.as_str());
    let directory = path.parent().unwrap();
    let file_name = path.file_name().unwrap();
    println!("directory: {:?}, file_name: {:?}", directory, file_name);
    let file_appender = never(directory, file_name);
    let (non_blocking_writer, _guard) = non_blocking(file_appender);
    let write_to_file_layer_with_settings_filter = if logging.enabled && !logging.path.is_empty() {
        let env_filter = EnvFilter::new(logging.level.as_str());
        Some(
            Layer::new()
                .with_writer(non_blocking_writer)
                .with_ansi(false)
                .with_filter(env_filter),
        )
    } else {
        None
    };

    write_to_file_layer_with_settings_filter
}

fn get_exporter_endpoint(server_settings: &GrpcSettings) -> SocketAddr {
    let ip: IpAddr = match server_settings.addr.parse() {
        Ok(ip) => ip,
        Err(e) => {
            tracing::error!(
                func = "get_exporter_endpoint",
                package = PACKAGE_NAME,
                "error parsing ip address: {}",
                e
            );
            IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
        }
    };
    let port: u16 = server_settings.port as u16;

    let socket_addr: SocketAddr = (ip, port).into();
    socket_addr
}
