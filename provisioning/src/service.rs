use crate::errors::ProvisioningError;
use crate::errors::ProvisioningErrorCodes;
use ::fs::construct_dir_path;
use ::fs::remove_files;
use ::fs::safe_write_to_path;
use agent_settings::constants;
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
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::error;
use tracing::{debug, info, trace};

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
    pub request_type: CertSignRequestType,
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
#[derive(Debug, Serialize, Deserialize)]
pub enum CertSignRequestType {
    Provision,
    ReIssue,
}

// Struct to hold the file paths and the associated byte data
struct CertFiles<'a> {
    root_cert_path: &'a str,
    cert_path: &'a str,
    ca_bundle_path: &'a str,
    root_cert: &'a [u8],
    cert: &'a [u8],
    ca_bundle: &'a [u8],
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
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "list of subjects - {:?}",
        list_of_subjects
    );
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
                ));
            }
        };
    }

    Ok(provisioning_subscribers)
}

pub async fn ping(service_url: &str) -> Result<PingResponse> {
    let fn_name = "ping";
    trace!(func = fn_name, package = PACKAGE_NAME, "init");
    let ping_service_url = format!("{}{}", service_url, constants::PING_QUERY_PATH);
    let client = reqwest::Client::builder().build().unwrap();

    let result = client
        .get(ping_service_url)
        .header("CONTENT_TYPE", "application/json")
        .header("ACCEPT", "application/json")
        .send()
        .await;

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
                            format!("ping call returned unauthorized error num - {}", 1002),
                        ))
                    }
                    StatusCode::NOT_FOUND => {
                        error!(
                            func = "ping",
                            package = PACKAGE_NAME,
                            "ping call returned not found error num - {}",
                            1003
                        );
                        bail!(ProvisioningError::new(
                            ProvisioningErrorCodes::NotFoundError,
                            format!("ping call returned not found error num - {}", 1003),
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
                            format!("ping call returned bad request num - {}", 1004),
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
                            format!("ping call returned internal server error num - {}", 1005),
                        ))
                    }
                    _ => {
                        error!(
                            func = "ping",
                            package = PACKAGE_NAME,
                            "ping call returned unknown error num - {}",
                            1006
                        );
                        bail!(ProvisioningError::new(
                            ProvisioningErrorCodes::UnknownError,
                            format!("ping call returned unknown error num - {}", 1006),
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
pub async fn provision_by_code(
    service_url: &str,
    data_dir: &str,
    code: &str,
    event_tx: Sender<Event>,
) -> Result<bool> {
    let fn_name = "provision_by_code";
    tracing::trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "init code - {:?}",
        code
    );

    // 1. Lookup the manifest, if lookup fails with not found then return error
    let manifest = match lookup_manifest(service_url, code).await {
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

    match perform_cryptography_operation(
        service_url,
        &manifest.machine_id,
        &manifest.cert_sign_url,
        data_dir,
        CertSignRequestType::Provision,
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

    match event_tx.send(Event::Provisioning(events::ProvisioningEvent::Provisioned)) {
        Ok(_) => trace!(
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
    service_url: &str,
    machine_id: &str,
    cert_sign_url: &str,
    data_dir: &str,
    request_type: CertSignRequestType,
) -> Result<bool> {
    let fn_name = "perform_cryptography_operation";

    // Construct the paths for the certificates
    let private_key_path = data_dir.to_owned() + constants::PRIVATE_KEY_PATH;
    let csr_path = data_dir.to_owned() + constants::CSR_PATH;
    let root_cert_path = data_dir.to_owned() + constants::ROOT_CERT_PATH;
    let cert_path = data_dir.to_owned() + constants::CERT_PATH;
    let ca_bundle_path = data_dir.to_owned() + constants::CA_BUNDLE_PATH;

    // 2. Generate the private key based
    match generate_rsa_private_key(&private_key_path) {
        Ok(_) => trace!(
            func = fn_name,
            package = PACKAGE_NAME,
            "private key generated successfully"
        ),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error generating private key on path - {}",
                private_key_path
            );
            bail!(e)
        }
    }

    // 3. Generate the CSR, using above private key
    match generate_csr(&csr_path, &private_key_path, machine_id) {
        Ok(_) => trace!(
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
        service_url,
        &csr_path,
        machine_id,
        cert_sign_url,
        request_type,
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
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error converting ca_bundle to string - {}",
                err
            );
            bail!(err)
        }
    };

    let cert_files = CertFiles {
        root_cert_path: &root_cert_path,
        cert_path: &cert_path,
        ca_bundle_path: &ca_bundle_path,
        root_cert: signed_certificates.root_cert.as_bytes(),
        cert: signed_certificates.cert.as_bytes(),
        ca_bundle: ca_bundle_str.as_bytes(),
    };
    // 5. Store the signed certificates in destination path
    match write_certificates_to_path(cert_files) {
        Ok(result) => {
            info!(
                func = fn_name,
                package = PACKAGE_NAME,
                "certificates written successfully",
            );
            result
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error writing certificates to path - {:?}",
                e
            );
            bail!(e)
        }
    };
    Ok(true)
}

pub fn de_provision(data_dir: &str, event_tx: Sender<Event>) -> Result<bool> {
    let fn_name = "de_provision";
    trace!(func = fn_name, package = PACKAGE_NAME, "init",);
    //1. Delete certs
    match remove_files(vec![
        &(data_dir.to_owned() + constants::CERT_PATH),
        &(data_dir.to_owned() + constants::PRIVATE_KEY_PATH),
        &(data_dir.to_owned() + constants::CSR_PATH),
        &(data_dir.to_owned() + constants::CA_BUNDLE_PATH),
        &(data_dir.to_owned() + constants::ROOT_CERT_PATH),
    ]) {
        Ok(_) => {
            trace!(
                func = fn_name,
                package = PACKAGE_NAME,
                "certificates deleted successfully"
            );
        }
        Err(e) => {
            error!(
                func = fn_name,
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
        Ok(_) => trace!(
            func = fn_name,
            package = PACKAGE_NAME,
            "de provisioning event sent successfully"
        ),
        Err(e) => {
            error!(
                func = fn_name,
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
            ));
        }
    }

    //TODO: Move this to settings service on deprovision event
    let storage_path = data_dir.to_owned() + constants::DB_PATH;
    let db_path = match construct_dir_path(&storage_path) {
        Ok(path) => {
            debug!(
                func = fn_name,
                package = PACKAGE_NAME,
                "db path constructed {:?}",
                path.display()
            );
            path
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error constructing db path - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::SettingsDatabaseDeleteError,
                format!("error constructing db path - {}", e),
            ))
        }
    };

    //TODO: Move this to settings service on deprovision event
    // 2. Flush database
    // let key_value_store = KeyValueStoreClient::new();
    // match key_value_store.flush_database() {
    //     Ok(_) => {
    //         trace!(
    //             func = fn_name,
    //             package = PACKAGE_NAME,
    //             "db flushed successfully"
    //         )
    //     }
    //     Err(e) => {
    //         error!(
    //             func = fn_name,
    //             package = PACKAGE_NAME,
    //             "error flushing db - {}",
    //             e
    //         );
    //         bail!(e);
    //     }
    // }

    //TODO: Move this to settings service on deprovision event
    //3. Delete db
    match fs::remove_dir_all(&db_path) {
        Ok(_) => {
            debug!(
                func = fn_name,
                package = PACKAGE_NAME,
                "db deleted successfully from path - {:?}",
                &db_path
            )
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error deleting db, from path {:?}, error - {}",
                &db_path,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::SettingsDatabaseDeleteError,
                format!("error deleting db, code: {}, error - {}", 1001, e),
            ));
        }
    }

    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        result = "success",
        "de provisioned successful",
    );
    Ok(true)
}

