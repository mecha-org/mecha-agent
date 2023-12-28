use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use sentry_anyhow::capture_anyhow;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::{
    collections::HashMap,
    process::{Command, Output},
};
use tracing::{debug, error, info, trace};

use crate::utils::run_command;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
#[derive(Debug, Default, Clone, Copy)]
pub enum NebulaErrorCodes {
    #[default]
    NebulaError,
    NebulaCommandError,
    NebulaCommandOutputError,
    NebulaCommandOutputDeserializeError,
    NebulaCertDecodeError,
    NebulaCertVerificationError,
    NebulaCertsGenError,
    NebulaStartError,
}

impl fmt::Display for NebulaErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NebulaErrorCodes::NebulaError => {
                write!(f, "NebulaErrorCodes: NebulaError")
            }
            NebulaErrorCodes::NebulaCommandError => {
                write!(f, "NebulaErrorCodes: NebulaCommandError")
            }
            NebulaErrorCodes::NebulaCommandOutputDeserializeError => {
                write!(f, "NebulaErrorCodes: NebulaCommandOutputDeserializeError")
            }
            NebulaErrorCodes::NebulaCommandOutputError => {
                write!(f, "NebulaErrorCodes: NebulaCommandOutputError")
            }
            NebulaErrorCodes::NebulaCertDecodeError => {
                write!(f, "NebulaErrorCodes: NebulaCertDecodeError")
            }
            NebulaErrorCodes::NebulaCertVerificationError => {
                write!(f, "NebulaErrorCodes: NebulaCertVerificationError")
            }
            NebulaErrorCodes::NebulaCertsGenError => {
                write!(f, "NebulaErrorCodes: NebulaCertsGenError")
            }
            NebulaErrorCodes::NebulaStartError => {
                write!(f, "NebulaErrorCodes: NebulaStartError")
            }
        }
    }
}
#[derive(Debug)]
pub struct NebulaError {
    pub code: NebulaErrorCodes,
    pub message: String,
}

impl std::fmt::Display for NebulaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NebulaErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl NebulaError {
    pub fn new(code: NebulaErrorCodes, message: String, capture_error: bool) -> Self {
        if capture_error {
            let error = &anyhow::anyhow!(code)
                .context(format!("error: (code: {:?}, message: {})", code, message));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NebulaSettings {
    pub pki: PKISettings,
    pub static_host_map: HashMap<String, Vec<String>>,
    pub lighthouse: LighthouseSettings,
    pub firewall: FirewallSettings,
    pub tun: TunSettings,
    pub punchy: PunchySettings,
    pub listen: ListenSettings,
    pub relay: RelaySettings,
    pub logging: LoggingSettings,
}

impl Default for NebulaSettings {
    fn default() -> Self {
        Self {
            pki: PKISettings::default(),
            static_host_map: HashMap::new(),
            lighthouse: LighthouseSettings::default(),
            firewall: FirewallSettings::default(),
            tun: TunSettings::default(),
            punchy: PunchySettings::default(),
            listen: ListenSettings::default(),
            relay: RelaySettings::default(),
            logging: LoggingSettings::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PKISettings {
    pub ca: String,
    pub cert: String,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct StaticHostMap {
    pub hosts: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LighthouseSettings {
    pub am_lighthouse: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
    pub hosts: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct FirewallSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbound_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbound_action: Option<String>,
    pub conntrack: ConntrackSettings,
    pub outbound: Vec<FirewallRule>,
    pub inbound: Vec<FirewallRule>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ConntrackSettings {
    pub tcp_timeout: String,
    pub udp_timeout: String,
    pub default_timeout: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct FirewallRule {
    pub port: String,
    pub proto: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cidr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct TunSettings {
    pub disabled: bool,
    pub dev: String,
    pub drop_local_broadcast: bool,
    pub drop_multicast: bool,
    pub tx_queue: u32,
    pub mtu: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PunchySettings {
    pub punch: bool,
    pub respond: bool,
    pub delay: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respond_delay: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListenSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RelaySettings {
    pub am_relay: bool,
    pub use_relays: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LoggingSettings {
    pub level: String,
    pub format: String,
}

// Add other necessary structs and implementations as needed

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct NebulaCertDetails {
    pub curve: String,
    pub domains: Vec<String>,
    pub groups: Vec<String>,
    pub ips: Vec<String>,
    pub is_ca: bool,
    pub issuer: String,
    pub name: String,
    pub not_after: DateTime<Utc>,
    pub not_before: DateTime<Utc>,
    pub public_key: String,
    pub subnets: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct NebulaCert {
    pub details: NebulaCertDetails,
    pub fingerprint: String,
    pub signature: String,
}

pub fn decode_cert(cert_path: &str) -> Result<NebulaCert> {
    let fn_name = "decode_cert";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "cert path is {}",
        cert_path
    );
    let output: Output = match Command::new("nebula-cert")
        .arg("print")
        .arg("-path")
        .arg(cert_path)
        .arg("-json")
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to execute command: {}",
                e
            );
            bail!(NebulaError::new(
                NebulaErrorCodes::NebulaCommandError,
                format!("failed to execute command: {}", e),
                true
            ))
        }
    };

    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "command output - {:?}",
        output
    );
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        error!(
            func = fn_name,
            package = PACKAGE_NAME,
            "failed to get output from command: {}",
            error
        );
        bail!(NebulaError::new(
            NebulaErrorCodes::NebulaCommandOutputError,
            format!("failed to get output from command: {}", error),
            true
        ))
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let decoded_cert = match serde_json::from_str::<NebulaCert>(&stdout) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to deserialize: {}",
                e
            );
            bail!(NebulaError::new(
                NebulaErrorCodes::NebulaCommandOutputDeserializeError,
                format!("failed to deserialize: {}", e),
                true
            ))
        }
    };
    info!(func = fn_name, package = PACKAGE_NAME, "cert decoded!");
    Ok(decoded_cert)
}

pub fn verify_cert(ca_path: &str, cert_path: &str) -> Result<bool> {
    let fn_name = "verify_cert";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "ca path is {}, cert path is {}",
        ca_path,
        cert_path
    );
    let output: Output = match Command::new("nebula-cert")
        .arg("verify")
        .arg("-ca")
        .arg(ca_path)
        .arg("-crt")
        .arg(cert_path)
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            info!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to execute command: {}",
                e
            );
            bail!(NebulaError::new(
                NebulaErrorCodes::NebulaCommandError,
                format!("failed to execute command: {}", e),
                true
            ))
        }
    };
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "command output - {:?}",
        output
    );
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        error!(
            func = fn_name,
            package = PACKAGE_NAME,
            "failed to get output from command: {}",
            error
        );
        bail!(NebulaError::new(
            NebulaErrorCodes::NebulaCommandOutputError,
            format!("failed to get output from command: {}", error),
            true
        ))
    }
    info!(func = fn_name, package = PACKAGE_NAME, "cert verified!");
    Ok(true)
}
pub fn is_cert_valid(cert_path: &str) -> Result<bool> {
    let fn_name = "is_cert_valid";
    let decoded_cert = match decode_cert(cert_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to decode cert: {}",
                e
            );
            bail!(NebulaError::new(
                NebulaErrorCodes::NebulaCertDecodeError,
                format!("failed to decode cert: {}", e),
                true
            ))
        }
    };

    let is_valid =
        Utc::now() > decoded_cert.details.not_before && Utc::now() < decoded_cert.details.not_after;
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "cert validity - {}",
        is_valid
    );
    Ok(is_valid)
}

