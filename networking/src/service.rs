use crate::errors::NetworkingError;
use crate::errors::NetworkingErrorCodes;
use crate::nebula::generate_nebula_key_cert;
use crate::nebula::is_cert_valid;
use crate::nebula::is_cert_verifed;
use crate::nebula::start_nebula;
use crate::nebula::FirewallRule;
use crate::nebula::NebulaSettings;
use crate::utils::extract_tar_file;
use crate::utils::extract_zip_file;
use crate::utils::is_sudo;
use crate::utils::sha256_file;
use anyhow::{bail, Result};
use crypto::base64::b64_decode;
use identity::handler::IdentityMessage;
use ipaddress::IPAddress;
use messaging::handler::MessagingMessage;
use messaging::Bytes;
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
use std::str;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tracing::{debug, error, info};
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

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

pub async fn get_provider_info(
    settings_tx: Sender<SettingMessage>,
) -> Result<ProviderMetadataPayload> {
    let trace_id = find_current_trace_id();
    let task = "get_provider_info";
    let target = "networking";
    let (tx, rx) = oneshot::channel();
    let _ = settings_tx
        .clone()
        .send(SettingMessage::GetSettingsByKey {
            reply_to: tx,
            key: String::from("networking.provider.name"),
        })
        .await;
    let mut app_name = String::new();
    match rx.await {
        Ok(settings_result) => {
            if settings_result.is_ok() {
                match settings_result {
                    Ok(settings_value) => {
                        println!("get settings result {}", settings_value);
                        app_name = settings_value;
                    }
                    Err(_) => {
                        println!("Error getting machine ID");
                    }
                }
            } else {
                println!("Error getting machine ID: {:?}", settings_result.err());
            }
        }
        Err(_) => {
            println!("Error receiving machine ID {:?}", task);
        }
    }

    let arch = std::env::consts::ARCH.to_lowercase();

    if arch.is_empty() {
        bail!(NetworkingError::new(
            NetworkingErrorCodes::SystemArchNotFoundError,
            format!("arch not found",),
            true
        ))
    };

    let os = std::env::consts::OS.to_lowercase();

    if os.is_empty() {
        bail!(NetworkingError::new(
            NetworkingErrorCodes::SystemOsNotFoundError,
            format!("os name not found",),
            true
        ))
    };

    let provider_metadata_payload = ProviderMetadataPayload { app_name, os, arch };

    debug!(
        task,
        target, trace_id, "provider config in device settings is {:?}", provider_metadata_payload
    );

    Ok(provider_metadata_payload)
}

pub async fn get_provider_configs(
    messaging_tx: Sender<MessagingMessage>,
    topic_to_publish: &str,
    provider_metadata_payload: &ProviderMetadataPayload,
) -> Result<ProviderMetadataReply> {
    let trace_id = find_current_trace_id();
    let task = "get_provider_configs";
    let target = "networking";
    let (tx, rx) = oneshot::channel();
    let payload_payload_json = json!(provider_metadata_payload);
    let _ = messaging_tx
        .clone()
        .send(MessagingMessage::Request {
            subject: topic_to_publish.to_string(),
            message: payload_payload_json.to_string(),
            reply_to: tx,
        })
        .await;
    debug!(
        task,
        target, trace_id, "topic is, payload is {} {}", topic_to_publish, payload_payload_json
    );
    let mut response_bytes = Bytes::new();
    match rx.await {
        Ok(publish_response) => {
            if publish_response.is_ok() {
                match publish_response {
                    Ok(result) => response_bytes = result,
                    Err(_) => {
                        println!("Error getting machine ID");
                    }
                }
            } else {
                println!(
                    "Error getting machine ID: {:?}",
                    publish_response.err().unwrap()
                );
            }
        }
        Err(_) => {
            println!("Error receiving machine ID");
        }
    };
    //todo: Confirm with akshay
    let provider_metadata_reply: ProviderMetadataReply = match parse_message_payload(response_bytes)
    {
        Ok(r) => r,
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::MessagingRequestProviderConfigError,
                format!("unable to parse provider config, error - {}", e.to_string()),
                true
            ))
        }
    };

    Ok(provider_metadata_reply)
}

