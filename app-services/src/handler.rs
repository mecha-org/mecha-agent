use anyhow::{bail, Result};
use events::Event;
use messaging::handler::MessagingMessage;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::errors::{AppServicesError, AppServicesErrorCodes};
use crate::service::{
    await_app_service_message, parse_settings_payload, reconnect_messaging_service,
    subscribe_to_nats, AppServiceSettings,
};
const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub struct AppServiceHandler {
    event_tx: broadcast::Sender<Event>,
    messaging_tx: mpsc::Sender<MessagingMessage>,
    app_services_subscriber_token: Option<CancellationToken>,
    sync_app_services_token: Option<CancellationToken>,
}

pub enum AppServiceMessage {}

pub struct AppServiceOptions {
    pub event_tx: broadcast::Sender<Event>,
    pub messaging_tx: mpsc::Sender<MessagingMessage>,
}

impl AppServiceHandler {
    pub fn new(options: AppServiceOptions) -> Self {
        Self {
            event_tx: options.event_tx,
            messaging_tx: options.messaging_tx,
            app_services_subscriber_token: None,
            sync_app_services_token: None,
        }
    }
    async fn app_service_subscriber(&mut self, dns_name: String, local_port: u16) -> Result<bool> {
        let fn_name = "app_service_subscriber";
        // safety: check for existing cancel token, and cancel it
        let exist_settings_token = &self.app_services_subscriber_token;
        if exist_settings_token.is_some() {
            let _ = exist_settings_token.as_ref().unwrap().cancel();
        }
        // create a new token
        let settings_token = CancellationToken::new();
        let settings_token_cloned = settings_token.clone();
        let messaging_tx = self.messaging_tx.clone();
        let subscribers = match subscribe_to_nats(&dns_name, messaging_tx.clone()).await {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = "subscribe_to_nats",
                    package = PACKAGE_NAME,
                    "subscribe to nats error - {:?}",
                    e
                );
                bail!(e)
            }
        };
        let mut futures = JoinSet::new();
        futures.spawn(await_app_service_message(
            dns_name,
            local_port,
            messaging_tx.clone(),
            subscribers.service_request.unwrap(),
        ));
        // create spawn for timer
        let _: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            loop {
                select! {
                        _ = settings_token.cancelled() => {
                            info!(
                                func = fn_name,
                                package = PACKAGE_NAME,
                                result = "success",
                                "settings subscriber cancelled"
                            );
                            return Ok(());
                    },
                    result = futures.join_next() => {
                        if result.is_none() {
                            continue;
                        }
                    },
                }
            }
        });

        // Save to state
        self.app_services_subscriber_token = Some(settings_token_cloned);
        Ok(true)
    }
    fn clear_app_services_subscriber(&self) -> Result<bool> {
        let exist_subscriber_token = &self.app_services_subscriber_token;
        if exist_subscriber_token.is_some() {
            let _ = exist_subscriber_token.as_ref().unwrap().cancel();
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    fn clear_sync_settings_subscriber(&self) -> Result<bool> {
        let exist_subscriber_token = &self.sync_app_services_token;
        if exist_subscriber_token.is_some() {
            let _ = exist_subscriber_token.as_ref().unwrap().cancel();
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    pub async fn run(&mut self, mut message_rx: mpsc::Receiver<AppServiceMessage>) -> Result<()> {
        let fn_name = "run";
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "app service initiated"
        );
        let mut event_rx = self.event_tx.subscribe();
        loop {
            select! {
                msg = message_rx.recv() => {
                    if msg.is_none() {
                        continue;
                    }

                    match msg.unwrap() {};
                }

                // Receive events from other services
                event = event_rx.recv() => {
                    if event.is_err() {
                        continue;
                    }
                    match event.unwrap() {
                        Event::Messaging(events::MessagingEvent::Disconnected) => {
                            info!(
                                func = fn_name,
                                package = PACKAGE_NAME,
                                "disconnected event in app service"
                            );
                            let _ = self.clear_sync_settings_subscriber();
                            let _ = self.clear_app_services_subscriber();
                        }
                        Event::Provisioning(events::ProvisioningEvent::Deprovisioned) => {
                            info!(
                                func = fn_name,
                                package = PACKAGE_NAME,
                                "deprovisioned event in app service"
                            );
                            let _ = self.clear_sync_settings_subscriber();
                            let _ = self.clear_app_services_subscriber();
                        }
                        Event::Settings(events::SettingEvent::Updated{ existing_settings, new_settings })  => {
                            info!(
                                func = "run",
                                package = PACKAGE_NAME,
                                "settings updated event in app service"
                            );
                            //TODO: create function to handle settings update
                            for (key, value) in new_settings.into_iter() {
                                println!("settings updated: {}", key);
                                if key == "app_services.config" {
                                    info!(
                                        func = fn_name,
                                        package = PACKAGE_NAME,
                                        "dns_name updated event in app service: {}", value
                                    );
                                    println!("app_service config: {}", value);
                                    if value.is_empty() {
                                        println!("empty value");
                                        // let _ = reconnect_messaging_service(self.messaging_tx.clone(), String::from(""), existing_settings.clone()).await;
                                        let _ = self.clear_app_services_subscriber();
                                        let _ = self.clear_sync_settings_subscriber();
                                        continue;
                                    } else {
                                        let app_service_settings:AppServiceSettings = match parse_settings_payload(value) {
                                            Ok(s) => s,
                                            Err(e) => {
                                                error!(
                                                    func = fn_name,
                                                    package = PACKAGE_NAME,
                                                    "error extracting req_id from key, error -  {:?}",
                                                    e
                                                );
                                                bail!(e)
                                            }
                                        };
                                        info!(
                                            func = fn_name,
                                            package = PACKAGE_NAME,
                                            "dns_name updated event in app service: {} / {}", app_service_settings.dns_name, app_service_settings.app_id
                                        );
                                            let local_port:u16 = match app_service_settings.port_mapping[0].local_port.parse::<u16>() {
                                                Ok(s) => s,
                                                Err(e) => {
                                                    error!(
                                                        func = fn_name,
                                                        package = PACKAGE_NAME,
                                                        "error extracting req_id from key, error -  {:?}",
                                                        e
                                                    );
                                                    bail!(AppServicesError::new(
                                                        AppServicesErrorCodes::PortParseError,
                                                        format!("error parsing local port - {:?} ", e.to_string()),
                                                    ))
                                                }
                                            };
                                            let _ = reconnect_messaging_service(self.messaging_tx.clone(), app_service_settings.dns_name.clone(), existing_settings.clone()).await;
                                            match self.app_service_subscriber(app_service_settings.dns_name, local_port).await {
                                                Ok(_) => {
                                                    info!(
                                                        func = fn_name,
                                                        package = PACKAGE_NAME,
                                                        "app service subscriber started"
                                                    );
                                                }
                                                Err(e) => {
                                                    error!(
                                                        func = fn_name,
                                                        package = PACKAGE_NAME,
                                                        "error starting app service subscriber - {:?}",
                                                        e
                                                    );
                                                }
                                            }

                                    }
                                }
                            }
                        },
                        _ => {}

                    }
                }
            }
        }
    }
}
