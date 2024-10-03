use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct NetworkingSettings {
    pub enabled: bool,
    pub discovery: DiscoverySettings,
    pub wireguard: WireguardSettings,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            discovery: DiscoverySettings::default(),
            wireguard: WireguardSettings::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DiscoverySettings {
    pub addr: String,
    pub port: u16,
}

impl Default for DiscoverySettings {
    fn default() -> Self {
        Self {
            addr: String::from("0.0.0.0"),
            port: 7774,
        }
    }
}
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WireguardSettings {
    pub tun: String,
    pub port: u16,
    pub userspace: String,
    pub dns: DNSSettings,
}

impl Default for WireguardSettings {
    fn default() -> Self {
        Self {
            tun: String::from("wg0"),
            port: 7775,
            userspace: String::from("linux"),
            dns: DNSSettings::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DNSSettings {
    pub enabled: bool,
    pub port: u16,
}

impl Default for DNSSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 53,
        }
    }
}
