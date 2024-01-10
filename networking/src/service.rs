use crate::errors::NetworkingError;
use crate::errors::NetworkingErrorCodes;
use crate::nebula::generate_nebula_key_cert;
use crate::nebula::is_cert_valid;
use crate::nebula::is_cert_verified;
use crate::nebula::start_nebula;
use crate::nebula::FirewallRule;
use crate::nebula::NebulaSettings;
use crate::utils::extract_tar_file;
use crate::utils::extract_zip_file;
use crate::utils::is_sudo;
use crate::utils::sha256_file;
use ::fs::construct_dir_path;
use agent_settings::read_settings_yml;
use agent_settings::AgentSettings;
use anyhow::{bail, Result};
use channel::recv_with_timeout;
use crypto::base64::b64_decode;
use identity::handler::IdentityMessage;
use ipaddress::IPAddress;
use messaging::handler::MessagingMessage;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use settings::handler::SettingMessage;
use sha256::digest;
use std::env::temp_dir;
use std::fs;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::copy;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::str;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tracing::trace;
use tracing::warn;
use tracing::{debug, error, info};

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetworkingServerResponseGeneric<T> {
    pub success: bool,
    pub status: String,
    pub status_code: i16,
    pub message: Option<String>,
    pub error_code: Option<String>,
    pub sub_errors: Option<String>,
    pub payload: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderMetadataPayload {
    pub app_name: String,
    pub os: String,
    pub arch: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderMetadataReply {
    pub name: String,
    pub file_name: String,
    pub file_type: String,
    pub download_url: String,
    pub checksum: String,
    pub base_config: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct IssueCertReq {
    pub provider: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct IssueCertRes {
    pub cert: String,
    pub cert_fingerprint: String,
    pub ca_cert: String,
    pub cert_valid_upto: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkingFirewallRules {
    pub machine_id: String,
    pub name: String,
    pub direction: String,
    pub roles: String,
    pub protocol: String,
    pub allow_ports: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct OverrideConfigurations {
    pub cert: String,
    pub key: String,
    pub ca: String,
    pub networking_firewall_rules: Vec<NetworkingFirewallRules>,
}

#[derive(Debug)]
pub struct StartRes {
    pub nebula_process: tokio::process::Child,
}

pub async fn get_provider_info(
    settings_tx: Sender<SettingMessage>,
) -> Result<ProviderMetadataPayload> {
    let fn_name = "get_provider_info";
    let (tx, rx) = oneshot::channel();
    match settings_tx
        .clone()
        .send(SettingMessage::GetSettingsByKey {
            reply_to: tx,
            key: String::from("networking.provider.name"),
        })
        .await
    {
        Ok(_) => (),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending message to get provider name - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!(
                    "error sending message to get provider name - {}",
                    e.to_string()
                ),
                true
            ))
        }
    }

    let app_name = match recv_with_timeout(rx).await {
        Ok(r) => r,
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelReceiveMessageError,
                format!(
                    "error receiving message to get provider name - {}",
                    e.to_string()
                ),
                true
            ))
        }
    };

    let arch = std::env::consts::ARCH.to_lowercase();
    debug!(func = fn_name, package = PACKAGE_NAME, "arch is {}", arch);
    if arch.is_empty() {
        error!(func = fn_name, package = PACKAGE_NAME, "arch not found");
        bail!(NetworkingError::new(
            NetworkingErrorCodes::SystemArchNotFoundError,
            format!("arch not found",),
            true
        ))
    };

    let os = std::env::consts::OS.to_lowercase();
    debug!(func = fn_name, package = PACKAGE_NAME, "os is {}", os);
    if os.is_empty() {
        error!(func = fn_name, package = PACKAGE_NAME, "os name not found");
        bail!(NetworkingError::new(
            NetworkingErrorCodes::SystemOsNotFoundError,
            format!("os name not found",),
            true
        ))
    };

    let provider_metadata_payload = ProviderMetadataPayload { app_name, os, arch };
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "provider metadata payload is {:?}",
        provider_metadata_payload
    );

    Ok(provider_metadata_payload)
}

