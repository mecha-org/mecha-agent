use crate::settings::Keys;
use crate::ProvisioningSettings;
use anyhow::bail;
use anyhow::Result;
use openssl::ec::EcGroup;
use openssl::ec::EcKey;
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::x509::X509NameBuilder;
use openssl::x509::X509Req;
use openssl::x509::X509ReqBuilder;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use sentry_anyhow::capture_anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::io::Write;
use std::process::Command;
use std::thread;
use std::{
    fmt, str,
    time::{Duration, Instant},
};
use tonic::Code;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

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

#[derive(Serialize, Deserialize, Debug)]
pub struct CsrBuilderParams {
    pub country: String,
    pub state: String,
    pub organization: String,
    pub common_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub success: bool,
    pub status: String,
    pub status_code: i16,
    pub message: Option<String>,
    pub error_code: Option<String>,
    pub sub_errors: Option<String>,
    pub payload: Value,
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self {
            success: true,
            status: String::from("OK"),
            status_code: 200,
            message: None,
            error_code: None,
            sub_errors: None,
            payload: json!({}),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum ProvisioningErrorCodes {
    #[default]
    ManifestationNotFound,
    CertificateGenerationFailed,
    CsrSignedFailed,
    CertificateWriteFailed,
}
impl std::fmt::Display for ProvisioningErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProvisioningErrorCodes::ManifestationNotFound => {
                write!(f, "Manifest not found error")
            }
            ProvisioningErrorCodes::CertificateGenerationFailed => {
                write!(f, "Certification generation failed")
            }
            ProvisioningErrorCodes::CsrSignedFailed => {
                write!(f, "CSR signing failed")
            }
            ProvisioningErrorCodes::CertificateWriteFailed => {
                write!(f, "Certificate write failed")
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

#[derive(Serialize, Deserialize, Debug)]
pub struct SignCsrRequestBody {
    pub device_id: String,
    pub csr: String,
}

impl std::fmt::Display for ProvisioningServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(code: {:?}, message: {})", self.code, self.message)
    }
}

impl ProvisioningServiceError {
    pub fn new(code: ProvisioningErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "server",
            "Error: (code: {:?}, message: {})", code, message
        );
        if capture_error {
            let error = &anyhow::anyhow!(code).context(format!(
                "Error: (code: {:?}, messages: {} trace:{:?})",
                code, message, trace_id
            ));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProvisioningService {
    settings: ProvisioningSettings,
}

impl ProvisioningService {
    pub fn new(settings: ProvisioningSettings) -> Self {
        Self { settings: settings }
    }

    pub fn new_request(&self) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::info!(trace_id, task = "provisioning_new_request", "init",);

        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        Ok(rand_string.to_uppercase())
    }

    pub async fn request_manifest(&self, code: &str) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::info!(trace_id, task = "request_manifest", "init",);
        let client = Client::new();
        let url = format!(
            "{}/provisioning/manifest/find?code={}",
            &self.settings.server_base_url, code
        );
        tracing::debug!(
            trace_id,
            task = "request_manifest",
            "find manifest url formatted {:?}",
            url
        );
        let timeout_duration = Duration::from_secs(60);
        let start_time = Instant::now();
        loop {
            let response = client.get(&url).send().await;
            let result = match response {
                Ok(v) => v,
                Err(e) => {
                    bail!(ProvisioningServiceError::new(
                        ProvisioningErrorCodes::ManifestationNotFound,
                        format!("Error {}", e),
                        true
                    ));
                }
            };
            let resp = result.json::<SuccessResponse>().await;

            let final_res = match resp {
                Ok(x) => x,
                Err(e) => {
                    let error = ProvisioningServiceError::new(
                        ProvisioningErrorCodes::CertificateGenerationFailed,
                        format!("Error: {:?}", e),
                        true,
                    );
                    return Err(anyhow::anyhow!(error));
                }
            };

            if final_res.success == true {
                tracing::info!(
                    trace_id,
                    task = "request_manifest",
                    "manifest found successfully",
                );
                // Successful response, break the loop
                if self.settings.openssl.engine == "trustm" {
                    let key = generate_key_pair_trustm("0xe0e1");
                    match key {
                        Ok(v) => println!("key: {}", v),
                        Err(e) => {
                            bail!(ProvisioningServiceError::new(
                                ProvisioningErrorCodes::CertificateGenerationFailed,
                                format!("Error {}", e),
                                true
                            ));
                        }
                    }
                } else {
                    //reading the input from the user
                    print!("\nEnter the Details for generating the CSR\n");
                    let country = read_input("Enter the country name (ISO alpha-2): ");
                    let state = read_input("Enter the state name: ");
                    let organization = read_input("prompt: Enter the organization name: ");

                    tracing::info!(
                        trace_id,
                        task = "request_manifest",
                        "CSR details entered successfully"
                    );
                    let csr_builder_params = CsrBuilderParams {
                        country: country.clone(),
                        state: state.clone(),
                        organization: organization.clone(),
                        common_name: final_res.payload["deviceId"].as_str().unwrap().to_string(),
                    };

                    //generate csr and store private key at the path provided in settings
                    let csr_params =
                        generate_csr(csr_builder_params, &self.settings.keys.device.privatekey)
                            .unwrap();

                    tracing::info!(
                        trace_id,
                        task = "request_manifest",
                        "CSR generated successfully"
                    );
                    let csr_pem = csr_params.to_pem()?;
                    let csr_str = String::from_utf8(csr_pem).unwrap();
                    let cert_sign_url = final_res.payload["deviceCertSigningURL"].to_string();

                    //construct payload for signing the csr
                    let sign_csr_request_body = SignCsrRequestBody {
                        device_id: final_res.payload["deviceId"].to_string(),
                        csr: csr_str.clone(),
                    };
                    tracing::info!(
                        trace_id,
                        task = "request_manifest",
                        "sign csr request body formatted successfully"
                    );
                    //format url for signing the csr
                    let url = format!(
                        "{}{}",
                        &self.settings.server_base_url,
                        cert_sign_url.trim_matches('"')
                    );

                    //request to sign the csr
                    let result = sign_csr(&url, sign_csr_request_body).await;
                    let success_response = match result {
                        Ok(v) => {
                            tracing::info!(
                                trace_id,
                                task = "request_manifest",
                                "csr signed successfully"
                            );
                            v
                        }
                        Err(e) => {
                            bail!(ProvisioningServiceError::new(
                                ProvisioningErrorCodes::CertificateGenerationFailed,
                                format!("Error {}", e),
                                true
                            ));
                        }
                    };

                    //validate response and if success then store certificate at the path provided in settings
                    let crt = success_response.payload["crt"]
                        .as_str()
                        .expect("Failed to get certificate");
                    let ca = success_response.payload["ca"]
                        .as_str()
                        .expect("Failed to get CA");
                    let cert_chain = success_response.payload["certChain"]
                        .as_array()
                        .expect("Failed to get intermediate certificate");

                    let result = write_file(
                        crt.as_bytes(),
                        ca.as_bytes(),
                        cert_chain.get(0).unwrap().as_str().unwrap().as_bytes(),
                        &self.settings.keys,
                    );

                    match result {
                        Ok(_v) => return Ok(true),
                        Err(e) => {
                            bail!(ProvisioningServiceError::new(
                                ProvisioningErrorCodes::CertificateWriteFailed,
                                format!("Error {}", e),
                                true
                            ));
                        }
                    }
                }
            } else {
                tracing::warn!(trace_id, task = "request_manifest", "manifest not found",)
            }

            // Check if the timeout duration has been reached
            if Instant::now() - start_time >= timeout_duration {
                tracing::error!(
                    trace_id,
                    task = "request_manifest",
                    "manifest not found after 60 seconds",
                );
                return Err(bail!(ProvisioningServiceError::new(
                    ProvisioningErrorCodes::ManifestationNotFound,
                    format!("Error {}", "Timed out after 60 seconds"),
                    true
                )));
                break;
            }

            // Wait for 10 seconds before making the next request
            thread::sleep(Duration::from_secs(10));
        }
        Ok(false)
    }
}

fn write_file(
    device_cert: &[u8],
    ca_cert: &[u8],
    intermediate_cert: &[u8],
    device_keys_path: &Keys,
) -> io::Result<()> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "write_file", "writing certificate to file");
    create_directory_if_not_exists(&device_keys_path.device.cert)?;
    write_cert_to_path(device_cert, &device_keys_path.device.cert)?;
    tracing::info!(
        trace_id,
        task = "write_file",
        "device certificate written successfully"
    );
    create_directory_if_not_exists(&device_keys_path.root.cert)?;
    write_cert_to_path(ca_cert, &device_keys_path.root.cert)?;

    tracing::info!(
        trace_id,
        task = "write_file",
        "ca certificate written successfully"
    );
    create_directory_if_not_exists(&device_keys_path.intermediate.cert)?;
    write_cert_to_path(intermediate_cert, &device_keys_path.intermediate.cert)?;
    tracing::info!(
        trace_id,
        task = "write_file",
        "intermediate certificate written successfully"
    );
    Ok(())
}

