use crate::server::provision_client::{PingResponse, ProvisionManagerClient};
use anyhow::{bail, Result};

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