pub async fn get_provider_configs(
    messaging_tx: Sender<MessagingMessage>,
    topic_to_publish: &str,
    provider_metadata_payload: &ProviderMetadataPayload,
) -> Result<ProviderMetadataReply> {
    let fn_name = "get_provider_configs";
    let (tx, rx) = oneshot::channel();
    let payload_payload_json = json!(provider_metadata_payload);
    match messaging_tx
        .clone()
        .send(MessagingMessage::Request {
            subject: topic_to_publish.to_string(),
            message: payload_payload_json.to_string(),
            reply_to: tx,
        })
        .await
    {
        Ok(_) => (),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error sending message to get provider config - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!(
                    "error sending message to get provider config - {}",
                    e.to_string()
                ),
                true
            ))
        }
    }

    let response_bytes = match recv_with_timeout(rx).await {
        Ok(r) => r,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error receiving message to get provider config - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelReceiveMessageError,
                format!(
                    "unable to receive provider config, error - {}",
                    e.to_string()
                ),
                true
            ))
        }
    };
    let provider_metadata_reply: ProviderMetadataReply = match parse_message_payload(response_bytes)
    {
        Ok(r) => r,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error parsing provider config - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::MessagingRequestProviderConfigError,
                format!("unable to parse provider config, error - {}", e.to_string()),
                false
            ))
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "provider metadata reply is {:?}",
        provider_metadata_reply
    );
    Ok(provider_metadata_reply)
}

pub async fn extract_provider_package(
    provider_dir: &str,
    provider_config: &ProviderMetadataReply,
) -> Result<bool> {
    let fn_name = "extract_provider_package";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "provider dir is {}",
        provider_dir
    );
    let provider_dir_exists = Path::new(&provider_dir).is_dir();

    if !provider_dir_exists {
        match create_dir_all(&provider_dir) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while creating provider directory, path - {}, error - {}",
                    provider_dir,
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderDirectoryCreateError,
                    format!(
                        "provider directory create error, path - {},  - {}",
                        provider_dir,
                        e.to_string()
                    ),
                    true
                ))
            }
        };
    }

    let package_file_path = format!("{}/{}", provider_dir, provider_config.file_name);
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "package file path is {}",
        package_file_path
    );
    let package_exists = Path::new(&package_file_path).is_file();
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "package exists is {}",
        package_exists
    );
    let mut package_checksum_mismatch = false;

    if package_exists {
        let package_checksum = match sha256_file(&package_file_path) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while calculating package checksum, path - {}, error - {}",
                    package_file_path,
                    e.to_string()
                );
                String::from("")
            }
        };

        debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "package checksum is {}",
            package_checksum
        );

        package_checksum_mismatch = package_checksum.ne(&provider_config.checksum);
    }

    //Download provider file using download url
    if !package_exists || package_checksum_mismatch {
        let response = match reqwest::get(&provider_config.download_url).await {
            Ok(r) => r,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while downloading provider package - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderDownloadError,
                    format!(
                        "error while downloading provider package - {}",
                        e.to_string()
                    ),
                    true
                ))
            }
        };
        let mut file = match File::create(&package_file_path) {
            Ok(r) => r,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while creating package file - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderFileCreateError,
                    format!("error while creating package file - {}", e.to_string()),
                    true
                ))
            }
        };
        match copy(&mut response.bytes().await?.as_ref(), &mut file) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while writing package file - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderFileWriteError,
                    format!("error while writing package file - {}", e.to_string()),
                    true
                ))
            }
        };
        debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "package file downloaded successfully, saved at {}",
            package_file_path
        )
    }

    //Extract provider package in temp
    let extract_to_path: PathBuf = temp_dir();

    let package_extracted = match provider_config.file_type.as_str() {
        "zip" => match extract_zip_file(&package_file_path, &extract_to_path).await {
            Ok(r) => r,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while extracting package file - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderFileExtractError,
                    format!("error while extracting package file - {}", e.to_string()),
                    true
                ))
            }
        },
        "tar.gz" => match extract_tar_file(&package_file_path, &extract_to_path).await {
            Ok(r) => r,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while extracting package file - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderFileExtractError,
                    format!("error while extracting package file - {}", e.to_string()),
                    true
                ))
            }
        },
        _ => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while extracting package file - provider file type invalid"
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::InvalidProviderFileType,
                format!("error while extracting package file - provider file type invalid"),
                true
            ))
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "package file extracted successfully, saved at {}",
        package_extracted
    );
    Ok(package_extracted)
}

