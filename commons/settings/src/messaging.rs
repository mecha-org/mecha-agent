use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessagingSettings {
}

impl Default for MessagingSettings {
    fn default() -> Self {
        Self {
        }
    }
}
