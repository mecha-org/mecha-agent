use anyhow::Result;
use grpc_server::{set_tracing, GrpcServerOptions};
use heartbeat::handler::{HeartbeatHandler, HeartbeatMessage, HeartbeatOptions};
use identity::handler::{IdentityHandler, IdentityMessage, IdentityOptions};
use messaging::handler::{MessagingHandler, MessagingMessage, MessagingOptions};
use networking::handler::{NetworkingHandler, NetworkingMessage, NetworkingOptions};
use provisioning::handler::{ProvisioningHandler, ProvisioningMessage, ProvisioningOptions};
use settings::handler::{SettingHandler, SettingMessage, SettingOptions};
use std::error::Error;
use tokio::{
    sync::{broadcast, mpsc},
    task,
};
const CHANNEL_SIZE: usize = 32;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = init_services().await;
    Ok(())
}

async fn init_services() -> Result<bool> {
    let _ = set_tracing().await;
    let (event_tx, _) = broadcast::channel(CHANNEL_SIZE);
    // start services
    let (prov_t, prov_tx) = init_provisioning_service(ProvisioningOptions {
        event_tx: event_tx.clone(),
    })
    .await;

    let (identity_t, identity_tx) = init_identity_service(IdentityOptions {
        event_tx: event_tx.clone(),
    })
    .await;

    let (messaging_t, messaging_tx) = init_messaging_service(MessagingOptions {
        event_tx: event_tx.clone(),
        identity_tx: identity_tx.clone(),
    })
    .await;

    let (heartbeat_t, heartbeat_tx) = init_heartbeat_service(HeartbeatOptions {
        event_tx: event_tx.clone(),
        messaging_tx: messaging_tx.clone(),
        identity_tx: identity_tx.clone(),
    })
    .await;

    let (setting_t, setting_tx) = init_setting_service(SettingOptions {
        event_tx: event_tx.clone(),
        messaging_tx: messaging_tx.clone(),
        identity_tx: identity_tx.clone(),
    })
    .await;
    let (networking_t, networking_tx) = init_networking_service(NetworkingOptions {
        event_tx: event_tx.clone(),
        messaging_tx: messaging_tx.clone(),
        identity_tx: identity_tx.clone(),
        setting_tx: setting_tx.clone(),
    })
    .await;

    let grpc_t = init_grpc_server(
        prov_tx.clone(),
        identity_tx.clone(),
        messaging_tx.clone(),
        setting_tx.clone(),
    )
    .await;
    // wait on all join handles
    prov_t.await.unwrap();
    identity_t.await.unwrap();
    messaging_t.await.unwrap();
    heartbeat_t.await.unwrap();
    setting_t.await.unwrap();
    networking_t.await.unwrap();
    grpc_t.await.unwrap();

    Ok(true)
}

async fn init_provisioning_service(
    opt: ProvisioningOptions,
) -> (task::JoinHandle<()>, mpsc::Sender<ProvisioningMessage>) {
    let (prov_tx, prov_rx) = mpsc::channel(CHANNEL_SIZE);

    let prov_t = tokio::spawn(async move {
        ProvisioningHandler::new(opt).run(prov_rx).await;
    });

    (prov_t, prov_tx)
}

async fn init_identity_service(
    opt: IdentityOptions,
) -> (task::JoinHandle<()>, mpsc::Sender<IdentityMessage>) {
    let (identity_tx, identity_rx) = mpsc::channel(CHANNEL_SIZE);

    let identity_t = tokio::spawn(async move {
        IdentityHandler::new(opt).run(identity_rx).await;
    });

    (identity_t, identity_tx)
}

async fn init_messaging_service(
    opt: MessagingOptions,
) -> (task::JoinHandle<()>, mpsc::Sender<MessagingMessage>) {
    let (messaging_tx, messaging_rx) = mpsc::channel(CHANNEL_SIZE);

    let messaging_t = tokio::spawn(async move {
        MessagingHandler::new(opt).run(messaging_rx).await;
    });

    (messaging_t, messaging_tx)
}
async fn init_setting_service(
    opt: SettingOptions,
) -> (task::JoinHandle<()>, mpsc::Sender<SettingMessage>) {
    let (setting_tx, setting_rx) = mpsc::channel(CHANNEL_SIZE);

    let setting_t = tokio::spawn(async move {
        SettingHandler::new(opt).run(setting_rx).await;
    });

    (setting_t, setting_tx)
}

async fn init_heartbeat_service(
    opt: HeartbeatOptions,
) -> (task::JoinHandle<()>, mpsc::Sender<HeartbeatMessage>) {
    let (heartbeat_tx, heartbeat_rx) = mpsc::channel(CHANNEL_SIZE);

    let prov_t = tokio::spawn(async move {
        HeartbeatHandler::new(opt).run(heartbeat_rx).await;
    });

    (prov_t, heartbeat_tx)
}

async fn init_networking_service(
    opt: NetworkingOptions,
) -> (task::JoinHandle<()>, mpsc::Sender<NetworkingMessage>) {
    let (networking_tx, networking_rx) = mpsc::channel(CHANNEL_SIZE);

    let networking_t = tokio::spawn(async move {
        NetworkingHandler::new(opt).run(networking_rx).await;
    });

    (networking_t, networking_tx)
}
async fn init_grpc_server(
    provisioning_tx: mpsc::Sender<ProvisioningMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    settings_tx: mpsc::Sender<SettingMessage>,
) -> task::JoinHandle<()> {
    let grpc_t = tokio::spawn(async move {
        let _ = grpc_server::start_grpc_service(GrpcServerOptions {
            provisioning_tx,
            identity_tx,
            messaging_tx,
            settings_tx,
        })
        .await;
    });

    grpc_t
}
