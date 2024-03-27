use crate::{
    config::init_otlp_configuration,
    metrics::initialize_metrics,
    service::{process_logs, process_metrics, telemetry_init},
};
use anyhow::Result;
use events::Event;
use identity::handler::IdentityMessage;
use messaging::handler::MessagingMessage;
use settings::handler::SettingMessage;
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub struct TelemetryHandler {
    event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
    pub settings_tx: mpsc::Sender<SettingMessage>,
    telemetry_task_token: Option<CancellationToken>,
}
pub struct TelemetryOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
    pub identity_tx: mpsc::Sender<IdentityMessage>,
    pub settings_tx: mpsc::Sender<SettingMessage>,
}

pub enum TelemetryMessage {
    SendLogs {
        logs: Vec<u8>,
        logs_type: String,
        reply_to: oneshot::Sender<Result<bool>>,
    },
    SendMetrics {
        metrics: Vec<u8>,
        metrics_type: String,
        reply_to: oneshot::Sender<Result<bool>>,
    },
}

impl TelemetryHandler {
    pub fn new(options: TelemetryOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            identity_tx: options.identity_tx,
            messaging_tx: options.messaging_tx,
            telemetry_task_token: None,
            settings_tx: options.settings_tx,
        }
    }
    async fn initialize_telemetry(&mut self) -> Result<bool> {
        let fn_name = "initialize_telemetry";
        // safety: check for existing cancel token, and cancel it
        let telemetry_task_token = &self.telemetry_task_token;
        if telemetry_task_token.is_some() {
            let _ = telemetry_task_token.as_ref().unwrap().cancel();
        }
        // create new token
        let telemetry_task_token = CancellationToken::new();
        let telemetry_task_token_cloned = Some(telemetry_task_token.clone());
        let mut telemetry_process = None;
        match telemetry_init() {
            Ok(child) => {
                println!("telemetry process started: {:?}", child);
                telemetry_process = Some(child.telemetry_process);
            }
            Err(e) => {
                error!(
                    func = fn_name,
                    package = env!("CARGO_PKG_NAME"),
                    "failed to start telemetry, error - {}",
                    e
                );
            }
        };
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(10));
        let _: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            loop {
                select! {
                    _ = telemetry_task_token.cancelled() => {
                        info!(
                            func = fn_name,
                            package = env!("CARGO_PKG_NAME"),
                            "telemetry task cancelled"
                        );
                        if telemetry_process.is_some() {
                            match telemetry_process.unwrap().kill().await {
                                Ok(_) => {
                                    info!(
                                        func = fn_name,
                                        package = env!("CARGO_PKG_NAME"),
                                        "telemetry process killed"
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        func = fn_name,
                                        package = env!("CARGO_PKG_NAME"),
                                        "failed to kill telemetry process, error - {}",
                                        e
                                    );
                                }
                            }
                        }
                        return Ok(());
                    }
                    _ = timer.tick() => {
                        if telemetry_process.is_some(){
                            println!("telemetry process is running");
                        } else {
                            println!("telemetry process is not running");
                        }
                    }
                }
            }
        });
        self.telemetry_task_token = telemetry_task_token_cloned;
        Ok(true)
    }
    pub fn kill_telemetry_process(&self) -> Result<bool> {
        let exist_telemetry_task_token = &self.telemetry_task_token;
        if exist_telemetry_task_token.is_some() {
            let _ = exist_telemetry_task_token.as_ref().unwrap().cancel();
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<TelemetryMessage>) -> Result<()> {
        info!(func = "run", package = env!("CARGO_PKG_NAME"), "init");
        // Start the service
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {
                        TelemetryMessage::SendLogs {logs, logs_type, reply_to } => {
                            let result = process_logs(logs_type, logs, self.identity_tx.clone(), self.messaging_tx.clone(), self.settings_tx.clone() ).await;
                            let _ = reply_to.send(result);
                        }
                        TelemetryMessage::SendMetrics {metrics, metrics_type, reply_to } => {
                            let result = process_metrics(metrics, metrics_type, self.identity_tx.clone(), self.messaging_tx.clone(), self.settings_tx.clone()).await;
                            let _ = reply_to.send(result);
                        }
                    };
                }
              // Receive events from other services
              event = event_rx.recv() => {
                if event.is_err() {
                    continue;
                }
                match event.unwrap() {
                    Event::Messaging(events::MessagingEvent::Connected) => {
                        // let _ = &self.initialize_telemetry().await; //todo: deprecated
                        let _ = init_otlp_configuration();
                        match initialize_metrics().await {
                            Ok(_) => println!("metrics initialized"),
                            Err(e) => println!("error initializing metrics: {:?}", e),
                        }
                    },
                    Event::Messaging(events::MessagingEvent::Disconnected) => {
                        let _ = &self.kill_telemetry_process();
                    },
                    Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                        let _ = &self.kill_telemetry_process();
                    },
                    Event::Settings(events::SettingEvent::Updated { settings }) => {
                        info!(
                            func = "run",
                            package = env!("CARGO_PKG_NAME"),
                            "settings updated event received"
                        );
                        match settings.get("telemetry.enabled") {
                            Some(value) => {
                                if value == "true" {
                                    let _ = &self.initialize_telemetry().await;
                                } else if value == "false" {
                                    let _ = &self.kill_telemetry_process();
                                } else {
                                    // Can be add other function to perform
                                }
                            },
                            None => {},
                        }
                    },
                    _ => {}
                }
            }
            }
        }
    }
}
