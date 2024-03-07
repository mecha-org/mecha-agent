use crate::errors::ProvisioningError;
use crate::errors::ProvisioningErrorCodes;
use ::fs::construct_dir_path;
use ::fs::remove_files;
use ::fs::safe_write_to_path;
use agent_settings::provisioning::CertificatePaths;
use agent_settings::read_settings_yml;
use agent_settings::AgentSettings;
use anyhow::{bail, Result};
use channel::recv_with_timeout;
use crypto::random::generate_random_alphanumeric;
use crypto::x509::generate_csr;
use crypto::x509::generate_rsa_private_key;
use events::Event;
use futures::StreamExt;
use identity::handler::IdentityMessage;

use messaging::handler::MessagingMessage;
use messaging::Bytes;
use messaging::Subscriber as NatsSubscriber;
use reqwest::Client as RequestClient;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::str;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinSet;
use tracing::error;
use tracing::{debug, info, trace, warn};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    status: i32,
    message: String,
}
#[derive(Debug)]
pub struct PingResponse {
    pub code: String,
    pub message: String,
}
#[derive(Deserialize, Debug)]
struct DeprovisionRequest {
    pub machine_id: String,
}

#[derive(Deserialize, Debug)]
struct ReIssueCertificateRequest {
    pub machine_id: String,
}
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
pub struct ProvisioningManifest {
    pub machine_id: String,
    pub cert_sign_url: String,
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
    pub root_cert: String,
    pub ca_bundle: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ProvisioningSubscriber {
    pub de_provisioning_request: Option<NatsSubscriber>,
    pub re_issue_certificate: Option<NatsSubscriber>,
}
#[derive(Debug)]
pub enum ProvisioningSubject {
    DeProvision(String),
    ReIssueCertificate(String),
}
pub async fn subscribe_to_nats(
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
) -> Result<ProvisioningSubscriber> {
    let fn_name = "subscribe_to_nats";
    // Get machine id
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(id) => id,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting machine id - {}",
                e
            );
            bail!(e)
        }
    };
    let list_of_subjects = vec![
        ProvisioningSubject::DeProvision(format!(
            "machine.{}.deprovision",
            sha256::digest(machine_id.clone())
        )),
        ProvisioningSubject::ReIssueCertificate(format!(
            "machine.{}.provisioning.cert.re_issue",
            sha256::digest(machine_id.clone())
        )),
    ];
    let mut provisioning_subscribers = ProvisioningSubscriber::default();
    // Iterate over everything.
    for subject in list_of_subjects {
        let (tx, rx) = oneshot::channel();
        let subject_string = match &subject {
            ProvisioningSubject::DeProvision(s) => s.to_string(),
            ProvisioningSubject::ReIssueCertificate(s) => s.to_string(),
        };
        match messaging_tx
            .send(MessagingMessage::Subscriber {
                reply_to: tx,
                subject: subject_string,
            })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error sending get que subscriber for issue token- {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::ChannelSendMessageError,
                    format!("error sending subscriber message - {}", e),
                    true
                ));
            }
        }
        match recv_with_timeout(rx).await {
            Ok(subscriber) => match &subject {
                ProvisioningSubject::DeProvision(_) => {
                    provisioning_subscribers.de_provisioning_request = Some(subscriber)
                }
                ProvisioningSubject::ReIssueCertificate(_) => {
                    provisioning_subscribers.re_issue_certificate = Some(subscriber)
                }
            },
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while get networking subscriber - {:?}, error - {}",
                    &subject,
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::ChannelReceiveMessageError,
                    format!(
                        "error get networking subscriber - {:?}, error - {}",
                        &subject, e
                    ),
                    true
                ));
            }
        };
    }

    Ok(provisioning_subscribers)
}

