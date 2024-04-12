use std::time::Duration;

use crate::{
    pages::machine_info::InputMessage as Message,
    server::provision_client::{PingResponse, ProvisionManagerClient},
};
use anyhow::{bail, Result};
use relm4::Sender;
use tokio::{select, time};

// pub async fn machine_ping_status_service(sender: Sender<Message>) {
//     let mut interval = time::interval(Duration::from_secs(5));

//     loop {
//         select! {
//             _ = interval.tick() => {
//                     match get_status().await {
//                         Ok(response) => {
//                             let _ = sender.send(Message::ShowStatus(response.code == "success"));
//                         },
//                         Err(e) => {
//                             tracing::error!(
//                                 func = "Machine Info Screen -> active status -> get_status",
//                                 package = env!("CARGO_PKG_NAME"),
//                                 "API Error {:?}",
//                                 e
//                             );
//                             let _ = sender.send(Message::ShowStatus(false));
//                         },
//                     }
//             }
//         }
//     }
// }

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