pub async fn validate_or_create_certs(
    settings_tx: Sender<SettingMessage>,
    certs_dir: &str,
    provider_config: &ProviderMetadataReply,
) -> Result<bool> {
    let fn_name = "validate_or_create_certs";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "certs dir is {}",
        certs_dir
    );

    let certs_dir_exists = Path::new(&certs_dir).is_dir();

    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "certs dir exists is {}",
        certs_dir_exists
    );

    if !certs_dir_exists {
        match create_dir_all(&certs_dir) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while creating certs directory, path - {}, error - {}",
                    certs_dir,
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CertsDirectoryCreateError,
                    format!("certs directory create error - {}", e.to_string()),
                    true
                ))
            }
        };
    }

    //Check certs exists
    let unsigned_cert_path = format!("{}/unsigned.crt", certs_dir);
    let key_path = format!("{}/machine.key", certs_dir);
    let signed_cert_path = format!("{}/machine.crt", certs_dir);
    let ca_path = format!("{}/ca.crt", certs_dir);

    let are_certs_valid = match check_existing_certs_valid(&ca_path, &signed_cert_path, &key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while validating existing certs - {}",
                e.to_string()
            );
            false
        }
    };

    if !are_certs_valid {
        //Enrollment process
        match generate_nebula_key_cert(&unsigned_cert_path, &key_path) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while generating certs to path - {}, error - {}",
                    &key_path,
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CertsGenerateError,
                    format!("error while generating certs {}", e.to_string()),
                    true
                ))
            }
        };

        let enrollment_url =
            match get_settings_by_key(settings_tx, String::from("networking.enrollment.url")).await
            {
                Ok(v) => v,
                Err(e) => {
                    error!(
                        func = fn_name,
                        package = PACKAGE_NAME,
                        "error while reading enrollment url - {}",
                        e.to_string()
                    );
                    bail!(NetworkingError::new(
                        NetworkingErrorCodes::MachineSettingsEnrollmentUrlFoundError,
                        format!("error while reading enrollment url {}", e.to_string()),
                        true
                    ))
                }
            };

        let raw_unsigned_cert = match fs::read_to_string(PathBuf::from(&unsigned_cert_path)) {
            Ok(v) => v,
            Err(e) => {
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CertReadFileError,
                    format!(
                        "unable to read cert from path {} error - {}",
                        &unsigned_cert_path,
                        e.to_string()
                    ),
                    true
                ))
            }
        };

        let sign_cert_req = IssueCertReq {
            provider: String::from(&provider_config.name),
            public_key: raw_unsigned_cert,
        };

        debug!(
            fn_name,
            package = PACKAGE_NAME,
            "sign cert request is {:?}",
            sign_cert_req
        );

        let sign_cert_res = match sign_cert(&enrollment_url, sign_cert_req).await {
            Ok(r) => r,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while signing cert - {}",
                    e.to_string()
                );
                bail!(e);
            }
        };
        debug!(
            fn_name,
            package = PACKAGE_NAME,
            "sign cert successfully, response is {:?}",
            sign_cert_res
        );

        let decoded_cert_bytes = match b64_decode(&sign_cert_res.cert) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while decoding signed cert - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertDecodeError,
                    format!("error in decoding signed cert - {}", e.to_string()),
                    true
                ))
            }
        };

        // Convert the decoded bytes to a UTF-8 string
        let decoded_cert_str = match str::from_utf8(&decoded_cert_bytes) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while converting signed cert to string - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertDecodeError,
                    format!(
                        "error while converting signed cert to string- {}",
                        e.to_string()
                    ),
                    true
                ))
            }
        };

        debug!(fn_name, package = PACKAGE_NAME, "cert decoded successfully");

        let mut file = match File::create(format!("{}/machine.crt", certs_dir)) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while creating signed cert file to path - {}, error - {}",
                    &certs_dir,
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertFileCreateError,
                    format!("error while creating signed cert file- {}", e.to_string()),
                    true
                ))
            }
        };
        match file.write_all(decoded_cert_str.as_bytes()) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while saving signed cert file to path - {}, error - {}",
                    &certs_dir,
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertFileSaveError,
                    format!("error while saving signed cert file- {}", e.to_string()),
                    true
                ))
            }
        };
        let decoded_ca_cert_bytes = match b64_decode(&sign_cert_res.ca_cert) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while decoding ca cert - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CaCertDecodeError,
                    format!("error in decoding ca cert - {}", e.to_string()),
                    true
                ))
            }
        };

        // Convert the decoded bytes to a UTF-8 string
        let decoded_ca_cert_str = match str::from_utf8(&decoded_ca_cert_bytes) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while converting ca cert to string - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CaCertConvertStringError,
                    format!(
                        "error while converting ca cert to string- {}",
                        e.to_string()
                    ),
                    true
                ))
            }
        };

        debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "ca cert decoded successfully"
        );

        let mut ca_cert_file = match File::create(format!("{}/ca.crt", certs_dir)) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while creating ca cert file to path - {}, error - {}",
                    &certs_dir,
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CaCertFileCreateError,
                    format!("error while creating ca cert file- {}", e.to_string()),
                    true
                ))
            }
        };
        match ca_cert_file.write_all(decoded_ca_cert_str.as_bytes()) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while saving ca cert file to path - {}, error - {}",
                    &certs_dir,
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CaCertFileSaveError,
                    format!("error while saving ca cert file- {}", e.to_string()),
                    true
                ))
            }
        };
    }
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "certs are valid and available"
    );
    Ok(true)
}

