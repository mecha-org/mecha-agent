use crate::errors::{ScreenError, ScreenErrorCodes};
use anyhow::bail;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{env, fs::File, path::PathBuf};
use tracing::debug;

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ScreenSettings {
    pub window: WindowSettings, // Window Settings
    pub modules: Modules,
    pub widget_configs: WidgetConfigs,
    pub css: CssConfigs,
}

impl Default for ScreenSettings {
    fn default() -> Self {
        Self {
            window: WindowSettings::default(),
            modules: Modules::default(),
            css: CssConfigs::default(),
            widget_configs: WidgetConfigs::default(),
        }
    }
}

/// # Window Settings
///
/// Part of the settings.yml to control the behavior of
/// the application window
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WindowSettings {
    pub size: (i32, i32),             // Size of the window
    pub position: (i32, i32),         // Default position to start window
    pub min_size: Option<(u32, u32)>, // Minimum size the window can be resized to
    pub max_size: Option<(u32, u32)>, // Maximum size the window can be resized to
    pub visible: bool,                // Sets visibility of the window
    pub resizable: bool,              // Enables or disables resizing
    pub decorations: bool,            // Enables or disables the title bar
    pub transparent: bool,            // Enables transparency
    pub always_on_top: bool,          // Forces window to be always on top
    pub icon_path: Option<String>,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            size: (480, 480),
            position: (0, 0),
            min_size: None,
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            always_on_top: false,
            icon_path: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Modules {
    pub pages_settings: PagesSettings,
}
impl Default for Modules {
    fn default() -> Self {
        Self {
            pages_settings: PagesSettings::default(),
        }
    }
}

/// # Custom Widgets config
///
/// Custom Widgets config
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WidgetConfigs {
    pub footer: FooterWidgetConfigs,
}

impl Default for WidgetConfigs {
    fn default() -> Self {
        Self {
            footer: FooterWidgetConfigs::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FooterWidgetConfigs {
    pub back_icon: Option<String>,
    pub next_icon: Option<String>,
    pub settings_icon: Option<String>,
    pub refresh_icon: Option<String>,
    pub exit_icon: Option<String>,
    pub trash_icon: Option<String>,
}

impl Default for FooterWidgetConfigs {
    fn default() -> Self {
        Self {
            back_icon: None,
            next_icon: None,
            settings_icon: None,
            refresh_icon: None,
            exit_icon: None,
            trash_icon: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct PagesSettings {
    pub start_screen: StartScreenSettings,
    pub check_internet: CheckInternetSettings,
    pub no_internet: NoInternetSettings,
    pub configure_machine: ConfigureMachineSettings,
    pub setup_success: SetupSuccessSettings,
    pub setup_failure: SetupFailedSettings,
    pub device_info: DeviceInfoSettings,
    pub timeout_screen: TimeoutScreenSettings,
}

impl Default for PagesSettings {
    fn default() -> Self {
        Self {
            start_screen: StartScreenSettings::default(),
            check_internet: CheckInternetSettings::default(),
            no_internet: NoInternetSettings::default(),
            configure_machine: ConfigureMachineSettings::default(),
            setup_success: SetupSuccessSettings::default(),
            setup_failure: SetupFailedSettings::default(),
            device_info: DeviceInfoSettings::default(),
            timeout_screen: TimeoutScreenSettings::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct StartScreenSettings {
    pub app_icon: Option<String>,
    pub virtual_network_icon: Option<String>,
    pub real_time_icon: Option<String>,
    pub encypt_icon: Option<String>,
    pub info_icon: Option<String>,
}

impl Default for StartScreenSettings {
    fn default() -> Self {
        Self {
            app_icon: None,
            virtual_network_icon: None,
            real_time_icon: None,
            encypt_icon: None,
            info_icon: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TimeoutScreenSettings {
    pub timeout_image: Option<String>,
}

impl Default for TimeoutScreenSettings {
    fn default() -> Self {
        Self {
            timeout_image: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CheckInternetSettings {
    pub search_wifi: Option<String>,
}

impl Default for CheckInternetSettings {
    fn default() -> Self {
        Self { search_wifi: None }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct NoInternetSettings {
    pub no_internet_found: Option<String>,
}

impl Default for NoInternetSettings {
    fn default() -> Self {
        Self {
            no_internet_found: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ConfigureMachineSettings {
    pub machine_searching: Option<String>,
}

impl Default for ConfigureMachineSettings {
    fn default() -> Self {
        Self {
            machine_searching: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SetupSuccessSettings {
    pub success: Option<String>,
}

impl Default for SetupSuccessSettings {
    fn default() -> Self {
        Self { success: None }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SetupFailedSettings {
    pub failure: Option<String>,
}

impl Default for SetupFailedSettings {
    fn default() -> Self {
        Self { failure: None }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeviceInfoSettings {
    pub user_profile_img: Option<String>,
    pub active_status_icon: Option<String>,
    pub not_active_status_icon: Option<String>,
}

impl Default for DeviceInfoSettings {
    fn default() -> Self {
        Self {
            user_profile_img: None,
            active_status_icon: None,
            not_active_status_icon: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CssConfigs {
    pub default: String,
}

impl Default for CssConfigs {
    fn default() -> Self {
        Self {
            default: "".to_string(),
        }
    }
}

/// # Reads Settings path from arg
///
/// Reads the `-s` or `--settings` argument for the path
pub fn read_settings_path_from_args() -> Option<String> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 && (args[1] == "-s" || args[1] == "--settings") {
        println!("Using settings path from argument - {}", args[2]);
        debug!("Using settings path from argument - {}", args[2]);
        return Some(String::from(args[2].clone()));
    }
    None
}

/// # Reads Settings YML
///
/// Reads the `settings.yml` and parsers to pages/screens
///
/// **Important**: Ensure all fields are present in the yml due to strict parsing
pub fn read_settings_yml() -> Result<ScreenSettings> {
    let mut file_path = PathBuf::from(
        std::env::var("MECHA_CONNECT_APP_SETTINGS_PATH").unwrap_or(String::from("settings.yml")),
    ); // Get path of the library
    println!(
        "settings::mod::read_settings_yml-file_path : {:?}",
        file_path
    );

    // read from args
    let file_path_in_args = read_settings_path_from_args();
    if file_path_in_args.is_some() {
        file_path = PathBuf::from(file_path_in_args.unwrap());
        println!(
            "IF read_settings_yml::settings file location - {:?}",
            file_path
        );
    }
    println!(
        "read_settings_yml::settings file location - {:?}",
        file_path
    );
    tracing::info!(
        func = "read_settings",
        package = env!("CARGO_PKG_NAME"),
        "settings file location - {:?}",
        file_path,
    );

    tracing::info!("CHECKING LOGS");

    // open file
    let settings_file_handle = match File::open(file_path) {
        Ok(file) => {
            tracing::info!(
                func = "read_settings",
                package = env!("CARGO_PKG_NAME"),
                "settings_file_handle::open file"
            );
            file
        }
        Err(e) => {
            eprintln!("cannot read the settings.yml in the path - {}", e);
            bail!(ScreenError::new(
                ScreenErrorCodes::SettingsReadError,
                format!("cannot read the settings.yml in the path - {}", e),
            ));
        }
    };

    // read and parse
    let config: ScreenSettings = match serde_yaml::from_reader(settings_file_handle) {
        Ok(config) => config,
        Err(e) => {
            bail!(ScreenError::new(
                ScreenErrorCodes::SettingsParseError,
                format!("error parsing the settings.yml - {}", e),
            ));
        }
    };

    Ok(config)
}
