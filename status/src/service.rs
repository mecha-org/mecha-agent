use std::env;

use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use channel::recv_with_timeout;
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
}
pub struct SendStatusOptions {
    pub messaging_tx: Sender<MessagingMessage>,
    pub identity_tx: Sender<IdentityMessage>,
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
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (publish_result_tx, publish_result_rx) = tokio::sync::oneshot::channel();
    let send_output = status_options
        .identity_tx
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await;

    match send_output {
        Ok(_) => (),
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error send identity message to get machine_id: {:?}",
                err
            );
            bail!(StatusError::new(
                StatusErrorCodes::ChannelSendMessageError,
                format!("error send identity message to get machine_id: {:?}", err),
                false
            ));
        }
    }
    let machine_id = match recv_with_timeout(rx).await {
        Ok(machine_id) => machine_id,
        Err(err) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error receiving message from identity service {:?}",
                err
            );
            bail!(err);
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
    let publish_payload = StatusPublishPayload {
        time: formatted_utc_time,
        machine_id: machine_id.clone(),
    };

    let hostname = hostname().unwrap();
    println!("Platform hostname: {}", hostname);

    let five = sys_info::loadavg().unwrap().one;
    println!("Load average: {}", five);

    println!("Version: release {}", sys_info::os_release().unwrap());
    println!("OS Type: {}", sys_info::os_type().unwrap());

    // Publish message
    let send_output = status_options
        .messaging_tx
        .send(MessagingMessage::Send {
            reply_to: publish_result_tx,
            message: json!(publish_payload).to_string(),
            subject: format!("machine.{}.status", digest(machine_id.clone())),
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