async fn lookup_manifest(service_url: &str, code: &str) -> Result<ProvisioningManifest> {
    let fn_name = "lookup_manifest";
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "init, code - {:?}",
        code
    );
    let url = format!(
        "{}{}{}",
        service_url,
        constants::FIND_MANIFEST_URL_QUERY_PATH,
        code
    );
    debug!(
        func = fn_name,
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
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "manifest find endpoint url returned internal server error for url - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::InternalServerError,
                    format!("manifest find endpoint url returned server error - {}", e),
                ))
            }
            Some(StatusCode::BAD_REQUEST) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "manifest find endpoint url returned bad request - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::BadRequestError,
                    format!("manifest find endpoint url returned bad request - {}", e),
                ))
            }
            Some(StatusCode::NOT_FOUND) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "manifest find endpoint url not found - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::NotFoundError,
                    format!("manifest find endpoint url not found - {}", e),
                ))
            }
            Some(_) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "manifest find endpoint url returned unknown error - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::UnknownError,
                    format!("manifest find endpoint url returned unknown error - {}", e),
                ))
            }
            None => {
                error!(
                    func = fn_name,
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
                func = fn_name,
                package = PACKAGE_NAME,
                "manifest lookup response - {:?}",
                parse_manifest
            );
            parse_manifest
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error parsing manifest lookup response - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ParseResponseError,
                format!("error parsing manifest lookup response - {}", e),
            ))
        }
    };

    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        result = "success",
        "manifest lookup successful"
    );
    Ok(manifest_response.payload)
}

