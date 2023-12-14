use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use grpc_server::GrpcServerOptions;
use heartbeat::handler::{HeartbeatHandler, HeartbeatMessage, HeartbeatOptions};
use identity::handler::{IdentityHandler, IdentityMessage, IdentityOptions};
use init_tracing_opentelemetry::tracing_subscriber_ext::{
    build_logger_text, build_loglevel_filter_layer, build_otel_layer,
};
use messaging::handler::{MessagingHandler, MessagingMessage, MessagingOptions};
use networking::handler::{NetworkingHandler, NetworkingMessage, NetworkingOptions};
use provisioning::handler::{ProvisioningHandler, ProvisioningMessage, ProvisioningOptions};
use sentry_tracing::EventFilter;
use settings::handler::{SettingHandler, SettingMessage, SettingOptions};
use std::path::Path;
use telemetry::handler::{TelemetryHandler, TelemetryMessage, TelemetryOptions};
use tokio::{
    sync::{broadcast, mpsc},
    task,
};
use tracing_appender::non_blocking;
use tracing_appender::rolling::never;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::EnvFilter;
const CHANNEL_SIZE: usize = 32;
#[tokio::main]
async fn main() -> Result<()> {
    // Setting tracing
    let settings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };
    // enable the sentry exception reporting if enabled in settings and a DSN path is specified
    if settings.clone().sentry.enabled && settings.clone().sentry.dsn.is_some() {
        let sentry_path = settings.clone().sentry.dsn.unwrap();

        let _guard = sentry::init((
            sentry_path,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                trim_backtraces: true,
                ..Default::default()
            },
        ));
    }

    let path = Path::new(settings.logging.path.as_str());
    let directory = path.parent().unwrap();
    let file_name = path.file_name().unwrap();
    let file_appender = never(directory, file_name);
    let (non_blocking_writer, _guard) = non_blocking(file_appender);
    // Set optional layer for logging to a file
    let layer = if settings.logging.enabled && !settings.logging.path.is_empty() {
        Some(
            Layer::new()
                .with_writer(non_blocking_writer)
                .with_ansi(false),
        )
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry()
        .with(layer)
        .with(EnvFilter::new(settings.logging.level.as_str()))
        .with(sentry_tracing::layer().event_filter(|_| EventFilter::Ignore))
        .with(build_loglevel_filter_layer()) //temp for terminal log
        .with(build_logger_text()) //temp for terminal log
        .with(build_otel_layer().unwrap()); // trace collection layer

    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => (),
        Err(e) => bail!(e),
    };

    tracing::info!(
        //sample log
        task = "tracing_setup",
        result = "success",
        "tracing set up",
    );
    let _ = init_services().await;
    Ok(())
}

async fn init_services() -> Result<bool> {
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
async fn init_telemetry_service(
    opt: TelemetryOptions,
) -> (task::JoinHandle<()>, mpsc::Sender<TelemetryMessage>) {
    let (telemetry_tx, telemetry_rx) = mpsc::channel(CHANNEL_SIZE);

    let telemetry_t = tokio::spawn(async move {
        TelemetryHandler::new(opt).run(telemetry_rx).await;
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
