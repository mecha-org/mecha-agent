use opentelemetry::logs::LogError;
use opentelemetry::{metrics, KeyValue};
use opentelemetry_otlp::{ExportConfig, WithExportConfig};
use opentelemetry_sdk::logs::{Config, LoggerProvider};
use opentelemetry_sdk::{metrics::MeterProvider, runtime, Resource};
use std::time::Duration;
pub fn init_otlp_configuration() -> metrics::Result<MeterProvider> {
    let export_config = ExportConfig {
        endpoint: "http://0.0.0.0:3001".to_string(),
        ..ExportConfig::default()
    };

    let duration = Duration::from_secs(60); // define duration to export metrics after this duration

    // let exporter = opentelemetry_otlp::new_exporter().
    opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_export_config(export_config),
        )
        .with_period(duration)
        .with_resource(Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            "basic-otlp-metrics-example",
        )]))
        .build()
}

pub fn init_logs_config() -> Result<opentelemetry_sdk::logs::Logger, LogError> {
    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_log_config(Config::default().with_resource(Resource::new(vec![
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                "mecha-agent-service",
            ),
            KeyValue::new("stream_name", "log_stream"),
        ])))
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://0.0.0.0:3001"),
        )
        .install_batch(runtime::Tokio)
}
