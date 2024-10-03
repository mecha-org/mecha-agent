use crate::errors::{AgentError, AgentErrorCodes};
use agent_settings::AgentSettings;
use anyhow::{bail, Result};
use app_services::handler::{AppServiceHandler, AppServiceMessage, AppServiceOptions};
use grpc_server::GrpcServerOptions;
use identity::handler::{IdentityHandler, IdentityMessage, IdentityOptions};
use messaging::handler::{MessagingHandler, MessagingMessage, MessagingOptions};
use networking::handler::{NetworkingHandler, NetworkingMessage, NetworkingOptions};
use provisioning::handler::{ProvisioningHandler, ProvisioningMessage, ProvisioningOptions};
use settings::handler::{SettingHandler, SettingMessage, SettingOptions};
use status::handler::{StatusHandler, StatusMessage, StatusOptions};
use telemetry::handler::{TelemetryHandler, TelemetryMessage, TelemetryOptions};
use tokio::{
    sync::{broadcast, mpsc},
    task,
};
use tracing::error;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
const CHANNEL_SIZE: usize = 32;

pub async fn init_handlers(settings: AgentSettings, socket_addr: &str) -> Result<bool> {
    let (event_tx, _) = broadcast::channel(CHANNEL_SIZE);
    let (identity_t, identity_tx) = init_identity_service(IdentityOptions {
        event_tx: event_tx.clone(),
        settings: identity::handler::Settings {
            data_dir: settings.data.dir.clone(),
        },
    })
    .await;

    let (messaging_t, messaging_tx) = init_messaging_service(MessagingOptions {
        settings: messaging::handler::Settings {
            data_dir: settings.data.dir.clone(),
            service_url: settings.backend.service.clone(),
            messaging_url: settings.backend.messaging.clone(),
        },
        event_tx: event_tx.clone(),
        identity_tx: identity_tx.clone(),
    })
    .await;

    let (prov_t, prov_tx) = init_provisioning_service(ProvisioningOptions {
        settings: provisioning::handler::Settings {
            service_url: settings.backend.service.clone(),
            data_dir: settings.data.dir.clone(),
        },
        identity_tx: identity_tx.clone(),
        messaging_tx: messaging_tx.clone(),
        event_tx: event_tx.clone(),
    })
    .await;

    let (status_t, _status_tx) = init_status_service(StatusOptions {
        settings: status::handler::Settings {
            is_enabled: settings.status.enabled,
            interval: settings.status.interval,
        },
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
    let (networking_t, _networking_tx) = init_networking_service(NetworkingOptions {
        settings: settings.networking,
        event_tx: event_tx.clone(),
        messaging_tx: messaging_tx.clone(),
        identity_tx: identity_tx.clone(),
        setting_tx: setting_tx.clone(),
    })
    .await;

    let (telemetry_t, telemetry_tx) = init_telemetry_service(TelemetryOptions {
        settings: telemetry::handler::Settings {
            is_enabled: settings.telemetry.enabled,
            otlp_addr: socket_addr.to_string(),
        },
        event_tx: event_tx.clone(),
        messaging_tx: messaging_tx.clone(),
        identity_tx: identity_tx.clone(),
        settings_tx: setting_tx.clone(),
    })
    .await;

    let (app_service_t, _app_service_tx) = init_app_service(AppServiceOptions {
        event_tx: event_tx.clone(),
        messaging_tx: messaging_tx.clone(),
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
    let _ = identity_t.await.unwrap();
    let _ = messaging_t.await.unwrap();
    let _ = prov_t.await.unwrap();
    let _ = status_t.await.unwrap();
    let _ = setting_t.await.unwrap();
    let _ = networking_t.await.unwrap();
    let _ = telemetry_t.await.unwrap();
    let _ = app_service_t.await.unwrap();
    let _ = grpc_t.await.unwrap();

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
                    format!("error init/run provisioning service: {:?}", e)
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
                    format!("error init/run identity service: {:?}", e)
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
                    format!("error init/run messaging service: {:?}", e)
                ));
            }
        }
        Ok(())
    });

    (messaging_t, messaging_tx)
}
async fn init_status_service(
    opt: StatusOptions,
) -> (task::JoinHandle<Result<()>>, mpsc::Sender<StatusMessage>) {
    let (status_tx, status_rx) = mpsc::channel(CHANNEL_SIZE);

    let status_t = tokio::spawn(async move {
        match StatusHandler::new(opt).run(status_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_status_service",
                    package = PACKAGE_NAME,
                    "error init/run status service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::StatusInitError,
                    format!("error init/run status service: {:?}", e)
                ));
            }
        }
        Ok(())
    });

    (status_t, status_tx)
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
                    format!("error init/run settings service: {:?}", e)
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
                    format!("error init/run networking service: {:?}", e)
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
                    format!("error init/run telemetry service: {:?}", e)
                ));
            }
        }
        Ok(())
    });

    (telemetry_t, telemetry_tx)
}

async fn init_app_service(
    opt: AppServiceOptions,
) -> (
    task::JoinHandle<Result<()>>,
    mpsc::Sender<AppServiceMessage>,
) {
    let (app_service_tx, app_service_rx) = mpsc::channel(CHANNEL_SIZE);

    let app_service_t = tokio::spawn(async move {
        match AppServiceHandler::new(opt).run(app_service_rx).await {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "init_app_service",
                    package = PACKAGE_NAME,
                    "error init/run app service: {:?}",
                    e
                );
                bail!(AgentError::new(
                    AgentErrorCodes::AppServiceInitError,
                    format!("error init/run app service: {:?}", e)
                ));
            }
        }
        Ok(())
    });

    (app_service_t, app_service_tx)
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