pub async fn extract_provider_package(
    provider_dir: &str,
    provider_config: &ProviderMetadataReply,
) -> Result<bool> {
    let trace_id = find_current_trace_id();
    let task = "extract_provider_package";
    let target = "networking";

    info!(task, target, trace_id, "init");

    debug!(
        task,
        target, trace_id, "checking if provider directory {} exists", provider_dir
    );
    let provider_dir_exists = Path::new(&provider_dir).is_dir();

    debug!(
        task,
        target, trace_id, "provider directory exists is {}", provider_dir_exists
    );

    if !provider_dir_exists {
        match create_dir_all(&provider_dir) {
            Ok(_) => {
                info!(task, target, trace_id, "provider directory created");
            }
            Err(e) => {
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderDirectoryCreateError,
                    format!("provider directory create error - {}", e.to_string()),
                    true
                ))
            }
        };
    }

    let package_file_path = format!("{}/{}", provider_dir, provider_config.file_name);
    debug!(
        task,
        target, trace_id, "checking if package {} exists", package_file_path
    );
    let package_exists = Path::new(&package_file_path).is_file();

    info!(
        task,
        target, trace_id, "package exists is {}", package_exists
    );

    let mut package_checksum_mismatch = false;

    if package_exists {
        let package_checksum = match sha256_file(&package_file_path) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    task,
                    target, trace_id, "error while calculating sha256 of package {}", e
                );
                String::from("")
            }
        };

        debug!(
            task,
            target,
            trace_id,
            "package checksum is {} and package checksum calculated is {}",
            provider_config.checksum,
            package_checksum
        );

        package_checksum_mismatch = package_checksum.ne(&provider_config.checksum);
    }

    debug!(
        task,
        target, trace_id, "package checksum mismatch is {}", package_checksum_mismatch
    );

    //Download provider file using download url
    if !package_exists || package_checksum_mismatch {
        info!(
            task,
            target, trace_id, "downloading package from {}", provider_config.download_url
        );
        let response = match reqwest::get(&provider_config.download_url).await {
            Ok(r) => r,
            Err(e) => {
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
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderFileWriteError,
                    format!("error while writing package file - {}", e.to_string()),
                    true
                ))
            }
        };
        info!(
            task,
            target, trace_id, "provider package downloaded, saved at {}", package_file_path
        );
    }

    info!(task, target, trace_id = trace_id, "extracting package");

    //Extract provider package in temp
    let extract_to_path: PathBuf = temp_dir();

    let package_extracted = match provider_config.file_type.as_str() {
        "zip" => match extract_zip_file(&package_file_path, &extract_to_path).await {
            Ok(r) => {
                info!(task, target, trace_id = trace_id, "extracted package");
                r
            }
            Err(e) => {
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderFileExtractError,
                    format!("error while extracting package file - {}", e.to_string()),
                    true
                ))
            }
        },
        "tar.gz" => match extract_tar_file(&package_file_path, &extract_to_path).await {
            Ok(r) => {
                info!(task, target, trace_id = trace_id, "extracted package");
                r
            }
            Err(e) => {
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ProviderFileExtractError,
                    format!("error while extracting package file - {}", e.to_string()),
                    true
                ))
            }
        },
        _ => bail!(NetworkingError::new(
            NetworkingErrorCodes::InvalidProviderFileType,
            format!("error while extracting package file - provider file type invalid"),
            true
        )),
    };

    Ok(package_extracted)
}

