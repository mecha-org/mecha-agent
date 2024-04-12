use crate::server::identity_client::{GetProvisionStatusResponse, IdentityClient};
use anyhow::{bail, Result};

pub async fn machine_provision_status() -> Result<GetProvisionStatusResponse> {
    let mut service_client = match IdentityClient::new().await {
        Ok(r) => r,
        Err(e) => {
            bail!(e);
        }
    };

    let response: GetProvisionStatusResponse =
        match service_client.get_machine_provision_status().await {
            Ok(response) => response.into(),
            Err(e) => {
                bail!(e);
            }
        };

    Ok(response)
}