fn write_certificates_to_path(cert_files: CertFiles) -> Result<bool> {
    let fn_name = "write_certificates_to_path";
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "cert path - {}",
        &cert_files.cert_path,
    );

    // save the machine certificate
    match safe_write_to_path(&cert_files.cert_path, &cert_files.cert) {
        Ok(_) => debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "machine certificate saved in path - {}",
            &cert_files.cert_path
        ),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error saving machine certificate in path - {} - {}",
                &cert_files.cert_path,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CertificateWriteError,
                format!(
                    "error saving machine certificate in path - {} - {}",
                    &cert_files.cert_path, e
                ),
            ))
        }
    }

    // save the intermediate certificate
    match safe_write_to_path(&cert_files.ca_bundle_path, &cert_files.ca_bundle) {
        Ok(_) => debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "ca_bundle certificate saved in path - {}",
            &cert_files.ca_bundle_path
        ),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error saving ca_bundle certificate in path - {} - {}",
                &cert_files.ca_bundle_path,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CertificateWriteError,
                format!(
                    "error saving ca_bundle certificate in path - {} - {}",
                    &cert_files.ca_bundle_path, e
                ),
            ))
        }
    }

    // save the root certificate
    match safe_write_to_path(&cert_files.root_cert_path, &cert_files.root_cert) {
        Ok(_) => debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "root certificate saved in path - {}",
            &cert_files.root_cert_path
        ),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error saving root certificate in path - {} - {}",
                &cert_files.root_cert_path,
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CertificateWriteError,
                format!(
                    "error saving root certificate in path - {} - {}",
                    &cert_files.root_cert_path, e
                ),
            ))
        }
    }

    info!(
        func = fn_name,
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
    request_type: CertSignRequestType,
) -> Result<SignedCertificates> {
    let fn_name = "sign_csr";
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "init, request_url {}, csr_sign_url {}",
        request_url,
        cert_signing_url
    );

    let constructed_path = match construct_dir_path(csr_path) {
        Ok(path) => {
            debug!(
                func = fn_name,
                package = PACKAGE_NAME,
                "csr path constructed {:?}",
                path.display()
            );
            path
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error constructing csr path - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignReadFileError,
                format!("error opening csr in path - {} - {}", csr_path, e),
            ))
        }
    };
    let csr_pem = match fs::read_to_string(constructed_path) {
        Ok(csr_str) => {
            debug!(
                func = fn_name,
                package = PACKAGE_NAME,
                "read csr as string - {:?}",
                csr_str
            );
            csr_str
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error reading csr as string - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CSRSignReadFileError,
                format!("error reading csr in path - {} - {}", csr_path, e),
            ))
        }
    };

    // Construct payload for signing the csr
    let sign_csr_request_body = SignCSRRequest {
        csr: csr_pem,
        machine_id: machine_id.to_string(),
        request_type: request_type,
    };

    // Format url for signing the csr
    let url = format!("{}{}", request_url, cert_signing_url);
    debug!(
        func = fn_name,
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
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "csr sign url returned internal server error - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::InternalServerError,
                    format!("csr sign url returned server error - {}", e),
                ))
            }
            Some(StatusCode::BAD_REQUEST) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "csr sign url returned bad request - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::BadRequestError,
                    format!("csr sign url returned bad request - {}", e),
                ))
            }
            Some(StatusCode::NOT_FOUND) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "csr sign url not found - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::NotFoundError,
                    format!("csr sign url not found - {}", e),
                ))
            }
            Some(_) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "csr sign url returned unknown error - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::UnknownError,
                    format!("csr sign url returned unknown error - {}", e),
                ))
            }
            None => bail!(ProvisioningError::new(
                ProvisioningErrorCodes::UnknownError,
                format!("csr sign url returned unmatched error - {}", e),
            )),
        },
    };
    let result: ProvisioningServerResponseGeneric<SignedCertificates> =
        match serde_json::from_str(&csr_string) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error parsing csr sign response - {}",
                    e
                );
                bail!(ProvisioningError::new(
                    ProvisioningErrorCodes::ParseResponseError,
                    format!("error parsing csr sign response - {}", e),
                ));
            }
        };
    info!(
        func = fn_name,
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
            ))
        }
    };
    let payload: DeprovisionRequest = match serde_json::from_str(payload_value) {
        Ok(s) => s,
        Err(e) => {
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ExtractMessagePayloadError,
                format!("Error converting payload to AddTaskRequestPayload - {}", e),
            ))
        }
    };
    Ok(payload)
}