pub async fn validate_or_create_certs(
    settings_tx: Sender<SettingMessage>,
    certs_dir: &str,
    provider_config: &ProviderMetadataReply,
) -> Result<bool> {
    let trace_id = find_current_trace_id();
    let task = "validate_or_create_certs";
    let target = "networking";

    info!(task, target, trace_id, "init");

    debug!(
        task,
        target, trace_id, "checking if certs directory {} exists", certs_dir
    );

    let certs_dir_exists = Path::new(&certs_dir).is_dir();

    debug!(
        task,
        target, trace_id, "certs directory exists is {}", certs_dir_exists
    );

    if !certs_dir_exists {
        match create_dir_all(&certs_dir) {
            Ok(_) => {
                info!(task, target, trace_id, "certs directory created");
            }
            Err(e) => {
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

    let are_certs_valid = match check_existig_certs_valid(&ca_path, &signed_cert_path, &key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                task,
                target, trace_id, "error while checking existing certs {}", e
            );
            false
        }
    };
    info!(
        task,
        target, trace_id, "existing certs valid is {}", are_certs_valid
    );

    if !are_certs_valid {
        //Enrollment process
        match generate_nebula_key_cert(&unsigned_cert_path, &key_path) {
            Ok(_) => {
                info!(task, target, trace_id, "certs generated successfully");
            }
            Err(e) => {
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
            task,
            target, trace_id, "sign cert req is {:?}", sign_cert_req
        );

        let sign_cert_res = match sign_cert(&enrollment_url, sign_cert_req).await {
            Ok(r) => r,
            Err(e) => {
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::SignCertError,
                    format!("error in signing cert - {}", e.to_string()),
                    true
                ))
            }
        };

        info!(task, target, trace_id, "cert signed successfully");

        debug!(
            task,
            target, trace_id, "sign cert response is {:?}", sign_cert_res
        );

        let decoded_cert_bytes = match b64_decode(&sign_cert_res.cert) {
            Ok(v) => v,
            Err(e) => {
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

        debug!(task, target, trace_id, "sign cert decoded successfully ");

        let mut file = match File::create(format!("{}/machine.crt", certs_dir)) {
            Ok(v) => v,
            Err(e) => {
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

        debug!(task, target, trace_id, "ca cert decoded successfully ");

        let mut ca_cert_file = match File::create(format!("{}/ca.crt", certs_dir)) {
            Ok(v) => v,
            Err(e) => {
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
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::CaCertFileSaveError,
                    format!("error while saving ca cert file- {}", e.to_string()),
                    true
                ))
            }
        };

        info!(task, target, trace_id, "certs created successfully");
    }

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
    let trace_id = find_current_trace_id();
    let task = "generate_nebula_configuartion_file";
    let target = "networking";

    info!(task, target, trace_id, "init");

    debug!(task, target, trace_id, "decoding base config");
    let decoded_bytes = match b64_decode(encoded_base_config) {
        Ok(v) => v,
        Err(e) => {
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
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaBaseConfigParseError,
                format!("failed to convert bytes to string {}", e.to_string()),
                true
            ))
        }
    };

    info!(task, target, trace_id, "base config decoded successfully");
    // Deserialize the string into the NebulaSettings struct
    let mut nebula_settings: NebulaSettings = match serde_yaml::from_str(decoded_str) {
        Ok(v) => v,
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaConfigDeSerializeError,
                format!("failed to deserialize {}", e.to_string()),
                true
            ))
        }
    };

    debug!(
        task,
        target, trace_id, "base nebula settings is {:?}", nebula_settings
    );

    nebula_settings.pki.cert = overide_configurations.cert.to_string();
    nebula_settings.pki.key = overide_configurations.key.to_string();
    nebula_settings.pki.ca = overide_configurations.ca.to_string();

    let (inbound_firewall_roles, outbound_firewall_roles) =
        get_firewall_rules(&overide_configurations.networking_firewall_rules);

    nebula_settings.firewall.inbound = inbound_firewall_roles;
    nebula_settings.firewall.outbound = outbound_firewall_roles;

    info!(
        task,
        target, trace_id, "nebula settings overrided and crated successfully"
    );

    // Serialize NebulaSettings into a YAML-formatted string
    let yaml_string = match serde_yaml::to_string(&nebula_settings) {
        Ok(v) => v,
        Err(e) => {
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
    let mut file = match File::create(temp_dir) {
        Ok(v) => v,
        Err(e) => {
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
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaConfigFileCreateError,
                format!("failed to save nebula config file {}", e.to_string()),
                true
            ))
        }
    }

    info!(task, target, trace_id, "nebula config file crated ");

    info!(
        task,
        target, trace_id, "nebula config file crated successfully",
    );
    Ok(true)
}

