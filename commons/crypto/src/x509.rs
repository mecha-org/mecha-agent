use crate::errors::{CryptoError, CryptoErrorCodes};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use fs::{construct_dir_path, safe_open_file};
use openssl::{pkey::PKey, sign::Signer};
use serde::{Deserialize, Serialize};
use std::{fmt, io::Read, path::Path, process::Command};
use tracing::{debug, error, info, trace};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/**
 * Open SSL Commands Reference
 *
 * [Default]
 * ECDSA:
 * Generate Key: openssl ecparam -name secp521r1 -genkey -noout -out key.pem
 * Generate CSR: openssl req -new -sha256 -key key.pem -out req.pem
 * Sign: openssl dgst -sha256  -sign private.pem /path/to/data
 * Verify: openssl dgst -ecdsa-with-SHA1 -verify public.pem -signature /path/to/signature /path/to/data
 *
 * RSA:
 * Generate Key: openssl genrsa -out key.pem 2048
 * Generate CSR: openssl req -new -sha256 -key key.pem -out req.pem
 *
 * [TrustM]
 * TBD
 *
 */

// Certificate Attributes
#[derive(Serialize, Deserialize, Debug)]
pub struct CertificateAttributes {
    pub country: Option<String>,
    pub state: Option<String>,
    pub organization: Option<String>,
    pub common_name: String,
}

// Key algorithm enum
#[derive(Serialize, Deserialize, Debug)]
pub enum PrivateKeyAlgorithm {
    ECDSA,
}

// Key size enum
#[derive(Serialize, Deserialize, Debug)]
pub enum PrivateKeySize {
    #[serde(rename = "EC_P256")]
    EcP256,
    #[serde(rename = "EC_P384")]
    EcP384,
    #[serde(rename = "EC_P521")]
    EcP521,
}

impl fmt::Display for PrivateKeySize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrivateKeySize::EcP256 => write!(f, "EC_P256"),
            PrivateKeySize::EcP384 => write!(f, "EC_P384"),
            PrivateKeySize::EcP521 => write!(f, "EC_P521"),
        }
    }
}

// Decoded cert
#[derive(Debug)]
pub struct DecodedCert {
    pub not_after: DateTime<Utc>,
    pub not_before: DateTime<Utc>,
}

pub fn generate_ec_private_key(file_path: &str, key_size: PrivateKeySize) -> Result<bool> {
    let fn_name = "generate_ec_private_key";
    tracing::trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "file_path - {}, key_size - {:?}",
        file_path,
        key_size
    );
    let file_path_buf = match construct_dir_path(file_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to construct file path - {}",
                e
            );
            bail!(e)
        }
    };
    let elliptic_curve = match key_size {
        PrivateKeySize::EcP256 => String::from("secp256r1"),
        PrivateKeySize::EcP384 => String::from("secp384r1"),
        PrivateKeySize::EcP521 => String::from("secp521r1"),
        // k => bail!(CryptoError::new(
        //     CryptoErrorCodes::CryptoGeneratePrivateKeyError,
        //     format!("key size not supported for elliptical curve key - {}", k),
        //     true
        // ))
    };

    // Check if the directory of file_path exists, and create it if it doesn't.
    let parent_directory = match Path::new(&file_path_buf).parent() {
        Some(v) => v,
        None => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "invalid file path - {}",
                file_path
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::FilePathError,
                format!("invalid file path - {}", file_path),
                false
            ))
        }
    };
    println!("parent_directory: {:?}", parent_directory);
    if !parent_directory.exists() {
        let _res = safe_create_dir(&file_path);
    }

    // Command: openssl ecparam -name secp521r1 -genkey -noout -out key.pem
    let output_result = Command::new("openssl")
        .arg("ecparam")
        .arg("-name")
        .arg(elliptic_curve)
        .arg("-genkey")
        .arg("-noout")
        .arg("-out")
        .arg(file_path_buf.to_str().unwrap())
        .output();

    let output = match output_result {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "openssl ec private key generate command failed - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::GeneratePrivateKeyError,
                format!("openssl private key generate command failed - {}", e),
                true
            ))
        }
    };

    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "openssl ec private key generate command output - {:?}",
        output
    );
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!(
            func = fn_name,
            package = PACKAGE_NAME,
            "openssl error in generating private key, stderr - {}",
            stderr
        );
        bail!(CryptoError::new(
            CryptoErrorCodes::GeneratePrivateKeyError,
            format!(
                "openssl error in generating private key, stderr - {}",
                stderr
            ),
            true
        ))
    }

    let _stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // TODO: Update permissions of keypath to 400
    Ok(true)
}

