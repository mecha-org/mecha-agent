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
use crypto::x509::generate_ec_private_key;
use crypto::x509::PrivateKeyAlgorithm;
use crypto::x509::PrivateKeySize;
use events::Event;
use futures::StreamExt;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use messaging::Bytes;
use messaging::Subscriber as NatsSubscriber;
use serde::{Deserialize, Serialize};
use services_client::provisioning::{
    CertSignRequest, CertSignResponse, FindManifestRequest, ManifestDetailsResponse, PingResponse,
};
use services_client::ServicesClient;
use std::fs;
use std::str;
use std::str::FromStr;

use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::error;
use tracing::{debug, info, trace, warn};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    status: i32,
    message: String,
}

#[derive(Deserialize, Debug)]
struct DeprovisionRequest {
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

pub enum ProvisioningSubscriber {
    Deprovisioning { sub: NatsSubscriber },
}

pub async fn subscribe_to_nats(
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    event_tx: Sender<Event>,
) -> Result<NatsSubscriber> {
    // Get machine id
    let machine_id = match get_machine_id(identity_tx.clone()).await {
        Ok(id) => id,
        Err(e) => {
            error!(
                func = "subscribe",
                package = PACKAGE_NAME,
                "error getting machine id - {}",
                e
            );
            bail!(e)
        }
    };
    let (tx, rx) = oneshot::channel();
    match messaging_tx
        .send(MessagingMessage::Subscriber {
            reply_to: tx,
            subject: format!("machine.{}.deprovision", sha256::digest(machine_id.clone())),
        })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = "init",
                package = PACKAGE_NAME,
                "error sending subscriber message - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ChannelSendMessageError,
                format!("error sending subscriber message - {}", e),
                true
            ));
        }
    }
    let de_prov_subscriber = match recv_with_timeout(rx).await {
        Ok(id) => id,
        Err(e) => {
            error!(
                func = "init",
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

    Ok(de_prov_subscriber)
}

pub async fn ping() -> Result<PingResponse> {
    trace!(func = "ping", package = PACKAGE_NAME, "init",);
    let client = ServicesClient::new();
    let response = match client.ping().await {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "ping",
                package = PACKAGE_NAME,
                "error getting status - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::PingRequestError,
                format!("issue with grpc request: {}", e),
                true
            ))
        }
    };
    return Ok(PingResponse { success: response });
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

    let private_key_algorithm =
        match PrivateKeyAlgorithm::from_str(&manifest.cert_key_pair_algorithm) {
            Ok(algorithm) => algorithm,
            Err(e) => {
                error!(
                    func = "provision_by_code",
                    package = PACKAGE_NAME,
                    "error parsing private key algorithm"
                );
                bail!(e)
            }
        };
    let private_key_size = match PrivateKeySize::from_str(&manifest.cert_key_pair_size) {
        Ok(size) => size,
        Err(e) => {
            error!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "error parsing private key size"
            );
            bail!(e)
        }
    };
    // 2. Generate the private key based on the key algorithm
    match private_key_algorithm {
        PrivateKeyAlgorithm::ECDSA => match generate_ec_private_key(
            &settings.provisioning.paths.machine.private_key,
            private_key_size,
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

async fn lookup_manifest(settings: &AgentSettings, code: &str) -> Result<ManifestDetailsResponse> {
    trace!(
        func = "lookup_manifest",
        package = PACKAGE_NAME,
        "init, code - {:?}",
        code
    );

    let client = ServicesClient::new();
    let lookup_result = match client
        .find_manifest(FindManifestRequest {
            code: code.to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "lookup_manifest",
                package = PACKAGE_NAME,
                "error looking up manifest - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::ManifestLookupError,
                format!("error looking up manifest - {}", e),
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
    Ok(lookup_result)
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
) -> Result<CertSignResponse> {
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

    let client = ServicesClient::new();
    let cert_sign_response = match client
        .cert_sign(CertSignRequest {
            token: "".to_string(), //todo:check and fix
            csr: csr_pem,
            machine_id: machine_id.to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "lookup_manifest",
                package = PACKAGE_NAME,
                "error looking up manifest - {}",
                e
            );
            bail!(ProvisioningError::new(
                ProvisioningErrorCodes::CertSignError,
                format!("error signing certificate - {}", e),
                true
            ))
        }
    };
    Ok(cert_sign_response)
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
