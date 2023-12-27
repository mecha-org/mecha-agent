use crate::errors::ProvisioningError;
use crate::errors::ProvisioningErrorCodes;
use ::fs::construct_dir_path;
use ::fs::remove_files;
use ::fs::safe_write_to_path;
use agent_settings::provisioning::CertificatePaths;
use agent_settings::read_settings_yml;
use agent_settings::AgentSettings;
use anyhow::{bail, Result};
use crypto::random::generate_random_alphanumeric;
use crypto::x509::generate_csr;
use crypto::x509::generate_ec_private_key;
use crypto::x509::PrivateKeyAlgorithm;
use crypto::x509::PrivateKeySize;
use events::Event;
use reqwest::Client as RequestClient;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::str;
use tokio::sync::broadcast::Sender;
use tracing::error;
use tracing::{debug, info, trace, warn};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");

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
    pub machine_id: String,
    pub cert_signing_url: String,
    pub cert_key_pair_algorithm: PrivateKeyAlgorithm,
    pub cert_key_pair_size: PrivateKeySize,
    pub cert_valid_upto: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignCSRRequest {
    pub csr: String,
    pub machine_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignedCertificates {
    pub cert: String,
    pub intermediate_cert: String,
    pub root_cert: String,
}

pub fn generate_code() -> Result<String> {
    trace!(func = "generate_code", package = PACKAGE_NAME, "init",);

    let code = generate_random_alphanumeric(6);
    debug!(
        func = "generate_code",
        package = PACKAGE_NAME,
        "code generated - {:?}",
        code
    );
    info!(
        func = "generate_code",
        package = PACKAGE_NAME,
        result = "success",
        "code generated successfully",
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
    tracing::trace!(
        func = "provision_by_code",
        package = PACKAGE_NAME,
        "init code - {:?}",
        code
    );

    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => {
            warn!(
                func = "provision_me",
                package = PACKAGE_NAME,
                "settings.yml not found, using default settings"
            );
            AgentSettings::default()
        }
    };
    // 1. Lookup the manifest, if lookup fails with not found then return error
    let manifest = match lookup_manifest(&settings, &code).await {
        Ok(manifest) => {
            debug!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "provisioning manifest - {:?}",
                manifest
            );
            manifest
        }
        Err(e) => {
            error!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "error looking up manifest for code- {}",
                &code
            );
            bail!(e)
        } // throw error from manifest lookup
    };

    // 2. Generate the private key based on the key algorithm
    match manifest.cert_key_pair_algorithm {
        PrivateKeyAlgorithm::ECDSA => match generate_ec_private_key(
            &settings.provisioning.paths.machine.private_key,
            manifest.cert_key_pair_size,
        ) {
            Ok(_) => debug!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "private key generated successfully"
            ),
            Err(e) => {
                error!(
                    func = "provision_by_code",
                    package = PACKAGE_NAME,
                    "error generating private key on path - {}",
                    &settings.provisioning.paths.machine.private_key
                );
                bail!(e)
            }
        },
    };

    // 3. Generate the CSR, using above private key
    match generate_csr(
        &settings.provisioning.paths.machine.csr,
        &settings.provisioning.paths.machine.private_key,
        &manifest.machine_id,
    ) {
        Ok(_) => debug!(
            func = "provision_by_code",
            package = PACKAGE_NAME,
            "csr generated successfully"
        ),
        Err(e) => {
            error!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "error generating csr - {}",
                e
            );
            bail!(e)
        }
    };

    // 4. Sign the CSR using the cert signing url
    let signed_certificates = match sign_csr(
        &settings.provisioning.server_url,
        &settings.provisioning.paths.machine.csr,
        &manifest.machine_id,
        &manifest.cert_signing_url,
    )
    .await
    {
        Ok(signed_cer) => {
            debug!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "csr signed successfully"
            );
            signed_cer
        }
        Err(e) => {
            error!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "error signing csr for machine_id - {}",
                &manifest.machine_id
            );
            bail!(e)
        }
    };

    // 5. Store the signed certificates in destination path
    match write_certificates_to_path(
        &settings.provisioning.paths,
        signed_certificates.root_cert.as_bytes(),
        signed_certificates.intermediate_cert.as_bytes(),
        signed_certificates.cert.as_bytes(),
    ) {
        Ok(result) => {
            debug!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "certificates written successfully, result - {}",
                result
            );
            result
        }
        Err(e) => {
            error!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "error writing certificates to path - {:?}",
                &settings.provisioning.paths
            );
            bail!(e)
        }
    };

    match event_tx.send(Event::Provisioning(events::ProvisioningEvent::Provisioned)) {
        Ok(_) => debug!(
            func = "provision_by_code",
            package = PACKAGE_NAME,
            "provisioning event sent successfully"
        ),
        Err(e) => {
            error!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "error sending provisioning event - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::SendEventError,
                format!(
                    "error sending provisioning event, code: {}, error - {}",
                    1001, e
                ),
                true
            ));
        }
    }
    info!(
        func = "provision_by_code",
        package = PACKAGE_NAME,
        result = "success",
        "machine provisioned successfully"
    );
    Ok(true)
}