pub async fn await_deprovision_message(
    data_dir: String,
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

        match de_provision(&data_dir, event_tx.clone()) {
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
    service_url: String,
    data_dir: String,
    mut subscriber: NatsSubscriber,
) -> Result<()> {
    let fn_name = "await_re_issue_cert_message";
    // Don't exit loop in any case by returning a response
    while let Some(message) = subscriber.next().await {
        println!("message received on re issue certificate");

        // convert payload to string
        match process_re_issue_certificate_request(
            &service_url,
            &data_dir,
            message.subject.as_str(),
            message.payload,
        )
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

async fn process_re_issue_certificate_request(
    service_url: &str,
    data_dir: &str,
    subject: &str,
    payload: Bytes,
) -> Result<bool> {
    let fn_name = "process_services_re_issue_certificate_request";
    // parse payload
    let payload_str = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error converting payload to string - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ExtractMessagePayloadError,
                format!("error converting payload to string - {}", e),
            ))
        }
    };
    let request_payload: ReIssueCertificateRequest = match serde_json::from_str(&payload_str) {
        Ok(s) => {
            debug!(
                func = fn_name,
                package = PACKAGE_NAME,
                "re issue certificate request payload - {:?}",
                s
            );
            s
        }
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
        ));
    };

    let cert_sign_url = service_url.to_owned() + constants::CERT_SIGN_URL_QUERY_PATH;
    match perform_cryptography_operation(
        service_url,
        &request_payload.machine_id,
        &cert_sign_url,
        data_dir,
        CertSignRequestType::ReIssue,
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
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "re_issue certificate request processed!"
    );
    Ok(true)
}