pub async fn get_networking_firewall_rules(
    settings_tx: Sender<SettingMessage>,
) -> Result<Vec<NetworkingFirewallRules>> {
    let trace_id = find_current_trace_id();
    let task = "start";
    let target = "get_networking_firewall_rules";

    info!(task, target, trace_id,);
    let networking_firewall_rules_str =
        match get_settings_by_key(settings_tx, String::from("networking.firewall.rules")).await {
            Ok(v) => v,
            Err(e) => {
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
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::ExtractNetworkingFirewallRulesPayloadError,
                    format!(
                        "Error converting payload to NetworkingFirewallRules - {}",
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
) -> Result<bool> {
    let trace_id = find_current_trace_id();
    let task = "start";
    let target = "networking_service_start";

    info!(task, target, trace_id, "starting netwoking service",);

    //Get provider info from settings
    let provider_metadata_payload = match get_provider_info(setting_tx.clone()).await {
        Ok(r) => r,
        Err(e) => {
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
        task,
        target, trace_id, "provider metadata payload is {:?}", provider_metadata_payload
    );

    // Get machine id
    let machine_id = match get_machine_id(identity_tx).await {
        Ok(r) => r,
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::MachineSettingsMachineIdNotFoundError,
                format!("unable to get machine id, error - {}", e.to_string()),
                true
            ))
        }
    };
    debug!(task, target, trace_id, "machine id is {}", machine_id);

    //Get provider config
    let topic_to_publish = format!(
        "machine.{}.networking.provider.metadata",
        digest(machine_id.to_string())
    );
    let provider_config: ProviderMetadataReply =
        match get_provider_configs(messaging_tx, &topic_to_publish, &provider_metadata_payload)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                bail!(NetworkingError::new(
                    NetworkingErrorCodes::GetProviderConfigsError,
                    format!("unable to get provider configs, error - {}", e.to_string()),
                    true
                ))
            }
        };

    debug!(
        task,
        target, trace_id, "provider config is {:?}", provider_config
    );

    //Save provider package binaries in temp
    let home_dir = std::env::var("HOME").unwrap();
    let provider_dir = format!("{}/.mecha/networking/{}", home_dir, provider_config.name);
    match extract_provider_package(&provider_dir, &provider_config).await {
        Ok(_) => {
            debug!(
                task,
                target, trace_id, "provider package binaries extracted successfully"
            );
        }
        Err(e) => {
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

    info!(
        task,
        target,
        trace_id = trace_id,
        "provider package binaries saved successfully",
    );

    let certs_dir = format!("{}/certs", provider_dir);
    match validate_or_create_certs(setting_tx.clone(), &certs_dir, &provider_config).await {
        Ok(_) => {
            debug!(
                task,
                target, trace_id, "certs validated or created successfully"
            );
        }
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
    info!(task, target, trace_id = trace_id, "certs are available",);

    info!(
        task,
        target,
        trace_id = trace_id,
        "checking sudo permissions",
    );

    if !is_sudo() {
        bail!(NetworkingError::new(
            NetworkingErrorCodes::SudoCheckFailed,
            format!("sudo check failed",),
            true
        ))
    }

    info!(
        task = "start",
        target = "networking_service_start",
        trace_id = trace_id,
        "thread have sudo permissions",
    );

    let networking_firewall_rules = match get_networking_firewall_rules(setting_tx.clone()).await {
        Ok(v) => v,
        Err(_) => {
            vec![]
        }
    };

    let overide_configurations = OverrideConfigurations {
        cert: format!("{}/machine.crt", certs_dir),
        key: format!("{}/machine.key", certs_dir),
        ca: format!("{}/ca.crt", certs_dir),
        networking_firewall_rules,
    };

    match generate_nebula_configuartion_file(&provider_config.base_config, overide_configurations)
        .await
    {
        Ok(_) => {
            debug!(
                task,
                target, trace_id, "nebula config file created successfully"
            );
        }
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaConfigFileGenerateError,
                format!("unable to nebula config file - {}", e.to_string()),
                true
            ))
        }
    }

    info!(task, target, trace_id, "starting nebula");

    let binary_path = format!("{}", temp_dir().display());
    let config_path = format!("{}", temp_dir().display());

    match start_nebula(&binary_path, &config_path) {
        Ok(_) => (),
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::NebulaStartError,
                format!("nebula start error - {}", e.to_string()),
                true
            ))
        }
    };

    info!(
        task,
        target, trace_id, "networking service started successfully"
    );

    Ok(true)
}