fn create_directory_if_not_exists(path: &str) -> io::Result<()> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "create_directory_if_not_exists", "init");
    if let Some(parent_dir) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent_dir)?;
    }
    tracing::info!(
        trace_id,
        task = "create_directory_if_not_exists",
        "directory created successfully"
    );
    Ok(())
}

fn write_cert_to_path(cert: &[u8], path: &str) -> io::Result<()> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "write_cert_to_path", "init");
    fs::write(path, cert)?;
    Ok(())
}

async fn sign_csr(sign_url: &str, request_body: SignCsrRequestBody) -> Result<SuccessResponse> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "sign_csr", "init");
    let client = reqwest::Client::new();
    let csr_req = client
        .post(sign_url)
        .json(&request_body)
        .basic_auth("".to_string(), Some("".to_string()))
        .header("CONTENT_TYPE", "application/json")
        .header("ACCEPT", "application/json")
        .send()
        .await?;

    let csr_string = match csr_req.text().await {
        Ok(csr) => csr,
        Err(e) => {
            bail!(ProvisioningServiceError::new(
                ProvisioningErrorCodes::CsrSignedFailed,
                format!("Error {}", e),
                true
            ));
        }
    };
    tracing::info!(trace_id, task = "sign_csr", "csr signed successfully",);
    let result: Result<SuccessResponse, serde_json::Error> = serde_json::from_str(&csr_string);
    let result = match result {
        Ok(v) => v,
        Err(e) => {
            bail!(ProvisioningServiceError::new(
                ProvisioningErrorCodes::CertificateGenerationFailed,
                format!("Error {}", e),
                true
            ));
        }
    };
    Ok(result)
}

