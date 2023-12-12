use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct NetworkingSettings {
    pub enabled: bool,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}
