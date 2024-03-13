use std::env;

use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use channel::recv_with_timeout;
use chrono::Duration;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha256::digest;
use sys_info::hostname;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, trace, warn};
const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
use crate::errors::{StatusError, StatusErrorCodes};
#[derive(Serialize, Deserialize, Debug)]
pub struct StatusPublishPayload {
    pub time: String,
    pub machine_id: String,
    pub sys_uptime: String,
    pub sys_load_avg: String,
}

pub struct SendStatusOptions {
    pub messaging_tx: Sender<MessagingMessage>,
    pub identity_tx: Sender<IdentityMessage>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PlatformInfo {
    pub machine_id: String,
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub os_build: String,
    pub platform: String,
    pub platform_release: String,
    pub arch: String,
    pub agent_version: String,
    pub agent_name: String,
}
pub fn get_time_interval() -> u64 {
    let settings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(err) => {
            warn!(
                func = "get_time_interval",
                package = PACKAGE_NAME,
                "failed to get machine id: {:?}",
                err
            );
            AgentSettings::default()
        }
    };
    debug!(
        func = "get_time_interval",
        package = PACKAGE_NAME,
        "time_interval_sec: {}",
        settings.status.time_interval_sec
    );
    settings.status.time_interval_sec
}
pub async fn send_status(status_options: SendStatusOptions) -> Result<bool> {
    let fn_name = "send_status";
    // Get machine id
    let (publish_result_tx, publish_result_rx) = tokio::sync::oneshot::channel();
    let machine_id = match get_machine_id(status_options.identity_tx.clone()).await {
        Ok(machine_id) => machine_id,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting machine id - {}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::FetchMachineIdError,
                format!("error getting machine id - {}", err),
                false
            ));
        }
    };

    // Construct payload
    let current_utc_time = chrono::Utc::now();
    let formatted_utc_time = current_utc_time.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
    trace!(
        func = fn_name,
        package = PACKAGE_NAME,
        "formatted utc time - {}",
        formatted_utc_time
    );
    let uptime = match uptime_lib::get() {
        Ok(uptime) => uptime,
        Err(err) => {
            bail!(StatusError::new(
                StatusErrorCodes::FetchUptimeError,
                format!("error getting uptime - {}", err),
                false
            ));
        }
    };
    //calculate duration
    let system_uptime_duration = Duration::seconds(uptime.as_secs_f64() as i64);
    let load_avg = match sys_info::loadavg() {
        Ok(load_avg) => format!(
            "1m:{} 5m:{} 15m:{}",
            load_avg.one, load_avg.five, load_avg.fifteen
        ),
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting load average - {}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::FetchLoadAverageError,
                format!("error getting load average - {}", err),
                false
            ));
        }
    };

    let publish_payload = StatusPublishPayload {
        time: formatted_utc_time,
        machine_id: machine_id.clone(),
        sys_uptime: system_uptime_duration.num_seconds().to_string(),
        sys_load_avg: load_avg,
    };

    // Publish message
    let send_output = status_options
        .messaging_tx
        .send(MessagingMessage::Send {
            reply_to: publish_result_tx,
            message: json!(publish_payload).to_string(),
            subject: format!("machine.{}.status.heartbeat", digest(machine_id.clone())),
        })
        .await;
    match send_output {
        Ok(_) => (),
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error send status message {:?}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::ChannelSendMessageError,
                "error send status message".to_string(),
                false
            ));
        }
    }
    match recv_with_timeout(publish_result_rx).await {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = "provision_by_code",
                package = PACKAGE_NAME,
                "error publishing status - {}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::ChannelRecvTimeoutError,
                format!("error receiving message: {}", err),
                false
            ));
        }
    }
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "status message published!"
    );
    Ok(true)
}
pub async fn machine_platform_info(
    identity_tx: Sender<IdentityMessage>,
    messaging_tx: Sender<MessagingMessage>,
) -> Result<()> {
    let fn_name = "machine_platform_info";
    //construct machine info

    let hostname = match hostname() {
        Ok(hostname) => hostname,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting hostname - {}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::FetchPlatformInfoError,
                format!("error getting hostname - {}", err),
                false
            ));
        }
    };
    let platform_release = match sys_info::os_release() {
        Ok(release) => release,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting platform release - {}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::FetchPlatformInfoError,
                format!("error getting platform release - {}", err),
                false
            ));
        }
    };
    let machine_id = match get_machine_id(identity_tx).await {
        Ok(machine_id) => machine_id,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting machine id - {}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::FetchPlatformInfoError,
                format!("error getting machine id - {}", err),
                false
            ));
        }
    };
    let platform_info = PlatformInfo {
        machine_id: machine_id.clone(),
        hostname: hostname,
        os_name: std::env::consts::OS.to_string(),
        os_version: sys_info::os_release().unwrap(),
        platform_release: platform_release,
        arch: std::env::consts::ARCH.to_string(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        agent_name: String::from("mecha-agent"),
        ..Default::default()
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    match messaging_tx
        .send(MessagingMessage::Send {
            reply_to: tx,
            message: json!(platform_info).to_string(),
            subject: format!("machine.{}.status.info", digest(machine_id.clone())),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error send machine platform info message to messaging service - {}",
                err
            );
        }
    };
    match recv_with_timeout(rx).await {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error receive machine platform info from messaging service - {}",
                err
            );
        }
    }
    Ok(())
}
pub async fn device_provision_status(identity_tx: Sender<IdentityMessage>) -> bool {
    let fn_name = "device_provision_status";
    let (tx, rx) = tokio::sync::oneshot::channel();
    match identity_tx
        .send(IdentityMessage::GetProvisionStatus { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error send provision status message to identity service - {}",
                err
            );
        }
    };

    match recv_with_timeout(rx).await {
        Ok(provision_status) => provision_status,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error receive provision status from identity service - {}",
                err
            );
            false
        }
    }
}

pub async fn get_machine_id(identity_tx: Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    match identity_tx
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error send get machine id message to identity service - {}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::ChannelSendMessageError,
                format!(
                    "error send get machine id message to identity service - {}",
                    err
                ),
                false
            ));
        }
    }
    match recv_with_timeout(rx).await {
        Ok(machine_id) => Ok(machine_id),
        Err(err) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error receive machine id from identity service - {}",
                err
            );
            bail!(err);
        }
    }
}