pub fn de_provision(event_tx: Sender<Event>) -> Result<bool> {
    trace!(func = "de_provision", package = PACKAGE_NAME, "init",);
    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => {
            warn!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "settings.yml not found, using default settings"
            );
            AgentSettings::default()
        }
    };
    //1. Delete certs
    match remove_files(vec![
        &settings.provisioning.paths.machine.cert,
        &settings.provisioning.paths.machine.private_key,
        &settings.provisioning.paths.machine.csr,
        &settings.provisioning.paths.intermediate.cert,
        &settings.provisioning.paths.root.cert,
    ]) {
        Ok(_) => (),
        Err(e) => {
            error!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "error deleting certs - {}",
                e
            );
            bail!(e)
        }
    }

    //2. Event to stop all services
    match event_tx.send(Event::Provisioning(
        events::ProvisioningEvent::Deprovisioned,
    )) {
        Ok(_) => debug!(
            func = "de_provision",
            package = PACKAGE_NAME,
            "de provisioning event sent successfully"
        ),
        Err(e) => {
            error!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "error sending de provisioning event - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::SendEventError,
                format!(
                    "error sending de provisioning event, code:{}, error - {}",
                    1001, e
                ),
                true
            ));
        }
    }

    //3. Delete database
    match fs::remove_dir_all(&settings.settings.storage.path) {
        Ok(_) => {
            debug!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "db deleted successfully from path - {:?}",
                &settings.settings.storage.path
            )
        }
        Err(e) => {
            error!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "error deleting db, from path {}, error - {}",
                &settings.settings.storage.path,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::DatabaseDeleteError,
                format!("error deleting db, code: {}, error - {}", 1001, e),
                true
            ));
        }
    }

    info!(
        func = "de_provision",
        package = PACKAGE_NAME,
        result = "success",
        "de provisioned successful",
    );
    Ok(true)
}
async fn lookup_manifest(settings: &AgentSettings, code: &str) -> Result<ProvisioningManifest> {
    trace!(
        func = "lookup_manifest",
        package = PACKAGE_NAME,
        "init, code - {:?}",
        code
    );
    let url = format!(
        "{}/v1/provisioning/manifest/find?code={}",
        settings.provisioning.server_url, code
    );
    debug!(
        func = "lookup_manifest",
        package = PACKAGE_NAME,
        "looking for manifest at url - {:?}",
        url
    );
    let req_client = RequestClient::new();
    let response = req_client.get(&url).send().await;
    let lookup_result = match response {
        Ok(v) => v,
        Err(e) => match e.status() {
            Some(StatusCode::INTERNAL_SERVER_ERROR) => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "manifest find endpoint url returned internal server error for url - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::ManifestLookupServerError,
                    format!("manifest find endpoint url returned server error - {}", e),
                    true
                ))
            }
            Some(StatusCode::BAD_REQUEST) => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "manifest find endpoint url returned bad request - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::ManifestLookupBadRequestError,
                    format!("manifest find endpoint url returned bad request - {}", e),
                    true
                ))
            }
            Some(StatusCode::NOT_FOUND) => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "manifest find endpoint url not found - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::ManifestLookupNotFoundError,
                    format!("manifest find endpoint url not found - {}", e),
                    true
                ))
            }
            Some(_) => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "manifest find endpoint url returned unknown error - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::ManifestLookupUnknownError,
                    format!("manifest find endpoint url returned unknown error - {}", e),
                    true
                ))
            }
            None => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "manifest find endpoint url returned unknown error - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::ManifestLookupUnknownError,
                    format!(
                        "manifest find endpoint url returned unmatched error - {}",
                        e
                    ),
                    true
                ))
            }
        },
    };

    // parse the manifest lookup result
    let manifest_response = match lookup_result
        .json::<ProvisioningServerResponseGeneric<ProvisioningManifest>>()
        .await
    {
        Ok(parse_manifest) => {
            debug!(
                func = "lookup_manifest",
                package = PACKAGE_NAME,
                "manifest lookup response - {:?}",
                parse_manifest
            );
            parse_manifest
        }
        Err(e) => {
            error!(
                func = "lookup_manifest",
                package = PACKAGE_NAME,
                "error parsing manifest lookup response - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ManifestParseResponseError,
                format!("error parsing lookup manifest response - {}", e),
                true
            ))
        }
    };

    info!(
        func = "lookup_manifest",
        package = PACKAGE_NAME,
        result = "success",
        "manifest lookup successful"
    );
    Ok(manifest_response.payload)
}

