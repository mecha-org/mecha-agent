use anyhow::{bail, Result};
use crypto::errors::{CryptoError, CryptoErrorCodes};
use openssl::x509::X509;
use serde::{Deserialize, Serialize};
use settings::AgentSettings;
use std::{fs::File, io::Read, path::PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct Identity {
    settings: AgentSettings,
}

impl Identity {
    pub fn new(settings: AgentSettings) -> Self {
        Self { settings: settings }
    }
    pub fn is_device_provisioned(&self) -> Result<bool> {
        let certificate_paths = match settings::read_settings_yml() {
            Ok(v) => v.provisioning.paths,
            Err(e) => bail!(e),
        };

        let device_cert_path = PathBuf::from(certificate_paths.device.cert);
        let device_private_key = PathBuf::from(certificate_paths.device.private_key);

        if device_cert_path.exists() && device_private_key.exists() {
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn get_machine_id(&self) -> Result<String> {
        let mut public_key_buf = Vec::new();
        let public_key_path = self.settings.provisioning.paths.device.cert.clone();
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
            Err(_err) => {
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
}
