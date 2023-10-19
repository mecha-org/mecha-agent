use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeviceSettings {
    pub r#type: String,
    pub file_path: String,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            r#type: String::from("file"),
            file_path: String::from("~/.mecha/agent/storage/key_value_store"),
        }
    }
}
