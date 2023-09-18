use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessagingSettings {
    pub system: MessagingNatsServer,
    pub user: MessagingNatsServer,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessagingNatsServer {
    pub url: String,
    pub enabled: bool,
}

impl Default for MessagingSettings {
    fn default() -> Self {
        Self {
            system: MessagingNatsServer {
                enabled: true,
                url: String::from("nats://127.0.0.1:4222")
            },
            user: MessagingNatsServer {
                enabled: false,
                url: String::from("nats://127.0.0.1:4222")
            }
        }
    }
}
