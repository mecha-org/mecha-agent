use crate::ProvisioningSettings;
use anyhow::bail;
use anyhow::Result;
use openssl::ec::EcGroup;
use openssl::ec::EcKey;
use openssl::nid::Nid;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::File;
use std::io;
use std::io::Write;
use std::process::Command;
use std::thread;
use std::{
    fmt, str,
    time::{Duration, Instant},
};
use tonic::Code;

#[derive(Debug)]
pub struct ProvisioningErrorResponseCode {
    pub code: Code,
    pub message: String,
}

// Key type enum
#[derive(Serialize, Deserialize, Debug)]
enum KeyType {
    Auth,
    Enc,
    Hfwu,
    DevM,
    Sign,
    Agmt,
}

// Key size enum
#[derive(Serialize, Deserialize, Debug)]
enum KeySize {
    ECC256,
    ECC384,
    ECC521,
    Brainpool256,
    Brainpool384,
    Brainpool512,
}
#[derive(Debug, Default, Clone, Copy)]
pub enum ProvisioningErrorCodes {
    #[default]
    ManifestationNotFound,
    CertificateGenerationFailed,
}
impl std::fmt::Display for ProvisioningErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProvisioningErrorCodes::ManifestationNotFound => {
                write!(f, "Manifestation not found error")
            }
            ProvisioningErrorCodes::CertificateGenerationFailed => {
                write!(f, "Certification generation failed")
            }
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct EventDetails {
    pub event: String,
    #[serde(rename = "clientid")]
    pub client_id: String,
}
#[derive(Debug)]
pub struct ProvisioningServiceError {
    pub code: ProvisioningErrorCodes,
    pub message: String,
}

impl std::fmt::Display for ProvisioningServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(code: {:?}, message: {})", self.code, self.message)
    }
}

impl ProvisioningServiceError {
    pub fn new(code: ProvisioningErrorCodes, message: String, capture_error: bool) -> Self {
        Self {
            code,
            message: message,
        }
    }
}

pub struct ProvisioningService {
    settings: ProvisioningSettings,
}

impl ProvisioningService {
    pub fn new(settings: ProvisioningSettings) -> Self {
        Self { settings: settings }
    }

    pub fn new_request(&self) -> Result<String> {
        println!("init provisioning");
        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        Ok(rand_string)
    }

    pub async fn manifest_request(&self) -> Result<String, ProvisioningErrorResponseCode> {
        println!("init manifest");
        let client = Client::new();
        let url = format!("http://localhost:3000/provisioning/manifest/find?code=1234");
        let timeout_duration = Duration::from_secs(5);
        let start_time = Instant::now();

        loop {
            let response = client.get(&url).send().await;

            let result = match response {
                Ok(v) => v,
                Err(e) => {
                    return Err(ProvisioningErrorResponseCode {
                        code: Code::InvalidArgument,
                        message: format!("Error Manifest not Found : {}", e),
                    });
                }
            };
            if result.status().is_success() && (result.status().is_success()) {
                // Successful response, break the loop
                println!("Received successful response: {:?}", result);
                // generate_private_key();
                let key = generate_key_pair_trustm("0xe0e1");
                match key {
                    Ok(v) => println!("key: {}", v),
                    Err(e) => {
                        return Err(ProvisioningErrorResponseCode {
                            code: Code::InvalidArgument,
                            message: format!("Error Certificate generation Failed: {}", e),
                        });
                    }
                }
                break;
            } else {
                println!("Received response: {:?}", result);
            }

            // Check if the timeout duration has been reached
            if Instant::now() - start_time >= timeout_duration {
                eprintln!("Timed out after 60 seconds");
                return Err(ProvisioningErrorResponseCode {
                    code: Code::InvalidArgument,
                    message: format!("Error {}", "Manifest not Found. Timed out after 60 seconds"),
                });
                break;
            }

            // Wait for 10 seconds before making the next request
            thread::sleep(Duration::from_secs(2));
        }

        Ok("manifest".to_string())
    }
}

// Generate ECC key pair using trustm_ecc_keygen binary
fn generate_key_pair_trustm(oid: &str) -> Result<String, io::Error> {
    let output = Command::new("openssl")
        .arg("pkey")
        .arg("-engine")
        .arg("trustm_engine")
        .arg("-pubout")
        .arg("-inform")
        .arg("engine")
        .arg("-in")
        .arg(format!("{}:*:NEW", oid))
        .arg("-out")
        .arg("public_key.pem")
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(io::Error::new(io::ErrorKind::Other, stderr))
    }
}

fn generate_private_key() -> Result<Value> {
    // Create an Elliptic Curve group for prime256v1 (secp256r1)
    let group_result = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1);

    let group = match group_result {
        Ok(group) => group,
        Err(e) => {
            bail!(ProvisioningServiceError::new(
                ProvisioningErrorCodes::ManifestationNotFound,
                format!("Error {}", e),
                true
            ));
        }
    };
    // Generate an EC key pair
    let keypair = EcKey::generate(&group)?;

    // Convert the private key to PEM format
    let private_key_pem = keypair.private_key_to_pem()?;

    // Define the output file path
    let output_file_path = "private_key.pem";

    // Write the private key to the output file
    let mut output_file = File::create(output_file_path)?;
    output_file.write_all(&private_key_pem)?;

    println!("Private key generated and saved to {}", output_file_path);
    Ok(json!("manifestation"))
}
