use std::{fs::File, io::Read};

use anyhow::{bail, Result};
use base64::b64_encode;
use openssl::{hash::MessageDigest, x509::X509};
use serde::{Deserialize, Serialize};
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

use crate::errors::{CryptoError, CryptoErrorCodes};
pub mod base64;
pub mod errors;
pub mod x509;

#[derive(Serialize, Deserialize, Debug)]
pub struct MachineCert {
    pub expiry: String,
    pub common_name: String,
    pub fingerprint: String,
    pub public_cert: String,
    pub intermediate_cert: String,
    pub root_cert: String,
}
pub fn get_machine_id() -> Result<String> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "get_machine_id", "init");
    let settings = match agent_settings::read_settings_yml() {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    let mut public_key_buf = Vec::new();
    let public_key_path = settings.provisioning.paths.device.cert.clone();
    let mut file = match File::open(public_key_path) {
        Ok(v) => v,
        Err(e) => {
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to open private key file - {}", e),
                true
            ))
        }
    };

    match file.read_to_end(&mut public_key_buf) {
        Ok(v) => v,
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::ReadPrivateKeyError,
            format!("failed to read private key file - {}", e),
            true
        )),
    };
    let cert = match X509::from_pem(public_key_buf.as_slice()) {
        Ok(cert) => cert,
        Err(err) => {
            tracing::error!(
                trace_id,
                task = "issue_token",
                "error deserializing pem -{}",
                err
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::PemDeserializeError,
                format!("error deserializing pem",),
                true
            ))
        }
    };

    let sub_entries = match cert.subject_name().entries().next() {
        Some(sub) => sub,
        None => {
            bail!(CryptoError::new(
                CryptoErrorCodes::ExtractSubjectNameError,
                format!("error in getting subject name entries",),
                true
            ))
        }
    };

    match String::from_utf8(sub_entries.data().as_slice().to_vec()) {
        Ok(str) => {
            tracing::info!("extracted subject name from pem file");
            return Ok(str);
        }
        Err(err) => {
            tracing::error!("error extracting subject name: {:?}", err);
            bail!(CryptoError::new(
                CryptoErrorCodes::ExtractSubjectNameError,
                format!("error extracting subject name",),
                true
            ))
        }
    };
}

pub fn get_machine_cert() -> Result<MachineCert> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "get_machine_cert", "init");
    let settings = match agent_settings::read_settings_yml() {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    let mut public_key_buf = Vec::new();

    // Read public key
    let public_key_path = settings.provisioning.paths.device.cert.clone();
    let mut pub_key_file = match File::open(public_key_path) {
        Ok(v) => v,
        Err(e) => {
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to open private key file - {}", e),
                true
            ))
        }
    };

    // Read intermediate and root certificates
    let intermediate_cert_path = settings.provisioning.paths.intermediate.cert.clone();
    let root_cert_path = settings.provisioning.paths.root.cert.clone();

    let (intermediate_cert, root_cert) =
        read_certificates(intermediate_cert_path, root_cert_path).unwrap();

    match pub_key_file.read_to_end(&mut public_key_buf) {
        Ok(v) => v,
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::ReadPrivateKeyError,
            format!("failed to read private key file - {}", e),
            true
        )),
    };

    let cert = match X509::from_pem(public_key_buf.as_slice()) {
        Ok(cert) => cert,
        Err(err) => {
            tracing::error!(
                trace_id,
                task = "issue_token",
                "error deserializing pem -{}",
                err
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::PemDeserializeError,
                format!("error deserializing pem",),
                true
            ))
        }
    };

    let fingerprint_bytes = match cert.digest(MessageDigest::sha256()) {
        Ok(v) => v.to_vec(),
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::GenerateFingerprintError,
            format!("error while generating fingerprint - {}", e),
            true
        )),
    };
    let fingerprint = String::from_utf8_lossy(&fingerprint_bytes).to_string();
    let response = MachineCert {
        expiry: cert.not_after().to_string(),
        common_name: cert
            .subject_name()
            .entries()
            .next()
            .unwrap()
            .data()
            .as_slice()
            .to_vec()
            .into_iter()
            .map(|x| x as char)
            .collect::<String>(),
        fingerprint: fingerprint,
        public_cert: b64_encode(&public_key_buf),
        intermediate_cert: b64_encode(intermediate_cert),
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
        Err(e) => bail!(e),
    };
    let root_cert = match read_file_to_string(&root_cert_path) {
        Ok(v) => v,
        Err(e) => bail!(e),
    };

    Ok((intermediate_cert, root_cert))
}

fn read_file_to_string(file_path: &str) -> Result<String> {
    let mut file = match File::open(file_path) {
        Ok(v) => v,
        Err(e) => {
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadCertFileError,
                format!("failed to open private key file - {}", e),
                true
            ))
        }
    };

    let mut content = String::new();
    let _ = file.read_to_string(&mut content);

    Ok(content)
}
