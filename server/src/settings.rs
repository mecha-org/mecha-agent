use crate::server_settings;
use dotenv::dotenv;
use provisioning::ProvisioningSettings;
use serde::{Deserialize, Serialize};
use std::{env, fs::File};
use telemetry::messaging::MessagingSettings;
use telemetry::TelemetrySettings;
const DEFAULT1: &str = "~/.mecha/agent/settings.yml";
const DEFAULT2: &str = "/etc/mecha/agent/settings.yml";
#[derive(Debug, Deserialize, Serialize)]
pub struct SentrySettings {
    pub enable: bool,
    pub dsn: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppSettings {
    pub server: server_settings::Settings,
    pub provisioning: ProvisioningSettings,
    pub telemetry: TelemetrySettings,
    pub messaging: MessagingSettings,
}

pub fn read_yml() -> Result<AppSettings, String> {
    dotenv().ok();
    let mut file_path = match env::var("MECHA_SETTINGS_PATH") {
        Ok(v) => v,
        Err(_e) => "".to_string(),
    };
    let mut file: Option<File> = None;

    if file_path.is_empty() {
        file_path = "settings.yml".to_string();
    }

    if let Ok(file_value) = File::open(file_path) {
        file = Some(file_value);
    } else if let Ok(file_value) = File::open(DEFAULT1) {
        file = Some(file_value);
    } else if let Ok(file_value) = File::open(DEFAULT2) {
        file = Some(file_value);
    } else {
        println!("File path incorretc for setting config");
    }

    let settings: AppSettings = serde_yaml::from_reader(file.unwrap()).unwrap();
    Ok(settings)
}
