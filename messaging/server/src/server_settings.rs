use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Settings {
    pub url: String,
    pub port: i16,
    pub routes: RoutesSettings,
    pub tls: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RoutesSettings {
    pub health: String,
    pub provision_request: String,
    pub manifestation_request: String,
}