pub fn create_firewall_rules(roles: &str, port: &str, protocol: &str) -> Vec<FirewallRule> {
    let mut firewall_rules: Vec<FirewallRule> = vec![];
    roles.split(",").into_iter().for_each(|role| {
        let mut firewall_rule = FirewallRule {
            port: String::from(port),
            proto: protocol.to_lowercase(),
            ..Default::default()
        };
        //If IP address then add as cidr
        if IPAddress::is_valid(role) {
            firewall_rule.cidr = Some(role.to_owned());
        }
        //If colon separated string, use group
        else if role.contains(":") {
            firewall_rule.group = Some(role.to_owned());
        }
        //If any use CIDR 0.0.0.0/0
        else if role == "any" {
            firewall_rule.cidr = Some("0.0.0.0/0".to_string());
        }
        //If role is a string then use host
        else {
            firewall_rule.host = Some(role.to_owned());
        }

        firewall_rules.push(firewall_rule);
    });
    firewall_rules
}

pub fn get_firewall_rules(
    networking_firewall_rules: &Vec<NetworkingFirewallRules>,
) -> (Vec<FirewallRule>, Vec<FirewallRule>) {
    let mut inbound_firewall_rules: Vec<FirewallRule> = vec![];
    let mut outbound_firewall_rules: Vec<FirewallRule> = vec![];
    networking_firewall_rules
        .into_iter()
        .filter(|machine_firewall_rule| machine_firewall_rule.status.to_lowercase() == "active")
        .for_each(|machine_firewall_rule| {
            machine_firewall_rule
                .allow_ports
                .split(",")
                .into_iter()
                .for_each(|port| {
                    let firewall_rules = create_firewall_rules(
                        &machine_firewall_rule.roles,
                        port,
                        &machine_firewall_rule.protocol,
                    );
                    match machine_firewall_rule.direction.to_uppercase().as_str() {
                        "INBOUND" => {
                            inbound_firewall_rules.extend_from_slice(&firewall_rules);
                        }
                        "OUTBOUND" => {
                            outbound_firewall_rules.extend_from_slice(&firewall_rules);
                        }
                        _ => {}
                    }
                });
        });
    (inbound_firewall_rules, outbound_firewall_rules)
}

