use crate::{
    messaging::MessagingSettings, provisioning::ProvisioningSettings, telemetry::TelemetrySettings,
};
use anyhow::{bail, Result};
use dotenv::dotenv;
use networking::NetworkingSettings;
use serde::{Deserialize, Serialize};
use settings::Settings;
use status::StatusSettings;
use std::{env, fmt, fs::File, path::PathBuf};
use tracing::error;
pub mod messaging;
pub mod networking;
pub mod provisioning;
pub mod settings;
pub mod status;
pub mod telemetry;

/// Agent Settings - Struct corresponding to the settings.yml schema
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AgentSettings {
    pub sentry: SentrySettings,
    pub server: ServerSettings,
    pub logging: LoggingSettings,
    pub provisioning: ProvisioningSettings,
    pub messaging: MessagingSettings,
    pub status: StatusSettings,
    pub settings: Settings,
    pub networking: NetworkingSettings,
    pub telemetry: TelemetrySettings,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            sentry: SentrySettings::default(),
            server: ServerSettings::default(),
            logging: LoggingSettings::default(),
            provisioning: ProvisioningSettings::default(),
            messaging: MessagingSettings::default(),
            status: StatusSettings::default(),
            settings: Settings::default(),
            networking: NetworkingSettings::default(),
            telemetry: TelemetrySettings::default(),
        }
    }
}

/// SentrySettings - Settings parameter for configuring the sentry client
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SentrySettings {
    pub enabled: bool,
    pub dsn: Option<String>,
}

impl Default for SentrySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            dsn: None,
        }
    }
}

/// ServerSettings - Settings parameter for configuring the agent's grpc server settings
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ServerSettings {
    pub url: Option<String>,
    pub port: i16,
    pub tls: bool,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            url: Some(String::from("127.0.0.1")),
            port: 3001,
            tls: false,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LoggingSettings {
    pub enabled: bool,
    pub level: String,
    pub path: String,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            level: String::from("info"),
            path: String::from(""),
        }
    }
}
/// # Agent Error Codes
///
/// Implements standard errors for the agent
#[derive(Debug, Default, Clone, Copy)]
pub enum SettingsErrorCodes {
    #[default]
    UnknownError,
    ReadError,
    ParseError,
}

impl fmt::Display for SettingsErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SettingsErrorCodes::UnknownError => write!(f, "SettingsErrorCodes: UnknownError"),
            SettingsErrorCodes::ReadError => write!(f, "SettingsErrorCodes: ReadError"),
            SettingsErrorCodes::ParseError => write!(f, "SettingsErrorCodes: ParseError"),
        }
    }
}

/// # SettingsError
///
/// Implements a standard error type for all agent related errors
/// includes the error code (`SettingsErrorCodes`) and a message
#[derive(Debug, Default)]
pub struct SettingsError {
    pub code: SettingsErrorCodes,
    pub message: String,
}

impl SettingsError {
    pub fn new(code: SettingsErrorCodes, message: String) -> Self {
        error!("Error: (code: {:?}, message: {})", code, message);
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(code: {:?}, message: {})", self.code, self.message)
    }
}

/// # Reads Settings path from arg
///
/// Reads the `-s` or `--settings` argument for the path
pub fn read_settings_path_from_args() -> Option<String> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 && (args[1] == "-s" || args[1] == "--settings") {
        return Some(String::from(args[2].clone()));
    }
    None
}

/// # Reads Settings YML
///
/// Reads the `settings.yml` and parsers to AgentSettings
///
/// **Important**: Ensure all fields are present in the yml due to strict parsing
pub fn read_settings_yml() -> Result<AgentSettings> {
    dotenv().ok();
    // Add schema validator for yml
    let mut file_path = PathBuf::from(
        std::env::var("MECHA_AGENT_SETTINGS_PATH")
            .unwrap_or(String::from("~/.mecha/agent/settings.yml")),
    ); // Get path of the library

    // TODO: handle semver version support

    // read from args
    let file_path_in_args = read_settings_path_from_args();
    if file_path_in_args.is_some() {
        file_path = PathBuf::from(file_path_in_args.unwrap());
    }

    // open file
    let settings_file_handle = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            // TODO: add capture
            bail!(SettingsError::new(
                SettingsErrorCodes::ReadError,
                format!(
                    "error read the settings.yml in the path - {}",
                    e.to_string()
                ),
            ));
        }
    };

    // read and parse
    let config: AgentSettings = match serde_yaml::from_reader(settings_file_handle) {
        Ok(config) => config,
        Err(e) => {
            bail!(SettingsError::new(
                SettingsErrorCodes::ParseError,
                format!("error parsing the settings.yml - {}", e.to_string()),
            ));
        }
    };

    Ok(config)
}
