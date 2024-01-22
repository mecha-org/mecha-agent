use crate::errors::{AgentError, AgentErrorCodes};
use anyhow::{bail, Result};
use grpc_server::GrpcServerOptions;
use heartbeat::handler::{HeartbeatHandler, HeartbeatMessage, HeartbeatOptions};
use identity::handler::{IdentityHandler, IdentityMessage, IdentityOptions};
use messaging::handler::{MessagingHandler, MessagingMessage, MessagingOptions};
use networking::handler::{NetworkingHandler, NetworkingMessage, NetworkingOptions};
use provisioning::handler::{ProvisioningHandler, ProvisioningMessage, ProvisioningOptions};
use settings::handler::{SettingHandler, SettingMessage, SettingOptions};
use telemetry::handler::{TelemetryHandler, TelemetryMessage, TelemetryOptions};
use tokio::{
    sync::{broadcast, mpsc},
    task,
};
use tracing::error;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
const CHANNEL_SIZE: usize = 32;

pub async fn init_services() -> Result<bool> {
    let (event_tx, _) = broadcast::channel(CHANNEL_SIZE);
    let (identity_t, identity_tx) = init_identity_service(IdentityOptions {
        event_tx: event_tx.clone(),
    })
    .await;

    let (messaging_t, messaging_tx) = init_messaging_service(MessagingOptions {
        event_tx: event_tx.clone(),
        identity_tx: identity_tx.clone(),
    })
    .await;

    let (prov_t, prov_tx) = init_provisioning_service(ProvisioningOptions {
        identity_tx: identity_tx.clone(),
        messaging_tx: messaging_tx.clone(),
        event_tx: event_tx.clone(),
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

    let (telemetry_t, telemetry_tx) = init_telemetry_service(TelemetryOptions {
        event_tx: event_tx.clone(),
        messaging_tx: messaging_tx.clone(),
        identity_tx: identity_tx.clone(),
    })
    .await;

    let grpc_t = init_grpc_server(
        prov_tx.clone(),
        identity_tx.clone(),
        messaging_tx.clone(),
        setting_tx.clone(),
        telemetry_tx.clone(),
    )
    .await;

    //TODO: remove this
    // let start_t = tokio::task::spawn(async move {
    //     sleep(Duration::from_secs(20)).await;
    //     println!("triggering provisioned");
    //     let _ = event_tx_1.send(Event::Provisioning(events::ProvisioningEvent::Provisioned));
    // });

    // wait on all join handles
    identity_t.await.unwrap();
    messaging_t.await.unwrap();
    prov_t.await.unwrap();
    heartbeat_t.await.unwrap();
    setting_t.await.unwrap();
    networking_t.await.unwrap();
    telemetry_t.await.unwrap();
    grpc_t.await.unwrap();

    Ok(true)
}

async fn init_provisioning_service(
    opt: ProvisioningOptions,
) -> (
    task::JoinHandle<Result<()>>,
    mpsc::Sender<ProvisioningMessage>,
) {
    let (prov_tx, prov_rx) = mpsc::channel(CHANNEL_SIZE);

    let prov_t = tokio::spawn(async move {
        match ProvisioningHandler::new(opt).run(prov_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_provisioning_service",
                    package = PACKAGE_NAME,
                    "error init/run provisioning service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::ProvisioningInitError,
                    format!("error init/run provisioning service: {:?}", e),
                    true
                ));
            }
        };
        Ok(())
    });

    (prov_t, prov_tx)
}

async fn init_identity_service(
    opt: IdentityOptions,
) -> (task::JoinHandle<Result<()>>, mpsc::Sender<IdentityMessage>) {
    let (identity_tx, identity_rx) = mpsc::channel(CHANNEL_SIZE);

    let identity_t = tokio::spawn(async move {
        match IdentityHandler::new(opt).run(identity_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_identity_service",
                    package = PACKAGE_NAME,
                    "error init/run identity service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::IdentityInitError,
                    format!("error init/run identity service: {:?}", e),
                    true
                ));
            }
        }
        Ok(())
    });

    (identity_t, identity_tx)
}