pub async fn ping() -> Result<PingResponse> {
    trace!(func = "ping", package = PACKAGE_NAME, "init",);
    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => {
            warn!(
                func = "ping",
                package = PACKAGE_NAME,
                "settings.yml not found, using default settings"
            );
            AgentSettings::default()
        }
    };
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let result = client
        .get(format!("{}/v1/ping", settings.provisioning.server_url).as_str())
        .header("CONTENT_TYPE", "application/json")
        .header("ACCEPT", "application/json")
        .send()
        .await;
    println!("result {:?}", result);
    match result {
        Ok(res) => {
            //step-ca returns error payload with 200 status code, error is inside payload
            if res.status() == StatusCode::CREATED || res.status().is_success() {
                return Ok(PingResponse {
                    code: String::from("success"),
                    message: String::from(""),
                });
            } else {
                let error_status_code = res.status();
                match error_status_code {
                    StatusCode::UNAUTHORIZED => {
                        error!(
                            func = "ping",
                            package = PACKAGE_NAME,
                            "ping call returned unauthorized error num - {}",
                            1002,
                        );
                        bail!(ProvisioningError::new(
                            ProvisioningErrorCodes::UnauthorizedError,
                            format!("ping call returned unauthorized error num - {}", 1002,),
                            true
                        ))
                    }
                    StatusCode::NOT_FOUND => {
                        error!(
                            func = "ping",
                            package = PACKAGE_NAME,
                            "ping call returned not found error num - {}",
                            1003,
                        );
                        bail!(ProvisioningError::new(
                            ProvisioningErrorCodes::NotFoundError,
                            format!("ping call returned not found error num - {}", 1003,),
                            true
                        ))
                    }
                    StatusCode::BAD_REQUEST => {
                        error!(
                            func = "ping",
                            package = PACKAGE_NAME,
                            "ping call returned bad request num - {}",
                            1004,
                        );
                        bail!(ProvisioningError::new(
                            ProvisioningErrorCodes::BadRequestError,
                            format!("ping call returned bad request num - {}", 1004,),
                            true
                        ))
                    }
                    StatusCode::INTERNAL_SERVER_ERROR => {
                        error!(
                            func = "ping",
                            package = PACKAGE_NAME,
                            "ping call returned internal server error num - {}",
                            1005,
                        );
                        bail!(ProvisioningError::new(
                            ProvisioningErrorCodes::InternalServerError,
                            format!("ping call returned internal server error num - {}", 1005,),
                            true
                        ))
                    }
                    _ => {
                        error!(
                            func = "ping",
                            package = PACKAGE_NAME,
                            "ping call returned unknown error num - {}",
                            1006,
                        );
                        bail!(ProvisioningError::new(
                            ProvisioningErrorCodes::UnknownError,
                            format!("ping call returned unknown error num - {}", 1006,),
                            true
                        ))
                    }
                }
            }
        }
        Err(e) => {
            error!(
                func = "ping",
                package = PACKAGE_NAME,
                "ping call returned error num - {}, error - {}",
                1007,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::UnreachableError,
                format!("ping call returned error num - {}, error - {}", 1007, e,),
                true
            ))
        }
    };
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
    let fn_name = "provision_by_code";
    tracing::trace!(
        func = fn_name,
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
                func = fn_name,
                package = PACKAGE_NAME,
                "provisioning manifest - {:?}",
                manifest
            );
            manifest
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error looking up manifest for code- {}",
                &code
            );
            bail!(e)
        } // throw error from manifest lookup
    };

    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "manifest response :{:?}",
        manifest
    );
    match perform_cryptography_operation(manifest.machine_id, manifest.cert_sign_url, settings)
        .await
    {
        Ok(_) => (),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error performing cryptography operation - {}",
                e
            );
            bail!(e)
        }
    }

    match event_tx.send(Event::Provisioning(events::ProvisioningEvent::Provisioned)) {
        Ok(_) => debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "provisioning event sent successfully"
        ),
        Err(e) => {
            error!(
                func = fn_name,
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
        func = fn_name,
        package = PACKAGE_NAME,
        result = "success",
        "machine provisioned successfully"
    );
    Ok(true)
}
async fn perform_cryptography_operation(
    machine_id: String,
    cert_sign_url: String,
    settings: AgentSettings,
) -> Result<bool> {
    let fn_name = "perform_cryptography_operation";
    // 2. Generate the private key based
    match generate_rsa_private_key(&settings.provisioning.paths.machine.private_key) {
        Ok(_) => debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "private key generated successfully"
        ),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error generating private key on path - {}",
                &settings.provisioning.paths.machine.private_key
            );
            bail!(e)
        }
    }

    // 3. Generate the CSR, using above private key
    match generate_csr(
        &settings.provisioning.paths.machine.csr,
        &settings.provisioning.paths.machine.private_key,
        machine_id.as_str().clone(),
    ) {
        Ok(_) => debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "csr generated successfully"
        ),
        Err(e) => {
            error!(
                func = fn_name,
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
        &machine_id.clone(),
        &cert_sign_url,
    )
    .await
    {
        Ok(signed_cer) => {
            debug!(
                func = fn_name,
                package = PACKAGE_NAME,
                "csr signed successfully"
            );
            signed_cer
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error signing csr for machine_id - {}",
                &machine_id
            );
            bail!(e)
        }
    };
    let ca_bundle_str = match serde_json::to_string(&signed_certificates.ca_bundle) {
        Ok(res) => res,
        Err(err) => {
            bail!(err)
        }
    };
    // 5. Store the signed certificates in destination path
    match write_certificates_to_path(
        &settings.provisioning.paths,
        signed_certificates.root_cert.as_bytes(),
        signed_certificates.cert.as_bytes(),
        ca_bundle_str.as_bytes(),
    ) {
        Ok(result) => {
            debug!(
                func = fn_name,
                package = PACKAGE_NAME,
                "certificates written successfully, result - {}",
                result
            );
            result
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error writing certificates to path - {:?}",
                &settings.provisioning.paths
            );
            bail!(e)
        }
    };
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
        &settings.provisioning.paths.ca_bundle.cert,
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
    let db_path = match construct_dir_path(&settings.settings.storage.path) {
        Ok(path) => {
            debug!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "db path constructed {:?}",
                path.display()
            );
            path
        }
        Err(e) => {
            error!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "error constructing db path - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::SettingsDatabaseDeleteError,
                format!(
                    "error constructing db path - {} - {}",
                    &settings.settings.storage.path, e
                ),
                true
            ))
        }
    };
    //2. Delete db
    match fs::remove_dir_all(&db_path) {
        Ok(_) => {
            debug!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "db deleted successfully from path - {:?}",
                &db_path
            )
        }
        Err(e) => {
            error!(
                func = "de_provision",
                package = PACKAGE_NAME,
                "error deleting db, from path {:?}, error - {}",
                &db_path,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::SettingsDatabaseDeleteError,
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
                    ProvisioningErrorCodes::InternalServerError,
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
                    ProvisioningErrorCodes::BadRequestError,
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
                    ProvisioningErrorCodes::NotFoundError,
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
                    ProvisioningErrorCodes::UnknownError,
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
                    ProvisioningErrorCodes::UnknownError,
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
                ProvisioningErrorCodes::ParseResponseError,
                format!("error parsing manifest lookup response - {}", e),
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
    cert: &[u8],
    ca_bundle: &[u8],
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
    match safe_write_to_path(&certificate_paths.ca_bundle.cert, ca_bundle) {
        Ok(_) => debug!(
            func = "write_file",
            package = PACKAGE_NAME,
            "ca_bundle certificate saved in path - {}",
            &certificate_paths.ca_bundle.cert
        ),
        Err(e) => {
            error!(
                func = "write_file",
                package = PACKAGE_NAME,
                "error saving ca_bundle certificate in path - {} - {}",
                &certificate_paths.ca_bundle.cert,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CertificateWriteError,
                format!(
                    "error saving ca_bundle certificate in path - {} - {}",
                    &certificate_paths.ca_bundle.cert, e
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
    println!("request url for sign csr: {:?}", url);

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
                    ProvisioningErrorCodes::InternalServerError,
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
                    ProvisioningErrorCodes::BadRequestError,
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
                    ProvisioningErrorCodes::NotFoundError,
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
                    ProvisioningErrorCodes::UnknownError,
                    format!("csr sign url returned unknown error - {}", e),
                    true
                ))
            }
            None => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::UnknownError,
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
                    ProvisioningErrorCodes::ParseResponseError,
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

async fn get_machine_id(identity_tx: mpsc::Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    match identity_tx
        .clone()
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error sending get machine id message - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ChannelSendMessageError,
                format!("error sending get machine id message - {}", e),
                true
            ));
        }
    }
    let machine_id = match recv_with_timeout(rx).await {
        Ok(id) => id,
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error receiving get machine id message - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ChannelReceiveMessageError,
                format!("error receiving get machine id message - {}", e),
                true
            ));
        }
    };
    info!(
        func = "get_machine_id",
        package = PACKAGE_NAME,
        "get machine id request completed",
    );
    Ok(machine_id)
}

