use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct NetworkingSettings {}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {}
    }
}
