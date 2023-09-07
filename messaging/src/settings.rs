use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MessagingSettings {
    pub mqtt: MQTTSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MQTTSettings {
    pub url: String,
    pub username: String,
    pub password: String,
    pub vendor: String,
}
