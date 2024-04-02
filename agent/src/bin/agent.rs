use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer, build_otel_layer,
};

use mecha_agent::init::init_services;
use opentelemetry::global;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use sentry_tracing::EventFilter;
use telemetry::config::init_logs_config;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
#[tokio::main]
async fn main() -> Result<()> {
    // Setting tracing
    let settings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(e) => {
            tracing::error!(
                func = "main",
                package = PACKAGE_NAME,
                "error reading settings.yml: {}",
                e
            );
            AgentSettings::default()
        }
    };

    // enable error tracking on sentry
    let _guard = sentry::init((
        settings.sentry.dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            attach_stacktrace: true,
            send_default_pii: false,
            server_name: Some("mecha-agent".to_string().into()),
            ..Default::default()
        },
    ));

    // configure the global logger to use our opentelemetry logger
    let _ = init_logs_config();
    let logger_provider = opentelemetry::global::logger_provider();
    let tracing_bridge_layer = OpenTelemetryTracingBridge::new(&logger_provider);
    global::set_logger_provider(logger_provider);

    let subscriber = tracing_subscriber::registry()
        .with(tracing_bridge_layer)
        .with(sentry_tracing::layer().event_filter(|_| EventFilter::Ignore))
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
    let _ = init_services().await;
    Ok(())
}
