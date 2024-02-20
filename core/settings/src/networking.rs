use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct NetworkingSettings {
    pub enabled: bool,
    pub data_dir: String,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            data_dir: String::from(""),
        }
    }
}
