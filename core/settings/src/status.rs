use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatusSettings {
    pub enabled: bool,
    pub interval: u64,
}

impl Default for StatusSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: 60,
        }
    }
}
