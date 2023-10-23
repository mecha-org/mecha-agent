use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeviceSettings {
    pub storage: StorageSettings,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct StorageSettings {
    #[serde(rename = "type")]
    pub r#type: String,
    pub file_path: String,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            storage: StorageSettings {
                r#type: "file".to_string(),
                file_path: "/tmp".to_string(),
            },
        }
    }
}
