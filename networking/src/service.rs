use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::Result;
use tracing::warn;

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub fn configure_wireguard() -> Result<()> {
    // read settings from settings.yml
    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => {
            warn!(
                func = "provision_me",
                package = PACKAGE_NAME,
                "settings.yml not found, using default settings"
            );
            AgentSettings::default()
        }
    };
    // The agent will pull the networking settings
    // Generate a wireguard private key + public key
    let keys = wireguard::generate_new_key_pair();

    // Configure a wireguard interface as per settings.yml and machine settings
    // "node-1" "wg0" "192.168.0.11" "51823"
    Ok(())
}
