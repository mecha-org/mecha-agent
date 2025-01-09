use std::{
    ffi::OsStr,
    net::{IpAddr, SocketAddr},
    path::Path,
};

use agent_settings::{constants, read_settings_yml, GrpcSettings};
use anyhow::{bail, Result};
use clap::Parser;
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer, build_otel_layer,
};

use cli::cmd::{Reset, Setup, Whoami};
use mecha_agent::init::init_handlers;
use opentelemetry::global;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use telemetry::config::init_logs_config;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt::Layer, prelude::__tracing_subscriber_SubscriberExt};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Parser)]
struct StartCommand {
    /// Path to the settings file
    #[arg(short, long)]
    settings: String,

    /// Initialize the grpc server
    #[arg(long = "server")]
    init_grpc: bool,
}
#[derive(Debug, Parser)]
enum MectlCommand {
    #[command(about = "Start the agent")]
    Start(StartCommand),
    #[command(about = "Setup new machine")]
    Setup(Setup),
    #[command(about = "Machine details")]
    Whoami(Whoami),
    #[command(about = "Reset machine")]
    Reset(Reset),
}
#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "mectl")]
#[command(about = "mecha agent CLI", long_about = None)]
struct Mectl {
    #[command(subcommand)]
    command: MectlCommand,
}
#[tokio::main]
async fn main() -> Result<()> {
    // Setting tracing
    let mectl = Mectl::parse();
    match mectl.command {
        MectlCommand::Setup(configure) => {
            let settings_file_path = configure.settings.clone();
            let settings = read_settings_yml(Some(settings_file_path)).unwrap();
            let _ = configure
                .run(&settings.data.dir, &settings.backend.service)
                .await;
        }
        MectlCommand::Start(start) => {
            println!("Starting agent ... {:?}", start.settings);
            // configure the global logger to use our opentelemetry logger
            let _ = start_agent(start.settings, start.init_grpc).await;
        }
        MectlCommand::Whoami(whoami) => {
            let settings_file_path = whoami.settings.clone();
            let settings = read_settings_yml(Some(settings_file_path)).unwrap();
            let _ = whoami.run(&settings.data.dir).await;
        }
        MectlCommand::Reset(reset) => {
            let settings_file_path = reset.settings.clone();
            let settings = read_settings_yml(Some(settings_file_path)).unwrap();
            let _ = reset.run(&settings.data.dir).await;
        }
    }
    Ok(())
}

async fn start_agent(settings_path: String, init_grpc: bool) -> Result<()> {
    let settings = read_settings_yml(Some(settings_path)).unwrap();

    // Configure the global logger to use our opentelemetry logger
    let socket_addr = get_exporter_endpoint(&settings.grpc);
    let endpoint = format!("http://{}", socket_addr);
    let _ = init_logs_config(endpoint.as_str());
    let path = Path::new(settings.logging.path.as_str());

    //trying to separate a dir and file
    let directory = match path.parent() {
        Some(p) => p,
        None => Path::new(constants::DEFAULT_LOG_FILE_DIR),
    };
    let file_name = match path.file_name() {
        Some(f) => f,
        None => OsStr::new(constants::DEFAULT_LOG_FILE_NAME),
    };

    let file_appender = rolling::never(directory, file_name);
    let (non_blocking_writer, _guard) = non_blocking(file_appender);
    // Set optional layer for logging to a file
    let logs_to_file_layer = if settings.logging.enabled && !settings.logging.path.is_empty() {
        Some(
            Layer::new()
                .with_writer(non_blocking_writer)
                .with_ansi(false),
        )
    } else {
        None
    };

    let logger_provider = opentelemetry::global::logger_provider();
    let tracing_bridge_layer = OpenTelemetryTracingBridge::new(&logger_provider);
    global::set_logger_provider(logger_provider);

    let subscriber = tracing_subscriber::registry()
        .with(logs_to_file_layer)
        .with(tracing_bridge_layer)
        .with(build_loglevel_filter_layer()) //temp for terminal log
        .with(build_logger_text()) //temp for terminal log
        .with(build_otel_layer().unwrap());
    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => (),
        Err(e) => bail!("Error setting global default subscriber: {}", e),
    };
    tracing::info!(
        //sample log
        func = "set_tracing",
        package = env!("CARGO_PKG_NAME"),
        result = "success",
        "tracing set up",
    );
    let _ = init_handlers(settings, init_grpc, socket_addr).await;
    Ok(())
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