pub fn is_cert_verified(ca_path: &str, cert_path: &str) -> Result<bool> {
    trace!(
        func = "is_cert_verified",
        package = PACKAGE_NAME,
        "ca path is {}, cert path is {}",
        ca_path,
        cert_path
    );
    let cert_verified = match verify_cert(ca_path, cert_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "is_cert_verified",
                package = PACKAGE_NAME,
                "failed to verify cert on path - {}, error - {}",
                cert_path,
                e
            );
            bail!(NebulaError::new(
                NebulaErrorCodes::NebulaCertVerificationError,
                format!("failed to verify cert: {}", e),
                true
            ))
        }
    };
    info!(
        func = "is_cert_verified",
        package = PACKAGE_NAME,
        "cert verification - {}",
        cert_verified
    );
    Ok(cert_verified)
}

pub fn generate_nebula_key_cert(pub_path: &str, key_path: &str) -> Result<bool> {
    let mut command = Command::new("nebula-cert");
    command.arg("keygen");
    command.arg("-out-key");
    command.arg(key_path);
    command.arg("-out-pub");
    command.arg(pub_path);

    match command.status() {
        Ok(status) => match status.success() {
            true => Ok(true),
            false => {
                error!(
                    func = "generate_nebula_key_cert",
                    package = PACKAGE_NAME,
                    "nebula command status returned false"
                );
                bail!(NebulaError::new(
                    NebulaErrorCodes::NebulaCertsGenError,
                    format!("nebula command status returned false",),
                    true
                ))
            }
        },
        Err(e) => {
            error!(
                func = "generate_nebula_key_cert",
                package = PACKAGE_NAME,
                "error while generating nebula certs, error - {}",
                e.to_string()
            );
            bail!(NebulaError::new(
                NebulaErrorCodes::NebulaCertsGenError,
                format!(
                    "error while generating nebula certs, error - {}",
                    e.to_string()
                ),
                true
            ))
        }
    }
}

pub fn start_nebula(binary_path: &str, config_path: &str) -> Result<bool> {
    let fn_name = "start_nebula";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "binary path - {}, config path - {}",
        binary_path,
        config_path
    );
    let cmd = &format!("{}/nebula -config {}/config.yaml", binary_path, config_path);

    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "command to run - {}",
        cmd
    );

    let result = match run_command(cmd) {
        Ok(r) => r,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while running command to start nebula, error - {}",
                e.to_string()
            );
            bail!(NebulaError::new(
                NebulaErrorCodes::NebulaStartError,
                format!(
                    "error while running command to start nebula, error - {}",
                    e.to_string()
                ),
                true
            ))
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "nebula started - {}",
        result
    );
    Ok(result)
}