fn parse_message_payload(payload: Bytes) -> Result<DeprovisionRequest> {
    let payload_value = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = "parse_message_payload",
                package = PACKAGE_NAME,
                "error converting payload to string - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ExtractMessagePayloadError,
                format!("Error converting payload to string - {}", e),
                true
            ))
        }
    };
    let payload: DeprovisionRequest = match serde_json::from_str(payload_value) {
        Ok(s) => s,
        Err(e) => {
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ExtractMessagePayloadError,
                format!("Error converting payload to AddTaskRequestPayload - {}", e),
                true
            ))
        }
    };
    Ok(payload)
}

pub async fn await_deprovision_message(
    identity_tx: mpsc::Sender<IdentityMessage>,
    event_tx: Sender<Event>,
    mut subscriber: NatsSubscriber,
) -> Result<()> {
    // Don't exit loop in any case by returning a response
    while let Some(message) = subscriber.next().await {
        let machine_id = match get_machine_id(identity_tx.clone()).await {
            Ok(id) => id,
            Err(e) => {
                error!(
                    func = "await_deprovision_message",
                    package = PACKAGE_NAME,
                    "error getting machine id - {}",
                    e
                );
                continue;
            }
        };
        // Parse payload and validate machine id
        let request_payload: DeprovisionRequest = match parse_message_payload(message.payload) {
            Ok(s) => s,
            Err(e) => {
                error!(
                    func = "await_deprovision_message",
                    package = PACKAGE_NAME,
                    "error getting machine id - {}",
                    e
                );
                continue;
            }
        };

        // Validate request machine id with current machine id is same or not
        if request_payload.machine_id != machine_id {
            error!(
                func = "handle_deprovision_message",
                package = PACKAGE_NAME,
                "error validating machine id in request - req_machine_id: {} - machine_id: {}",
                request_payload.machine_id,
                machine_id
            );
            continue;
        }

        match de_provision(event_tx.clone()) {
            Ok(_) => {
                info!(
                    func = "init",
                    package = PACKAGE_NAME,
                    result = "success",
                    "de provisioned successfully"
                );
            }
            Err(e) => {
                error!(
                    func = "init",
                    package = PACKAGE_NAME,
                    "error de provisioning - {}",
                    e
                );
                continue;
            }
        }
    }
    Ok(())
}