fn write_certificates_to_path(
    certificate_paths: &CertificatePaths,
    root_cert: &[u8],
    intermediate_cert: &[u8],
    cert: &[u8],
) -> Result<bool> {
    trace!(
        func = "write_certificates_to_path",
        package = PACKAGE_NAME,
        "cert path - {}",
        certificate_paths.machine.cert,
    );

    // save the machine certificate
    match safe_write_to_path(&certificate_paths.machine.cert, cert) {
        Ok(_) => debug!(
            func = "write_file",
            package = PACKAGE_NAME,
            "machine certificate saved in path - {}",
            &certificate_paths.machine.cert
        ),
        Err(e) => {
            error!(
                func = "write_file",
                package = PACKAGE_NAME,
                "error saving machine certificate in path - {} - {}",
                &certificate_paths.machine.cert,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CertificateWriteError,
                format!(
                    "error saving machine certificate in path - {} - {}",
                    &certificate_paths.machine.cert, e
                ),
                true
            ))
        }
    }

    // save the intermediate certificate
    match safe_write_to_path(&certificate_paths.intermediate.cert, intermediate_cert) {
        Ok(_) => debug!(
            func = "write_file",
            package = PACKAGE_NAME,
            "intermediate certificate saved in path - {}",
            &certificate_paths.intermediate.cert
        ),
        Err(e) => {
            error!(
                func = "write_file",
                package = PACKAGE_NAME,
                "error saving intermediate certificate in path - {} - {}",
                &certificate_paths.intermediate.cert,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CertificateWriteError,
                format!(
                    "error saving intermediate certificate in path - {} - {}",
                    &certificate_paths.intermediate.cert, e
                ),
                true
            ))
        }
    }

    // save the root certificate
    match safe_write_to_path(&certificate_paths.root.cert, root_cert) {
        Ok(_) => debug!(
            func = "write_file",
            package = PACKAGE_NAME,
            "root certificate saved in path - {}",
            &certificate_paths.root.cert
        ),
        Err(e) => {
            error!(
                func = "write_file",
                package = PACKAGE_NAME,
                "error saving root certificate in path - {} - {}",
                &certificate_paths.root.cert,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CertificateWriteError,
                format!(
                    "error saving root certificate in path - {} - {}",
                    &certificate_paths.root.cert, e
                ),
                true
            ))
        }
    }

    info!(
        func = "write_certificates_to_path",
        package = PACKAGE_NAME,
        result = "success",
        "certificates written successfully"
    );
    Ok(true)
}

