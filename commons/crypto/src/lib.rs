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
pub struct MachineCertDetails {
    pub serial_number: String,
    pub expiry: String,
    pub common_name: String,
    pub fingerprint: String,
    pub public_cert: String,
    pub ca_bundle: String,
    pub root_cert: String,
}
pub fn get_machine_id(public_key_path: &str) -> Result<String> {
    let public_key_cert = match x509::read_public_key(&public_key_path) {
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
            ))
        }
    }
}
pub fn get_serial_number(public_key_path: &str) -> Result<String> {
    let public_key_cert = match read_public_key(&public_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "get_serial_number",
                package = PACKAGE_NAME,
                "failed to read public key - {}",
                e
            );
            bail!(e)
        }
    };

    // Convert ASN.1 Integer to a hexadecimal string
    let serial_number_hex = public_key_cert.serial_number_asn1();
    let hex_string = serial_number_hex
        .as_slice()
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(":");
    Ok(hex_string)
}
pub fn get_machine_cert(
    public_key_path: &str,
    ca_bundle_path: &str,
    root_cert_path: &str,
) -> Result<MachineCertDetails> {
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
            ))
        }
    };
    let serial_number = match get_serial_number(public_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "get_machine_cert",
                package = PACKAGE_NAME,
                "failed to get serial number - {}",
                e
            );
            bail!(e)
        }
    };
    let response = MachineCertDetails {
        serial_number: serial_number,
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
    intermediate_cert_path: &str,
    root_cert_path: &str,
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
            ))
        }
    };

    let mut content = String::new();
    let _ = file.read_to_string(&mut content);

    Ok(content)
}
