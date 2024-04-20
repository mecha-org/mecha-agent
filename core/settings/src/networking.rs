use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WireguardSettings {
    pub tun: String,
    pub port: u32,
    pub userspace: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct PeerSettings {
    pub ipv4_address: String,
    pub ipv6_address: String,
    pub subnet: String,
    pub network_id: String,
    pub dns_name: String,
}
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct NetworkingSettings {
    pub disco_addr: String,
    pub enabled: bool,
    pub wireguard: WireguardSettings,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            disco_addr: "0.0.0.0".to_string(),
            enabled: false,
            wireguard: WireguardSettings {
                tun: "wg0".to_string(),
                port: 51820,
                userspace: "linux".to_string(),
            },
        }
    }
}