pub async fn generate_nebula_configuartion_file(
    encoded_base_config: &str,
    overide_configurations: OverrideConfigurations,
) -> Result<bool> {
    let fn_name = "generate_nebula_configuartion_file";

    let decoded_bytes = match b64_decode(encoded_base_config) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while decoding base config - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaBaseConfigParseError,
                format!("unable to decode nebula base config {}", e.to_string()),
                true
            ))
        }
    };

    // Convert the decoded bytes to a UTF-8 string
    let decoded_str = match str::from_utf8(&decoded_bytes) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while converting base config to string - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaBaseConfigParseError,
                format!("failed to convert bytes to string {}", e.to_string()),
                true
            ))
        }
    };

    // Deserialize the string into the NebulaSettings struct
    let mut nebula_settings: NebulaSettings = match serde_yaml::from_str(decoded_str) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while deserializing base config - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaConfigDeSerializeError,
                format!("failed to deserialize {}", e.to_string()),
                true
            ))
        }
    };

    nebula_settings.pki.cert = overide_configurations.cert.to_string();
    nebula_settings.pki.key = overide_configurations.key.to_string();
    nebula_settings.pki.ca = overide_configurations.ca.to_string();

    let (inbound_firewall_roles, outbound_firewall_roles) =
        get_firewall_rules(&overide_configurations.networking_firewall_rules);

    nebula_settings.firewall.inbound = inbound_firewall_roles;
    nebula_settings.firewall.outbound = outbound_firewall_roles;

    // Serialize NebulaSettings into a YAML-formatted string
    let yaml_string = match serde_yaml::to_string(&nebula_settings) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while serializing base config - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaConfigSerializeError,
                format!("failed to serialize to YAML {}", e.to_string()),
                true
            ))
        }
    };

    let mut temp_dir = temp_dir();
    temp_dir.push("config.yaml");

    // Write the YAML string to a file named "config.yaml"
    let mut file = match File::create(&temp_dir) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while creating file - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaConfigFileCreateError,
                format!("failed to create file {}", e.to_string()),
                true
            ))
        }
    };
    match file.write_all(yaml_string.as_bytes()) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while writing file on dir - {:?}, error - {}",
                &temp_dir,
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaConfigFileCreateError,
                format!("failed to save nebula config file {}", e.to_string()),
                true
            ))
        }
    }
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "nebula config file created successfully"
    );
    Ok(true)
}

pub async fn get_networking_firewall_rules(
    settings_tx: Sender<SettingMessage>,
) -> Result<Vec<NetworkingFirewallRules>> {
    let fn_name = "get_networking_firewall_rules";
    let networking_firewall_rules_str =
        match get_settings_by_key(settings_tx, String::from("networking.firewall.rules")).await {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while reading firewall rules - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::MachineSettingsNetworkingFirewallRulesNotFoundError,
                    format!("error while reading firewall rules {}", e.to_string()),
                    true
                ))
            }
        };

    let networking_firewall_rules: Vec<NetworkingFirewallRules> =
        match serde_json::from_str(&networking_firewall_rules_str) {
            Ok(s) => s,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while parsing firewall rules - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ExtractNetworkingFirewallRulesPayloadError,
                    format!(
                        "error converting payload to NetworkingFirewallRules - {}",
                        e
                    ),
                    true
                ))
            }
        };

    Ok(networking_firewall_rules)
}

