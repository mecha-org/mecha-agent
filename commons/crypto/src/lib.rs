use std::io::Read;

use agent_settings::AgentSettings;
use anyhow::{bail, Result};
use base64::b64_encode;
use fs::safe_open_file;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use x509::read_public_key;
use x509_certificate::DigestAlgorithm;

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
use crate::errors::{CryptoError, CryptoErrorCodes};
pub mod base64;
pub mod errors;
pub mod random;
pub mod x509;

#[derive(Serialize, Deserialize, Debug)]
pub struct MachineCert {
    pub expiry: String,
    pub common_name: String,
    pub fingerprint: String,
    pub public_cert: String,
    pub ca_bundle: String,
    pub root_cert: String,
}
pub fn get_machine_id() -> Result<String> {
    let settings = match agent_settings::read_settings_yml() {
        Ok(v) => v,
        Err(e) => {
            warn!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error reading settings.yml - {}",
                e
            );
            AgentSettings::default()
        }
    };
    let public_key_path = settings.provisioning.paths.machine.cert.clone();
    let public_key_cert = match read_public_key(&public_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "failed to read public key - {}",
                e
            );
            bail!(e)
        }
    };
    match public_key_cert.subject_common_name() {
        Some(v) => Ok(v.to_string()),
        None => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "failed to get common name from certificate"
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ExtractSubjectNameError,
                "failed to get common name from certificate".to_string(),
                true
            ))
        }
    }
}

pub fn get_machine_cert() -> Result<MachineCert> {
    let settings = match agent_settings::read_settings_yml() {
        Ok(v) => v,
        Err(e) => {
            warn!(
                func = "get_machine_cert",
                package = PACKAGE_NAME,
                "error reading settings.yml - {}",
                e
            );
            AgentSettings::default()
        }
    };

    // Read public key
    let public_key_path = settings.provisioning.paths.machine.cert.clone();

    // Read intermediate and root certificates
    let ca_bundle_path = settings.provisioning.paths.ca_bundle.cert.clone();
    let root_cert_path = settings.provisioning.paths.root.cert.clone();

    let (ca_bundle, root_cert) = read_certificates(ca_bundle_path, root_cert_path).unwrap();
    let public_key_cert = match read_public_key(&public_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "get_machine_cert",
                package = PACKAGE_NAME,
                "failed to read public key - {}",
                e
            );
            bail!(e)
        }
    };
    let fingerprint = match public_key_cert.fingerprint(DigestAlgorithm::Sha256) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "get_machine_cert",
                package = PACKAGE_NAME,
                "failed to generate fingerprint - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::GenerateFingerprintError,
                format!("failed to generate fingerprint - {}", e),
                true
            ))
        }
    };
    let response = MachineCert {
        expiry: public_key_cert.validity_not_after().to_string(),
        common_name: public_key_cert.subject_common_name().unwrap().to_string(),
        fingerprint: b64_encode(fingerprint),
        public_cert: public_key_cert.encode_pem(),
        ca_bundle: b64_encode(ca_bundle),
        root_cert: b64_encode(root_cert),
    };
    Ok(response)
}
fn read_certificates(
    intermediate_cert_path: String,
    root_cert_path: String,
) -> Result<(String, String)> {
    let intermediate_cert = match read_file_to_string(&intermediate_cert_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "read_certificates",
                package = PACKAGE_NAME,
                "failed to read intermediate certificate file on path - {}, error - {}",
                &intermediate_cert_path,
                e
            );
            bail!(e)
        }
    };
    let root_cert = match read_file_to_string(&root_cert_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "read_certificates",
                package = PACKAGE_NAME,
                "failed to read root certificate file on path - {}, error - {}",
                &root_cert_path,
                e
            );
            bail!(e)
        }
    };
    info!(
        func = "read_certificates",
        package = PACKAGE_NAME,
        "read intermediate and root certificates"
    );
    Ok((intermediate_cert, root_cert))
}

fn read_file_to_string(file_path: &str) -> Result<String> {
    let mut file = match safe_open_file(file_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "read_file_to_string",
                package = PACKAGE_NAME,
                "failed to open file on path - {}, error - {}",
                file_path,
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadCertFileError,
                format!("failed to open file on path - {}, error - {}", file_path, e),
                true
            ))
        }
    };

    let mut content = String::new();
    let _ = file.read_to_string(&mut content);

    Ok(content)
}
