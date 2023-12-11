use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessagingSettings {
    pub system: MessagingNatsServer,
    pub user: MessagingNatsServer,
    pub service_urls: ServiceUrlSettings
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ServiceUrlSettings {
    pub base_url: String,
    pub get_nonce: String,
    pub issue_auth_token: String,
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
            },
            service_urls: ServiceUrlSettings {
                base_url: String::from("http://127.0.0:3000"),
                get_nonce: String::from("/v1/auth/get_nonce"),
                issue_auth_token: String::from("/v1/auth/issue_token"),
        }
    }
    }
}