pub fn check_existig_certs_valid(ca_path: &str, cert_path: &str, key_path: &str) -> Result<bool> {
    let cert_exists = Path::new(&cert_path).is_file();

    if !cert_exists {
        bail!(NetworkingError::new(
            NetworkingErrorCodes::CertNotFoundError,
            format!("cert not found in - {}", cert_path),
            true
        ))
    };

    let key_exists = Path::new(&key_path).is_file();

    if !key_exists {
        bail!(NetworkingError::new(
            NetworkingErrorCodes::KeyNotFoundError,
            format!("key not found in - {}", key_path),
            true
        ))
    }

    let is_cert_valid = match is_cert_valid(&cert_path) {
        Ok(v) => v,
        Err(e) => bail!(NetworkingError::new(
            NetworkingErrorCodes::CertValidityCheckError,
            format!("cert validity check failed failed {}", e.to_string()),
            true
        )),
    };

    if !is_cert_valid {
        bail!(NetworkingError::new(
            NetworkingErrorCodes::CertExpiredError,
            format!("cert expired error",),
            true
        ))
    };

    let is_cert_verified = match is_cert_verifed(&ca_path, &cert_path) {
        Ok(v) => v,
        Err(e) => bail!(NetworkingError::new(
            NetworkingErrorCodes::CertVerificationCheckError,
            format!("cert verification check failed {}", e.to_string()),
            true
        )),
    };

    if !is_cert_verified {
        bail!(NetworkingError::new(
            NetworkingErrorCodes::CertVerifyError,
            format!("cert not verified error",),
            true
        ))
    };

    Ok(true)
}

pub async fn sign_cert(enrollment_url: &str, body: IssueCertReq) -> Result<IssueCertRes> {
    let trace_id = find_current_trace_id();
    let task = "start";
    let target = "sign_cert";
    info!(task, target, trace_id, "init");

    let client = reqwest::Client::new();

    debug!(
        task,
        target,
        trace_id,
        "raw request is {} {}",
        enrollment_url,
        json!(&body)
    );
    let sign_cert_req = client
        .post(enrollment_url)
        .json(&body)
        .header("CONTENT_TYPE", "application/json")
        .send()
        .await?;

    let sign_cert_res: String = match sign_cert_req.text().await {
        Ok(csr) => csr,
        Err(e) => {
            bail!(e)
        }
    };

    debug!(
        task,
        target, trace_id, "raw sign cert res {}", sign_cert_res
    );

    let result: NetworkingServerResponseGeneric<IssueCertRes> =
        match serde_json::from_str(&sign_cert_res) {
            Ok(v) => v,
            Err(e) => {
                bail!(e);
            }
        };
    Ok(result.payload)
}

fn parse_message_payload<T>(payload: messaging::Bytes) -> Result<T>
where
    T: DeserializeOwned,
{
    let payload_value = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ExtractMessagePayloadError,
                format!("Error converting payload to string - {}", e),
                true
            ))
        }
    };
    let payload: T = match serde_json::from_str(payload_value) {
        Ok(s) => s,
        Err(e) => {
            bail!(NetworkingError::new(
                NetworkingErrorCodes::ExtractMessagePayloadError,
                format!("Error converting payload to AddTaskRequestPayload - {}", e),
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
    let trace_id = find_current_trace_id();
    let task = "get_settings_by_key";
    let target = "networking";
    let (tx, rx) = oneshot::channel();
    let _ = settings_tx
        .clone()
        .send(SettingMessage::GetSettingsByKey {
            reply_to: tx,
            key: key.clone(),
        })
        .await;
    let mut settings = String::new();
    match rx.await {
        Ok(settings_result) => {
            if settings_result.is_ok() {
                match settings_result {
                    Ok(settings_value) => {
                        println!("get settings result {}", settings_value);
                        settings = settings_value;
                    }
                    Err(_) => {
                        println!("Error getting machine ID");
                    }
                }
            } else {
                println!("Error getting machine ID: {:?}", settings_result.err());
            }
        }
        Err(_) => {
            println!("Error receiving machine ID");
        }
    }
    Ok(settings)
}

async fn get_machine_id(identity_tx: Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    let mut machine_id = String::new();
    let _ = identity_tx
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await;
    match rx.await {
        Ok(machine_id_result) => {
            if machine_id_result.is_ok() {
                match machine_id_result {
                    Ok(machine_id_value) => {
                        println!("Machine ID: {}", machine_id_value);
                        machine_id = machine_id_value;
                    }
                    Err(_) => {
                        println!("Error getting machine ID");
                    }
                }
            } else {
                println!(
                    "Error getting machine ID: {:?}",
                    machine_id_result.err().unwrap()
                );
            }
        }
        Err(_) => {
            println!("Error receiving machine ID");
        }
    }
    Ok(machine_id)
}
