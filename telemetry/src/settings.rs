use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetrySettingsCollect {
    pub system: bool,
    pub user: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryOtelCollector {
    pub bin: String,
    pub conf: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetrySettings {
    pub enabled: bool,
    pub collect: TelemetrySettingsCollect,
    pub otel_collector: TelemetryOtelCollector,
}