pub fn generate_csr(
    csr_file_path: &str,
    private_key_path: &str,
    common_name: &str,
) -> Result<bool> {
    trace!(
        func = "generate_csr",
        package = PACKAGE_NAME,
        "csr_file_path - {}, private_key_path - {}, common_name - {}",
        csr_file_path,
        private_key_path,
        common_name
    );

    let subject = format!("/C=/ST=/L=/O=/OU=/CN={}", common_name);
    let private_key_path_buf = match construct_dir_path(&private_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "generate_csr",
                package = PACKAGE_NAME,
                "failed to construct private key path - {}, error - {}",
                &private_key_path,
                e
            );
            bail!(e)
        }
    };

    let csr_file_path_buf = match construct_dir_path(csr_file_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "generate_csr",
                package = PACKAGE_NAME,
                "failed to construct csr file path - {}, error - {}",
                csr_file_path,
                e
            );
            bail!(e)
        }
    };

    // Command: openssl req -new -sha256 -key key.pem -subj "/C=/ST=/L=/O=/OU=/CN=" -out req.pem
    let output_result = Command::new("openssl")
        .arg("req")
        .arg("-new")
        .arg("-sha256")
        .arg("-key")
        .arg(private_key_path_buf.to_str().unwrap())
        .arg("-subj")
        .arg(subject)
        .arg("-out")
        .arg(csr_file_path_buf.to_str().unwrap())
        .output();

    debug!(
        func = "generate_csr",
        package = PACKAGE_NAME,
        "openssl csr generate command output - {:?}",
        output_result
    );
    let output = match output_result {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "generate_csr",
                package = PACKAGE_NAME,
                "openssl csr generate command failed - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::GenerateCSRError,
                format!("openssl csr generate command failed - {}", e),
                true
            ))
        }
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        info!(
            func = "generate_csr",
            package = PACKAGE_NAME,
            "openssl csr generate command output - {}",
            stdout
        );
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!(
            func = "generate_csr",
            package = PACKAGE_NAME,
            "openssl error in generating csr, stderr - {}",
            stderr
        );
        bail!(CryptoError::new(
            CryptoErrorCodes::GenerateCSRError,
            format!("openssl error in generating csr, stderr - {}", stderr),
            true
        ))
    }
}

pub fn sign_with_private_key(private_key_path: &str, data: &[u8]) -> Result<Vec<u8>> {
    // Load the private key from a file
    let mut private_key_buf = Vec::new();
    let mut file = match safe_open_file(private_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "sign_with_private_key",
                package = PACKAGE_NAME,
                "failed to open private key file - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to open private key file - {}", e),
                true
            ))
        }
    };

    match file.read_to_end(&mut private_key_buf) {
        Ok(v) => v,
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::ReadPrivateKeyError,
            format!("failed to read private key file - {}", e),
            true
        )),
    };

    let private_key = match PKey::private_key_from_pem(&private_key_buf) {
        Ok(v) => v,
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::OpenPrivateKeyError,
            format!("failed to open private key - {}", e),
            true
        )),
    };

    // Sign the message using the private key
    let mut signer = match Signer::new(openssl::hash::MessageDigest::sha256(), &private_key) {
        Ok(v) => v,
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::LoadSignerError,
            format!("failed to load openssl signer - {}", e),
            true
        )),
    };
    match signer.update(data) {
        Ok(v) => v,
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::UpdateSignerError,
            format!("failed to update openssl signer - {}", e),
            true
        )),
    };
    match signer.sign_to_vec() {
        Ok(v) => {
            tracing::info!("signature completed: {:?}", v);
            return Ok(v);
        }
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::UpdateSignerError,
            format!("failed to sign data - {}", e),
            true
        )),
    };
}

fn safe_create_dir(path: &str) -> Result<bool> {
    trace!(
        func = "safe_create_dir",
        package = PACKAGE_NAME,
        "path - {}",
        path
    );
    let path_buf = match construct_dir_path(path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "safe_create_dir",
                package = PACKAGE_NAME,
                "failed to construct path - {}, error - {}",
                path,
                e
            );
            bail!(e)
        }
    };

    // Extract the file name (the last component of the path)
    if let Some(file_name) = path_buf.file_name() {
        if let Some(_file_name_str) = file_name.to_str() {
            let mut dir_to_create = path_buf.clone();
            //Last component will be pooled out
            dir_to_create.pop();
            match mkdirp::mkdirp(&dir_to_create) {
                Ok(p) => p,
                Err(err) => bail!(err),
            };
        }
    }
    info!(
        func = "safe_create_dir",
        package = PACKAGE_NAME,
        "directory created - {}",
        path
    );
    Ok(true)
}
