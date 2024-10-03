use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetrySettings {
    pub enabled: bool,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self { enabled: false }
    }
}
