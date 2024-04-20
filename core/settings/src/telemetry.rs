use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetrySettingsCollect {
    pub system: bool,
    pub user: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetrySettings {
    pub enabled: bool,
    pub collect: TelemetrySettingsCollect,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            collect: TelemetrySettingsCollect {
                system: false,
                user: false,
            },
        }
    }
}
