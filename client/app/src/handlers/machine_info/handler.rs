use std::time::Duration;

use anyhow::{bail, Result};
use relm4::Sender;
use tokio::{select, time};
use crate::{pages::machine_info::InputMessage as Message, server::provision_client::{PingResponse, ProvisionManagerClient}};
  
pub async fn machine_status_service(sender: Sender<Message>) {
    let mut interval = time::interval(Duration::from_secs(5));

    loop { 
        select! {
            _ = interval.tick() => {
                    let check_status = get_status().await;
                    match get_status().await {
                        Ok(response) => {
                            let _ = sender.send(Message::ShowStatus(response.code == "success"));
                        },
                        Err(e) => {
                            eprintln!("{}", e);
                            let _ = sender.send(Message::ShowStatus(false));
                        },
                    }
            }
        }

    }
}
pub async fn get_status() -> Result<PingResponse> {
    let request = ProvisionManagerClient::new().await;

    let mut service_client = match request {
        Ok(r) => r,
        Err(e) => {
            bail!(e);
        }
    };
        
    let response = match service_client.ping().await {
        Ok(response) => {
            response.into()
        },
        Err(e) => {
            bail!(e);
        },
    };

    Ok(response)
}