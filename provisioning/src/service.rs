use crate::errors::ProvisioningError;
use crate::errors::ProvisioningErrorCodes;
use ::fs::safe_write_to_path;
use agent_settings::provisioning::CertificatePaths;
use agent_settings::read_settings_yml;
use agent_settings::AgentSettings;
use anyhow::{bail, Result};
use crypto::x509::generate_csr;
use crypto::x509::generate_ec_private_key;
use crypto::x509::PrivateKeyAlgorithm;
use crypto::x509::PrivateKeySize;
use events::Event;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client as RequestClient;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str;
use tokio::sync::broadcast::Sender;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningServerResponseGeneric<T> {
    pub success: bool,
    pub status: String,
    pub status_code: i16,
    pub message: Option<String>,
    pub error_code: Option<String>,
    pub sub_errors: Option<String>,
    pub payload: T,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningManifest {
    pub device_id: String,
    pub cert_signing_url: String,
    pub cert_key_pair_algorithm: PrivateKeyAlgorithm,
    pub cert_key_pair_size: PrivateKeySize,
    pub cert_valid_upto: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignCSRRequest {
    pub csr: String,
    pub device_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignedCertificates {
    pub cert: String,
    pub intermediate_cert: String,
    pub root_cert: String,
}

pub fn generate_code() -> Result<String> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "generate_code", "init",);

    let code: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    tracing::debug!(
        trace_id,
        task = "generate_code",
        "code generated {:?}",
        code.to_uppercase()
    );
    Ok(code.to_uppercase())
}

/**! This function performs the machine provisioning, the flow works as follows -
    1. Look for the manifest in the url configured
    2. If the manifest is found then generate private key based on manifest params
    3. Generate the CSR with the private key
    4. Sign the certificate using the cert signing url in the manifest
    5. Store the certificate, intermediate and root in the target path
*/
pub async fn provision_by_code(code: String, event_tx: Sender<Event>) -> Result<bool> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "provision machine", "init",);

    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };
    // 1. Lookup the manifest, if lookup fails with not found then return error
    let manifest = match lookup_manifest(&settings, &code).await {
        Ok(m) => {
            tracing::info!(
                trace_id,
                task = "provision_me",
                result = "success",
                "provisioning manifest found",
            );
            m
        }
        Err(e) => bail!(e), // throw error from manifest lookup
    };

    // 2. Generate the private key based on the key algorithm
    let private_key_status = match manifest.cert_key_pair_algorithm {
        PrivateKeyAlgorithm::ECDSA => generate_ec_private_key(
            &settings.provisioning.paths.device.private_key,
            manifest.cert_key_pair_size,
        ),
    };

    match private_key_status {
        Ok(_) => tracing::info!(
            trace_id,
            task = "provision_me",
            "private key generated in path - {}",
            &settings.provisioning.paths.device.private_key
        ),
        Err(err) => bail!(err),
    }

    // 3. Generate the CSR, using above private key
    let csr_status = generate_csr(
        &settings.provisioning.paths.device.csr,
        &settings.provisioning.paths.device.private_key,
        &manifest.device_id,
    );

    match csr_status {
        Ok(_) => tracing::info!(
            trace_id,
            task = "provision_me",
            "private key generated in path - {}",
            &settings.provisioning.paths.device.private_key
        ),
        Err(err) => bail!(err),
    }

    // 4. Sign the CSR using the cert signing url
    let signed_certificates = match sign_csr(
        &settings.provisioning.server_url,
        &settings.provisioning.paths.device.csr,
        &manifest.device_id,
        &manifest.cert_signing_url,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => bail!(e),
    };

    // 5. Store the signed certificates in destination path
    match write_certificates_to_path(
        &settings.provisioning.paths,
        signed_certificates.root_cert.as_bytes(),
        signed_certificates.intermediate_cert.as_bytes(),
        signed_certificates.cert.as_bytes(),
    ) {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    let _ = event_tx.send(Event::Provisioning(events::ProvisioningEvent::Provisioned));
    Ok(true)
}

async fn lookup_manifest(settings: &AgentSettings, code: &str) -> Result<ProvisioningManifest> {
    println!("settings: {:?}", settings);
    let trace_id = find_current_trace_id();
    let url = format!(
        "{}/v1/provisioning/manifest/find?code={}",
        settings.provisioning.server_url, code
    );
    tracing::debug!(
        trace_id,
        task = "lookup_manifest",
        "looking for manifest at url - {:?}",
        url
    );
    let req_client = RequestClient::new();
    let response = req_client.get(&url).send().await;
    let lookup_result = match response {
        Ok(v) => v,
        Err(e) => match e.status() {
            Some(StatusCode::INTERNAL_SERVER_ERROR) => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ManifestLookupServerError,
                format!("manifest find endpoint url returned server error - {}", e),
                true
            )),
            Some(StatusCode::BAD_REQUEST) => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ManifestLookupBadRequestError,
                format!("manifest find endpoint url returned bad request - {}", e),
                false // Not reporting bad request errors
            )),
            Some(StatusCode::NOT_FOUND) => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ManifestLookupNotFoundError,
                format!("manifest find endpoint url not found - {}", e),
                false // Not reporting not found errors
            )),
            Some(_) => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ManifestLookupUnknownError,
                format!("manifest find endpoint url returned unknown error - {}", e),
                true
            )),
            None => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ManifestLookupUnknownError,
                format!("manifest find endpoint url returned unknown error - {}", e),
                true
            )),
        },
    };

    // parse the manifest lookup result
    let manifest_response = match lookup_result
        .json::<ProvisioningServerResponseGeneric<ProvisioningManifest>>()
        .await
    {
        Ok(m) => m,
        Err(e) => bail!(ProvisioningError::new(
            ProvisioningErrorCodes::ManifestParseResponseError,
            format!("error parsing lookup manifest response - {}", e),
            true
        )),
    };

    Ok(manifest_response.payload)
}