pub async fn start(
    setting_tx: Sender<SettingMessage>,
    identity_tx: Sender<IdentityMessage>,
    messaging_tx: Sender<MessagingMessage>,
) -> Result<StartRes> {
    let fn_name = "start";

    let agent_settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => {
            warn!(
                func = "networking_start",
                package = PACKAGE_NAME,
                "settings.yml not found, using default settings"
            );
            AgentSettings::default()
        }
    };

    //Get provider info from settings
    let provider_metadata_payload = match get_provider_info(setting_tx.clone()).await {
        Ok(r) => r,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while getting provider info - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ProviderMetadataPayloadCreateError,
                format!(
                    "networking provider metadata payload create, error - {}",
                    e.to_string()
                ),
                true
            ))
        }
    };
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "provider metadata payload is {:?}",
        provider_metadata_payload
    );

    // Get machine id
    let machine_id = match get_machine_id(identity_tx).await {
        Ok(r) => r,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while getting machine id - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::MachineSettingsMachineIdNotFoundError,
                format!("unable to get machine id, error - {}", e.to_string()),
                true
            ))
        }
    };

    //Get provider config
    let topic_to_publish = format!(
        "machine.{}.networking.provider.metadata",
        digest(machine_id.to_string())
    );
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "topic to publish is - {}",
        topic_to_publish
    );
    let provider_config: ProviderMetadataReply =
        match get_provider_configs(messaging_tx, &topic_to_publish, &provider_metadata_payload)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error while getting provider config - {}",
                    e.to_string()
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::GetProviderConfigsError,
                    format!("unable to get provider configs, error - {}", e.to_string()),
                    true
                ))
            }
        };

    // Save provider package binaries in temp
    let provider_dir = format!(
        "{}/{}",
        &agent_settings.networking.data_dir, provider_config.name
    );

    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "provider dir is {}",
        provider_dir
    );

    match extract_provider_package(&provider_dir, &provider_config).await {
        Ok(_) => (),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while extracting provider package - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ProviderBinariesSaveError,
                format!(
                    "unable to save provider package binaries, error - {}",
                    e.to_string()
                ),
                true
            ))
        }
    };

    let certs_dir = format!("{}/certs", provider_dir);
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "certs dir is {}",
        certs_dir
    );
    match validate_or_create_certs(setting_tx.clone(), &certs_dir, &provider_config).await {
        Ok(_) => (),
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::CertsValidateOrCreateError,
                format!(
                    "unable to create or validate certs, error - {}",
                    e.to_string()
                ),
                true
            ))
        }
    };

    if !is_sudo() {
        error!(
            func = fn_name,
            package = PACKAGE_NAME,
            "thread does not have sudo permissions"
        );
        bail!(NetworkingError::new(
            NetworkingErrorCodes::SudoCheckFailed,
            format!("sudo check failed",),
            false
        ))
    }

    let networking_firewall_rules = match get_networking_firewall_rules(setting_tx.clone()).await {
        Ok(v) => v,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while getting networking firewall rules - {}",
                err.to_string()
            );
            vec![]
        }
    };

    let overide_configurations = OverrideConfigurations {
        cert: format!("{}/machine.crt", certs_dir),
        key: format!("{}/machine.key", certs_dir),
        ca: format!("{}/ca.crt", certs_dir),
        networking_firewall_rules,
    };

    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "overide configurations are {:?}",
        overide_configurations
    );
    match generate_nebula_configuartion_file(&provider_config.base_config, overide_configurations)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while generating nebula config file - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaConfigFileGenerateError,
                format!("unable to nebula config file - {}", e.to_string()),
                true
            ))
        }
    }

    let binary_path = format!("{}", temp_dir().display());
    let config_path = format!("{}", temp_dir().display());

    let nebula_child_process = match start_nebula(&binary_path, &config_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while starting nebula on config path - {}, error - {}",
                &config_path,
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaStartError,
                format!("nebula start error - {}", e.to_string()),
                true
            ))
        }
    };

    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "nebula started successfully"
    );
    Ok(StartRes {
        nebula_process: nebula_child_process,
    })
}

pub fn check_existing_certs_valid(ca_path: &str, cert_path: &str, key_path: &str) -> Result<bool> {
    let fn_name = "check_existing_certs_valid";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "ca path is {}, cert path is {}, key path is {}",
        ca_path,
        cert_path,
        key_path
    );

    let cert_exists = Path::new(&cert_path).is_file();

    if !cert_exists {
        error!(
            func = fn_name,
            package = PACKAGE_NAME,
            "cert not found in - {}",
            cert_path
        );
        bail!(NetworkingError::new(
            NetworkingErrorCodes::CertNotFoundError,
            format!("cert not found in - {}", cert_path),
            false
        ))
    };

    let key_exists = Path::new(&key_path).is_file();

    if !key_exists {
        error!(
            func = fn_name,
            package = PACKAGE_NAME,
            "key not found in - {}",
            key_path
        );
        bail!(NetworkingError::new(
            NetworkingErrorCodes::KeyNotFoundError,
            format!("key not found in - {}", key_path),
            false
        ))
    }

    let is_cert_valid = match is_cert_valid(&cert_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while checking cert validity, cert_path - {}, error - {}",
                &cert_path,
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::CertValidityCheckError,
                format!("cert validity check failed failed {}", e.to_string()),
                false
            ))
        }
    };

    if !is_cert_valid {
        error!(func = fn_name, package = PACKAGE_NAME, "cert expired error");
        bail!(NetworkingError::new(
            NetworkingErrorCodes::CertExpiredError,
            format!("cert expired error",),
            true
        ))
    };

    match is_cert_verified(&ca_path, &cert_path) {
        Ok(verified) => {
            if !verified {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "cert not verified error"
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CertVerifyError,
                    format!("cert not verified error",),
                    true
                ))
            }
        }
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while checking cert verification, cert_path - {}, error - {}",
                &cert_path,
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::CertVerificationCheckError,
                format!("cert verification check failed {}", e.to_string()),
                true
            ))
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "certs are valid and available"
    );
    Ok(true)
}

