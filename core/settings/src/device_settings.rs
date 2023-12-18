use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeviceSettings {
    pub storage: StorageSettings,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct StorageSettings {
    #[serde(rename = "type")]
    pub r#type: String,
    pub path: String,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self {
            storage: StorageSettings {
                r#type: "file".to_string(),
                path: "/tmp".to_string(),
            },
        }
    }
}
