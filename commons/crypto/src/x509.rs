use crate::errors::{CryptoError, CryptoErrorCodes};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use fs::{construct_dir_path, safe_open_file, safe_write_to_path};
use rand::rngs::OsRng;
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_RSA_SHA256};
use rsa::{
    pkcs1v15::SigningKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{fmt, io::Read, path::Path};
use tracing::{error, info, trace};
use x509_certificate::CapturedX509Certificate;
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

pub fn generate_rsa_private_key(file_path: &str) -> Result<bool> {
    let fn_name = "generate_ec_private_key";
    tracing::trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "file_path - {}",
        file_path,
    );

    // Check if the directory of file_path exists, and create it if it doesn't.
    let parent_directory = match Path::new(&file_path).parent() {
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

    // Command: openssl genrsa -out key.pem 2048
    let mut rng = OsRng;
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).unwrap();
    let private_key_der = private_key.to_pkcs8_der().unwrap();
    let key_pair = rcgen::KeyPair::try_from(private_key_der.as_bytes()).unwrap();
    // key_pair.serialize_pem().as_bytes()
    match safe_write_to_path(file_path, key_pair.serialize_pem().as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            bail!(CryptoError::new(
                CryptoErrorCodes::WritePrivateKeyError,
                format!("failed to write private key file - {}", e),
                true
            ))
        }
    }
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

    // read private key
    let mut private_key_str = String::new();
    let mut file = match safe_open_file(&private_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "generate_csr",
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

    match file.read_to_string(&mut private_key_str) {
        Ok(v) => v,
        Err(e) => bail!(CryptoError::new(
            CryptoErrorCodes::ReadPrivateKeyError,
            format!("failed to read private key file - {}", e),
            true
        )),
    };
    let key_pair = match KeyPair::from_pem(&private_key_str) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "generate_csr",
                package = PACKAGE_NAME,
                "failed to read private key file - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to read private key file - {}", e),
                true
            ))
        }
    };
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, common_name);
    let mut params_rsa: CertificateParams = Default::default();
    params_rsa.alg = &PKCS_RSA_SHA256;
    params_rsa.distinguished_name = distinguished_name;
    params_rsa.key_pair = Some(key_pair);
    let cert = Certificate::from_params(params_rsa).unwrap();
    let csr = match cert.serialize_request_pem() {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "generate_csr",
                package = PACKAGE_NAME,
                "failed to serialize csr - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::GenerateCSRError,
                format!("failed to serialize csr - {}", e),
                true
            ))
        }
    };
    // write csr to path
    match safe_write_to_path(&csr_file_path, csr.as_bytes()) {
        Ok(_) => return Ok(true),
        Err(e) => {
            error!(
                func = "generate_csr",
                package = PACKAGE_NAME,
                "failed to write csr file - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::WritePrivateKeyError,
                format!("failed to write csr file - {}", e),
                true
            ))
        }
    }
}

pub fn sign_with_private_key(private_key_path: &str, data: &[u8]) -> Result<Vec<u8>> {
    let file_path = match construct_dir_path(private_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "sign_with_private_key",
                package = PACKAGE_NAME,
                "failed to construct path - {}, error - {}",
                private_key_path,
                e
            );
            bail!(e)
        }
    };
    let path_str = match file_path.into_os_string().into_string() {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "sign_with_private_key",
                package = PACKAGE_NAME,
                "failed to convert path to string - {}, error - {:?}",
                private_key_path,
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::FilePathError,
                format!("failed to convert path to string - {:?}", e),
                true
            ))
        }
    };

    let mut private_key = match safe_open_file(private_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "sign_with_private_key",
                package = PACKAGE_NAME,
                "failed to open private key file - {}, error - {}",
                private_key_path,
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to open private key file - {}", e),
                true
            ))
        }
    };
    let mut private_key_str = String::new();
    match private_key.read_to_string(&mut private_key_str) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "sign_with_private_key",
                package = PACKAGE_NAME,
                "failed to read private key file - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to read private key file - {}", e),
                true
            ))
        }
    };
    // Load the private key from a file
    let private_key = match RsaPrivateKey::from_pkcs8_pem(&private_key_str) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "sign_with_private_key",
                package = PACKAGE_NAME,
                "failed to load private key from file - {}, error - {}",
                private_key_path,
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to load private key from file - {}", e),
                true
            ))
        }
    };
    let signer = SigningKey::<Sha256>::new(private_key);
    let signature = signer.sign(data);
    Ok(signature.to_vec())
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

pub fn read_public_key(file_path: &str) -> Result<CapturedX509Certificate> {
    let fn_name = "read_public_key";
    let mut public_key_buf = Vec::new();
    let mut file = match safe_open_file(file_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to open private key file on path - {}, error - {}",
                file_path,
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to open private key file - {}", e),
                true
            ))
        }
    };

    match file.read_to_end(&mut public_key_buf) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to read private key file - {}",
                e
            );
            bail!(CryptoError::new(
                CryptoErrorCodes::ReadPrivateKeyError,
                format!("failed to read private key file - {}", e),
                true
            ))
        }
    };
    match CapturedX509Certificate::from_pem(public_key_buf) {
        Ok(cert) => return Ok(cert),
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
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
}
