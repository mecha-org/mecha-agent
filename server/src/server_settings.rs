use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Settings {
    pub url: String,
    pub port: i16,
    pub tls: bool,
}
