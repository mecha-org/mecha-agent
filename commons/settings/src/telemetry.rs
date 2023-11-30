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
pub struct WriteLogsSettings {
    pub enabled: bool,
    pub path: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetrySettings {
    pub enabled: bool,
    pub collect: TelemetrySettingsCollect,
    pub otel_collector: TelemetryOtelCollector,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            collect: TelemetrySettingsCollect {
                system: false,
                user: false,
            },
            otel_collector: TelemetryOtelCollector {
                bin: "/etc/mecha/telemetry/otelcol-contrib".to_string(),
                conf: "/etc/mecha/telemetry/tracing-promql.yml".to_string(),
            },
        }
    }
}
