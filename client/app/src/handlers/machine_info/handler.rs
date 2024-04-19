use std::time::Duration;

use crate::{
    pages::machine_info::InputMessage as Message,
    server::provision_client::{PingResponse, ProvisionManagerClient},
};
use anyhow::{bail, Result};
use relm4::Sender;
use tokio::{select, time};

pub async fn get_status() -> Result<PingResponse> {
    let mut service_client = match ProvisionManagerClient::new().await {
        Ok(r) => r,
        Err(e) => {
            bail!(e);
        }
    };

    let response = match service_client.ping().await {
        Ok(response) => response.into(),
        Err(e) => {
            bail!(e);
        }
    };

    Ok(response)
}
