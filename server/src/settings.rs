use axum::{http::StatusCode, response::Response};
use messaging_auth::settings::MessagingAuthSettings;
use provisioning::ProvisioningSettings;
use serde::{Deserialize, Serialize};
use std::{env, fs::File};

use crate::{server::ErrorResponse, server_settings};

#[derive(Debug, Deserialize, Serialize)]
pub struct SentrySettings {
    pub enable: bool,
    pub dsn: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppSettings {
    pub server: server_settings::Settings,
    pub provisioning: ProvisioningSettings,
    pub messaging_auth: MessagingAuthSettings,
}

pub fn read_yml() -> Result<AppSettings, Response<ErrorResponse>> {
    let mut file_path = match env::var("MECHA_SETTINGS_PATH") {
        Ok(v) => v,
        Err(_e) => "".to_string(),
    };

    if file_path.is_empty() {
        file_path = "settings.yml".to_string();
    }

    let response = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(ErrorResponse {
            ..Default::default()
        })
        .unwrap();

    let file = match File::open(file_path) {
        Ok(v) => v,
        Err(_e) => return Err(response),
    };
    let settings: AppSettings = serde_yaml::from_reader(file).unwrap();
    Ok(settings)
}
