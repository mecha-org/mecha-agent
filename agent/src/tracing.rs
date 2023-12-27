use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer, build_otel_layer,
};
use sentry_tracing::EventFilter;
use std::path::Path;
use tracing_appender::{non_blocking, rolling::never};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, EnvFilter};
pub fn set_tracing() -> Result<bool> {
    // Setting tracing
    let settings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };

    // enable error tracking on sentry
    let _guard = sentry::init((
        settings.sentry.dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            trim_backtraces: true,
            attach_stacktrace: true,
            send_default_pii: false,
            server_name: Some("mecha-agent".to_string().into()),
            ..Default::default()
        },
    ));

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
    Ok(true)
}