pub async fn sign_cert(enrollment_url: &str, body: IssueCertReq) -> Result<IssueCertRes> {
    let fn_name = "sign_cert";
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "enrollment url is {}, body is {:?}",
        enrollment_url,
        body
    );

    let client = reqwest::Client::new();
    let sign_cert_res = client
        .post(enrollment_url)
        .json(&body)
        .header("CONTENT_TYPE", "application/json")
        .send()
        .await;
    let response = match sign_cert_res {
        Ok(res) => res,
        Err(e) => match e.status() {
            Some(StatusCode::INTERNAL_SERVER_ERROR) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "sign cert request internal server error - {}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertRequestServerError,
                    format!("sign cert request returned server error - {}", e),
                    true
                ))
            }
            Some(StatusCode::BAD_REQUEST) => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "cert sign request returned bad request - {}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertRequestBadRequestError,
                    format!("sign cert request returned bad request - {}", e),
                    true
                ))
            }
            Some(StatusCode::NOT_FOUND) => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "sign cert request not found - {}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertRequestNotFoundError,
                    format!("sign cert request not found - {}", e),
                    true
                ))
            }
            Some(_) => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "sign cert request returned unknown error - {}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertRequestUnknownError,
                    format!("cert sign request returned unknown error - {}", e),
                    true
                ))
            }
            None => {
                error!(
                    func = "lookup_manifest",
                    package = PACKAGE_NAME,
                    "cert sign request returned unknown error - {}",
                    e
                );
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertRequestUnknownError,
                    format!("sign cert request returned unmatched error - {}", e),
                    true
                ))
            }
        },
    };

    // parse the manifest lookup result
    let issue_cert_response = match response
        .json::<NetworkingServerResponseGeneric<IssueCertRes>>()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while parsing sign cert response - {}",
                e.to_string()
            );
            bail!(e);
        }
    };
    Ok(issue_cert_response.payload)
}

fn parse_message_payload<T>(payload: messaging::Bytes) -> Result<T>
where
    T: DeserializeOwned,
{
    let payload_value = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(e) => {
            error!(
                fn_name = "parse_message_payload",
                package = PACKAGE_NAME,
                "Error converting payload to string - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ExtractMessagePayloadError,
                format!("error converting payload to string - {}", e),
                true
            ))
        }
    };
    let payload: T = match serde_json::from_str(payload_value) {
        Ok(s) => s,
        Err(e) => {
            error!(
                fn_name = "parse_message_payload",
                package = PACKAGE_NAME,
                "Error serializing payload - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ExtractMessagePayloadError,
                format!("error error serializing payload - {}", e),
                true
            ))
        }
    };
    Ok(payload)
}

pub async fn get_settings_by_key(
    settings_tx: Sender<SettingMessage>,
    key: String,
) -> Result<String> {
    let fn_name = "get_settings_by_key";
    let (tx, rx) = oneshot::channel();
    match settings_tx
        .clone()
        .send(SettingMessage::GetSettingsByKey {
            reply_to: tx,
            key: key.clone(),
        })
        .await
    {
        Ok(_) => (),
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while sending message to settings channel - {}",
                e.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!(
                    "error while sending message to settings channel - {}",
                    e.to_string()
                ),
                true
            ))
        }
    }
    let settings = match recv_with_timeout(rx).await {
        Ok(settings) => settings,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error receiving settings message - {:?}",
                err
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelReceiveMessageError,
                format!("error receiving settings message - {:?}", err),
                true
            ))
        }
    };

    Ok(settings)
}

async fn get_machine_id(identity_tx: Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();

    match identity_tx
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "unable to send get machine id request error - {:?}",
                err.to_string()
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelSendMessageError,
                format!(
                    "unable to receive get machine id, error - {}",
                    err.to_string()
                ),
                true
            ))
        }
    }

    let machine_id = match recv_with_timeout(rx).await {
        Ok(r) => r,
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "unable to receive machine id, error - {}",
                e
            );
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ChannelReceiveMessageError,
                format!("unable to receive machine ID, error - {}", e.to_string()),
                true
            ))
        }
    };
    Ok(machine_id)
}
