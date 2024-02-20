use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatusSettings {
    pub time_interval_sec: u64,
}

impl Default for StatusSettings {
    fn default() -> Self {
        Self {
            time_interval_sec: 60,
        }
    }
}
