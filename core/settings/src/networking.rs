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
    pub enabled: bool,
    pub wireguard: WireguardSettings,
    pub peer_settings: PeerSettings,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            wireguard: WireguardSettings {
                tun: "wg0".to_string(),
                port: 51820,
                userspace: "linux".to_string(),
            },
            peer_settings: PeerSettings {
                ipv4_address: "".to_string(),
                ipv6_address: "".to_string(),
                subnet: "".to_string(),
                network_id: "".to_string(),
                dns_name: "".to_string(),
            },
        }
    }
}