async fn sign_csr(
    request_url: &str,
    csr_path: &str,
    machine_id: &str,
    cert_signing_url: &str,
) -> Result<SignedCertificates> {
    trace!(
        func = "sign_csr",
        package = PACKAGE_NAME,
        "init, request_url {}, csr_sign_url {}",
        request_url,
        cert_signing_url
    );

    let constructed_path = match construct_dir_path(csr_path) {
        Ok(path) => {
            debug!(
                func = "sign_csr",
                package = PACKAGE_NAME,
                "csr path constructed {:?}",
                path.display()
            );
            path
        }
        Err(e) => {
            error!(
                func = "sign_csr",
                package = PACKAGE_NAME,
                "error constructing csr path - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignReadFileError,
                format!("error opening csr in path - {} - {}", csr_path, e),
                true
            ))
        }
    };
    let csr_pem = match fs::read_to_string(constructed_path) {
        Ok(csr_str) => {
            debug!(
                func = "sign_csr",
                package = PACKAGE_NAME,
                "read csr as string - {:?}",
                csr_str
            );
            csr_str
        }
        Err(e) => {
            error!(
                func = "sign_csr",
                package = PACKAGE_NAME,
                "error reading csr as string - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignReadFileError,
                format!("error reading csr in path - {} - {}", csr_path, e),
                true
            ))
        }
    };

    // Construct payload for signing the csr
    let sign_csr_request_body = SignCSRRequest {
        csr: csr_pem,
        machine_id: machine_id.to_string(),
    };

    // Format url for signing the csr
    let url = format!("{}{}", request_url, cert_signing_url);

    debug!(
        func = "sign_csr",
        package = PACKAGE_NAME,
        "sign csr url formatted successfully - {:?}",
        url
    );
    // Request to sign the csr
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
            Some(StatusCode::INTERNAL_SERVER_ERROR) => {
                error!(
                    func = "sign_csr",
                    package = PACKAGE_NAME,
                    "csr sign url returned internal server error - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::CSRSignServerError,
                    format!("csr sign url returned server error - {}", e),
                    true
                ))
            }
            Some(StatusCode::BAD_REQUEST) => {
                error!(
                    func = "sign_csr",
                    package = PACKAGE_NAME,
                    "csr sign url returned bad request - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::CSRSignBadRequestError,
                    format!("csr sign url returned bad request - {}", e),
                    true
                ))
            }
            Some(StatusCode::NOT_FOUND) => {
                error!(
                    func = "sign_csr",
                    package = PACKAGE_NAME,
                    "csr sign url not found - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::CSRSignNotFoundError,
                    format!("csr sign url not found - {}", e),
                    true
                ))
            }
            Some(_) => {
                error!(
                    func = "sign_csr",
                    package = PACKAGE_NAME,
                    "csr sign url returned unknown error - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::CSRSignUnknownError,
                    format!("csr sign url returned unknown error - {}", e),
                    true
                ))
            }
            None => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignUnknownError,
                format!("csr sign url returned unmatched error - {}", e),
                true
            )),
        },
    };
    let result: ProvisioningServerResponseGeneric<SignedCertificates> =
        match serde_json::from_str(&csr_string) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = "sign_csr",
                    package = PACKAGE_NAME,
                    "error parsing csr sign response - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::CSRSignResponseParseError,
                    format!("error parsing csr sign response - {}", e),
                    true
                ));
            }
        };
    info!(
        func = "sign_csr",
        package = PACKAGE_NAME,
        result = "success",
        "csr signed successfully"
    );
    Ok(result.payload)
}