fn generate_csr(
    csr_builder_params: CsrBuilderParams,
    device_key_path: &str,
) -> Result<X509Req, ErrorStack> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "generate_csr", "init");
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let key = EcKey::generate(&group).unwrap();

    let private_key = key.private_key_to_pem().unwrap();
    tracing::info!(
        trace_id,
        task = "generate_csr",
        "private key generated successfully"
    );
    // Create the directories leading to the new key_path
    if let Some(parent_dir) = std::path::Path::new(&device_key_path).parent() {
        fs::create_dir_all(parent_dir).unwrap();
    }
    // Write the private key to the specified path
    fs::write(device_key_path, private_key).unwrap();
    tracing::info!(
        trace_id,
        task = "generate_csr",
        "private key written successfully"
    );
    let mut req_builder = X509ReqBuilder::new()?;
    req_builder
        .set_pubkey(&PKey::from_ec_key(key.clone()).unwrap())
        .unwrap();
    let mut x509_name = X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", &csr_builder_params.country)?;
    x509_name.append_entry_by_text("ST", &csr_builder_params.state)?;
    x509_name.append_entry_by_text("O", &csr_builder_params.organization)?;
    x509_name.append_entry_by_text("CN", &csr_builder_params.common_name)?;
    let x509_name = x509_name.build();
    req_builder.set_subject_name(&x509_name)?;
    req_builder.sign(
        &PKey::from_ec_key(key.clone()).unwrap(),
        MessageDigest::sha256(),
    )?;
    let req = req_builder.build();
    Ok(req)
}

// Generate ECC key pair using trustm_ecc_keygen binary
fn generate_key_pair_trustm(oid: &str) -> Result<String> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "generate_key_pair_trustm", "init",);
    let output_result = Command::new("openssl")
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
        .output();

    let output = match output_result {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                trace_id,
                task = "generate_key_pair_trustm",
                "failed to generate key pair with trustm",
            );
            bail!(ProvisioningServiceError::new(
                ProvisioningErrorCodes::CertificateGenerationFailed,
                format!("Error {}", e),
                true
            ));
        }
    };

    if output.status.success() {
        tracing::info!(
            trace_id,
            task = "generate_key_pair_trustm",
            "key pair generated successfully!",
        );
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(bail!(ProvisioningServiceError::new(
            ProvisioningErrorCodes::CertificateGenerationFailed,
            format!("Error {}", stderr),
            true
        )));
    }
}

fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    // remove trailing newline
    input.trim().to_string()
}