fn write_certificates_to_path(
    certificate_paths: &CertificatePaths,
    root_cert: &[u8],
    intermediate_cert: &[u8],
    cert: &[u8],
) -> Result<bool> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "write_certificates_to_path", "init");

    // save the device certificate
    match safe_write_to_path(&certificate_paths.device.cert, cert) {
        Ok(_) => tracing::info!(
            trace_id,
            task = "write_file",
            "device certificate saved in path - {}",
            &certificate_paths.device.cert
        ),
        Err(e) => bail!(ProvisioningError::new(
            ProvisioningErrorCodes::CertificateWriteError,
            format!(
                "error saving device certificate in path - {} - {}",
                &certificate_paths.device.cert, e
            ),
            true
        )),
    }

    // save the intermediate certificate
    match safe_write_to_path(&certificate_paths.intermediate.cert, intermediate_cert) {
        Ok(_) => tracing::info!(
            trace_id,
            task = "write_file",
            "intermediate certificate saved in path - {}",
            &certificate_paths.intermediate.cert
        ),
        Err(e) => bail!(ProvisioningError::new(
            ProvisioningErrorCodes::CertificateWriteError,
            format!(
                "error saving intermediate certificate in path - {} - {}",
                &certificate_paths.intermediate.cert, e
            ),
            true
        )),
    }

    // save the root certificate
    match safe_write_to_path(&certificate_paths.root.cert, root_cert) {
        Ok(_) => tracing::info!(
            trace_id,
            task = "write_file",
            "root certificate saved in path - {}",
            &certificate_paths.root.cert
        ),
        Err(e) => bail!(ProvisioningError::new(
            ProvisioningErrorCodes::CertificateWriteError,
            format!(
                "error saving root certificate in path - {} - {}",
                &certificate_paths.root.cert, e
            ),
            true
        )),
    }

    Ok(true)
}

async fn sign_csr(
    request_url: &str,
    csr_path: &str,
    machine_id: &str,
    cert_signing_url: &str,
) -> Result<SignedCertificates> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "sign_csr", "init");

    let csr_pem = match fs::read_to_string(PathBuf::from(csr_path)) {
        Ok(pem) => pem,
        Err(e) => bail!(ProvisioningError::new(
            ProvisioningErrorCodes::CSRSignReadFileError,
            format!("error reading csr in path - {} - {}", csr_path, e),
            true
        )),
    };

    //construct payload for signing the csr
    let sign_csr_request_body = SignCSRRequest {
        csr: csr_pem,
        device_id: machine_id.to_string(),
    };

    tracing::info!(
        trace_id,
        task = "sign_csr",
        "sign csr request body formatted successfully"
    );

    // format url for signing the csr
    let url = format!("{}{}", request_url, cert_signing_url);

    //request to sign the csr
    let client = reqwest::Client::new();
    let csr_req = client
        .post(url)
        .json(&sign_csr_request_body)
        .header("CONTENT_TYPE", "application/json")
        .send()
        .await?;

    let csr_string = match csr_req.text().await {
        Ok(csr) => csr,
        Err(e) => match e.status() {
            Some(StatusCode::INTERNAL_SERVER_ERROR) => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignServerError,
                format!("csr sign url returned server error - {}", e),
                true
            )),
            Some(StatusCode::BAD_REQUEST) => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignBadRequestError,
                format!("csr sign url returned bad request - {}", e),
                true // Not reporting bad request errors
            )),
            Some(StatusCode::NOT_FOUND) => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignNotFoundError,
                format!("csr sign url not found - {}", e),
                false // Not reporting not found errors
            )),
            Some(_) => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignUnknownError,
                format!("csr sign url returned unknown error - {}", e),
                true
            )),
            None => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignUnknownError,
                format!("csr sign url returned unknown error - {}", e),
                true
            )),
        },
    };
    let result: ProvisioningServerResponseGeneric<SignedCertificates> =
        match serde_json::from_str(&csr_string) {
            Ok(v) => v,
            Err(e) => {
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::CSRSignResponseParseError,
                    format!("error parsing csr sign response - {}", e),
                    true
                ));
            }
        };
    Ok(result.payload)
}