pub async fn await_re_issue_cert_message(
    identity_tx: mpsc::Sender<IdentityMessage>,
    event_tx: Sender<Event>,
    mut subscriber: NatsSubscriber,
) -> Result<()> {
    let fn_name = "await_re_issue_cert_message";
    // Don't exit loop in any case by returning a response
    while let Some(message) = subscriber.next().await {
        println!("message received on re issue certificate");
        // convert payload to string
        match process_re_issue_certificate_request(message.subject.to_string(), message.payload)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while processing re issue certificate request - {}",
                    e
                );
            }
        }
    }
    Ok(())
}

async fn process_re_issue_certificate_request(subject: String, payload: Bytes) -> Result<bool> {
    let fn_name = "process_services_re_issue_certificate_request";
    let settings = match read_settings_yml() {
        Ok(res) => res,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while deserializing message payload - {:?}",
                err
            );
            AgentSettings::default()
        }
    };
    // parse payload
    let payload_str = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(e) => {
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ExtractMessagePayloadError,
                format!("error converting payload to string - {}", e),
                true
            ))
        }
    };
    let request_payload: ReIssueCertificateRequest = match serde_json::from_str(&payload_str) {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while deserializing message payload - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::PayloadDeserializationError,
                format!("error while deserializing message payload {}", e),
                true
            ))
        }
    };
    let hashed_machine_id = sha256::digest(request_payload.machine_id.clone());
    // Validate request machine id with current machine id is same or not
    if !subject.contains(hashed_machine_id.as_str()) {
        bail!(ProvisioningError::new(
            ProvisioningErrorCodes::InvalidMachineIdError,
            format!(
                "invalid machine id in request - req_machine_id: {} ",
                request_payload.machine_id
            ),
            true
        ));
    };
    match perform_cryptography_operation(
        request_payload.machine_id.clone(),
        settings.provisioning.cert_sign_url.clone(),
        settings,
    )
    .await
    {
        Ok(_) => (),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error performing cryptography operation - {}",
                e
            );
            bail!(e)
        }
    }
    Ok(true)
}
