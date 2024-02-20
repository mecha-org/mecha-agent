use opentelemetry::{metrics, KeyValue};
use opentelemetry_otlp::{ExportConfig, WithExportConfig};
use opentelemetry_sdk::{metrics::MeterProvider, runtime, Resource};
use std::time::Duration;
pub fn init_otlp_configuration() -> metrics::Result<MeterProvider> {
    let export_config = ExportConfig {
        endpoint: "http://0.0.0.0:3001".to_string(),
        ..ExportConfig::default()
    };

    let duration = Duration::from_secs(20); // define duration to export metrics after this duration

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
