use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MessagingAuthSettings {
    pub signed_key: SignedKeys,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignedKeys {
    pub privatekey: String,
}
