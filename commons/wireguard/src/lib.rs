use anyhow::Result;
use defguard_wireguard_rs::{InterfaceConfiguration, WireguardApiUserspace, WireguardInterfaceApi};
use serde::{Deserialize, Serialize};
use x25519_dalek::PublicKey;

use rand::rngs::OsRng;
use x25519_dalek::StaticSecret;

#[derive(Serialize, Deserialize)]
pub struct WgConfig {
    pub secret_key: String,
    pub public_key: String,
    pub ip_address: String,
    pub port: u32,
    pub interface_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct WgKeys {
    pub secret_key: String,
    pub public_key: String,
}

#[derive(Clone)]
pub struct Wireguard {
    pub api: WireguardApiUserspace,
    pub ifname: String,
}
impl Wireguard {
    pub fn new(ifname: String) -> Self {
        let api = WireguardApiUserspace::new(ifname.clone()).unwrap();
        Self { api, ifname }
    }

    pub fn setup_wireguard(&mut self, wg_config: &WgConfig) -> Result<bool> {
        let ifname = self.ifname.clone();

        // Remove existing
        match self.api.remove_interface() {
            Ok(_) => println!("Interface {ifname} removed."),
            Err(e) => {
                println!("Error removing interface: {}", e);
            }
        };

        // create interface
        match self.api.create_interface() {
            Ok(_) => (),
            Err(e) => {
                println!("Error creating interface: {}", e);
                return Err(e.into());
            }
        };

        // interface configuration
        let interface_config = InterfaceConfiguration {
            name: ifname.clone(),
            prvkey: wg_config.secret_key.clone(),
            address: format!("{}/24", wg_config.ip_address.clone()),
            port: wg_config.port,
            peers: vec![],
        };

        match self.api.configure_interface(&interface_config) {
            Ok(_) => (),
            Err(e) => {
                println!("Error configuring interface: {}", e);
                return Err(e.into());
            }
        };
        println!("Interface {ifname} configured.");
        // pause();

        Ok(true)
    }
}

pub fn generate_new_key_pair() -> Result<WgKeys> {
    let wg_secret_key = StaticSecret::random_from_rng(&mut OsRng);
    let wg_public_key = PublicKey::from(&wg_secret_key);

    let secret_key_bytes = wg_secret_key.to_bytes();
    let public_key_bytes = wg_public_key.to_bytes();
    let wg_key_pair = WgKeys {
        secret_key: base64::encode(secret_key_bytes),
        public_key: base64::encode(public_key_bytes),
    };
    Ok(wg_key_pair)
}