async fn init_messaging_service(
    opt: MessagingOptions,
) -> (task::JoinHandle<Result<()>>, mpsc::Sender<MessagingMessage>) {
    let (messaging_tx, messaging_rx) = mpsc::channel(CHANNEL_SIZE);

    let messaging_t = tokio::spawn(async move {
        match MessagingHandler::new(opt).run(messaging_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_messaging_service",
                    package = PACKAGE_NAME,
                    "error init/run messaging service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::MessagingInitError,
                    format!("error init/run messaging service: {:?}", e),
                    true
                ));
            }
        }
        Ok(())
    });

    (messaging_t, messaging_tx)
}
async fn init_heartbeat_service(
    opt: HeartbeatOptions,
) -> (task::JoinHandle<Result<()>>, mpsc::Sender<HeartbeatMessage>) {
    let (heartbeat_tx, heartbeat_rx) = mpsc::channel(CHANNEL_SIZE);

    let heartbeat_t = tokio::spawn(async move {
        match HeartbeatHandler::new(opt).run(heartbeat_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_heartbeat_service",
                    package = PACKAGE_NAME,
                    "error init/run heartbeat service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::HeartbeatInitError,
                    format!("error init/run heartbeat service: {:?}", e),
                    true
                ));
            }
        }
        Ok(())
    });

    (heartbeat_t, heartbeat_tx)
}
async fn init_setting_service(
    opt: SettingOptions,
) -> (task::JoinHandle<Result<()>>, mpsc::Sender<SettingMessage>) {
    let (setting_tx, setting_rx) = mpsc::channel(CHANNEL_SIZE);

    let setting_t = tokio::spawn(async move {
        match SettingHandler::new(opt).run(setting_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_setting_service",
                    package = PACKAGE_NAME,
                    "error init/run settings service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::SettingsInitError,
                    format!("error init/run settings service: {:?}", e),
                    true
                ));
            }
        }
        Ok(())
    });

    (setting_t, setting_tx)
}

async fn init_networking_service(
    opt: NetworkingOptions,
) -> (
    task::JoinHandle<Result<()>>,
    mpsc::Sender<NetworkingMessage>,
) {
    let (networking_tx, networking_rx) = mpsc::channel(CHANNEL_SIZE);

    let networking_t = tokio::spawn(async move {
        match NetworkingHandler::new(opt).run(networking_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_networking_service",
                    package = PACKAGE_NAME,
                    "error init/run networking service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::NetworkingInitError,
                    format!("error init/run networking service: {:?}", e),
                    true
                ));
            }
        }
        Ok(())
    });

    (networking_t, networking_tx)
}

async fn init_telemetry_service(
    opt: TelemetryOptions,
) -> (task::JoinHandle<Result<()>>, mpsc::Sender<TelemetryMessage>) {
    let (telemetry_tx, telemetry_rx) = mpsc::channel(CHANNEL_SIZE);

    let telemetry_t = tokio::spawn(async move {
        match TelemetryHandler::new(opt).run(telemetry_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_telemetry_service",
                    package = PACKAGE_NAME,
                    "error init/run telemetry service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::TelemetryInitError,
                    format!("error init/run telemetry service: {:?}", e),
                    true
                ));
            }
        }
        Ok(())
    });

    (telemetry_t, telemetry_tx)
}

async fn init_grpc_server(
    provisioning_tx: mpsc::Sender<ProvisioningMessage>,
    identity_tx: mpsc::Sender<IdentityMessage>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    settings_tx: mpsc::Sender<SettingMessage>,
    telemetry_tx: mpsc::Sender<TelemetryMessage>,
) -> task::JoinHandle<()> {
    let grpc_t = tokio::spawn(async move {
        let _ = grpc_server::start_grpc_service(GrpcServerOptions {
            provisioning_tx,
            identity_tx,
            messaging_tx,
            settings_tx,
            telemetry_tx,
        })
        .await;
    });

    grpc_t
}
