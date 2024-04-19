use std::str::FromStr;

use anyhow::{bail, Result};
use defguard_wireguard_rs::{
    error, host::Peer, key::Key, net::IpAddrMask, InterfaceConfiguration, WireguardApiUserspace,
    WireguardInterfaceApi,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use x25519_dalek::PublicKey;

use rand::rngs::OsRng;
use x25519_dalek::StaticSecret;

#[derive(Serialize, Deserialize, Debug)]
pub struct WgConfig {
    pub ip_address: String,
    pub port: u32,
}

#[derive(Serialize, Deserialize)]
pub struct WgKeys {
    pub secret_key: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerConfiguration {
    pub name: String, // peer name
    pub public_key: String,
    pub endpoint: Vec<String>,
    pub allowed_ips: Vec<String>,
}
//crate name
const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
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

    pub fn setup_wireguard(&mut self, wg_config: &WgConfig, private_key: String) -> Result<bool> {
        let fn_name = "setup wireguard";
        let ifname = self.ifname.clone();

        // Remove existing
        match self.api.remove_interface() {
            Ok(_) => (),
            Err(e) => {
                warn!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to remove interface  - {}",
                    e
                );
            }
        };

        // create interface
        match self.api.create_interface() {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to create interface - {}",
                    e
                );
                bail!(e)
            }
        };

        // interface configuration
        let interface_config = InterfaceConfiguration {
            name: ifname.clone(),
            prvkey: private_key,
            address: wg_config.ip_address.clone(),
            port: wg_config.port,
            peers: vec![],
        };

        match self.api.configure_interface(&interface_config) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to configure interface - {}",
                    e
                );
                bail!(e)
            }
        };
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "wireguard interface {} setup successfully",
            ifname
        );
        Ok(true)
    }

    pub fn add_peer(
        &mut self,
        peer_configuration: PeerConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("peer configuration: {:?}", peer_configuration.clone());
        let PeerConfiguration { public_key, .. } = peer_configuration;
        let peer_key: Key = Key::from_str(&public_key).unwrap();
        let mut peer = Peer::new(peer_key.clone());

        // Your WireGuard server endpoint which peer connects too
        if peer_configuration.endpoint.len() > 0 {
            peer.endpoint = Some(
                peer_configuration
                    .endpoint
                    .first()
                    .unwrap()
                    .parse()
                    .unwrap(),
            );
        }
        peer.persistent_keepalive_interval = Some(25);

        // Peer allowed ips
        for allowed_ip in peer_configuration.allowed_ips {
            let addr = IpAddrMask::from_str(format!("{}/32", &allowed_ip).as_str())?;
            peer.allowed_ips.push(addr);
        }

        match self.api.configure_peer(&peer) {
            Ok(_) => println!("Peer {:?} configured.", peer_configuration.name),
            Err(e) => {
                println!("Error configuring peer: {}", e);
                return Err(e.into());
            }
        }
        Ok(())
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
