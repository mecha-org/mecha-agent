use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HeartbeatSettings {
    pub time_interval_sec: u64,
}

impl Default for HeartbeatSettings {
    fn default() -> Self {
        Self {
            time_interval_sec: 60,
        }
    }
}
