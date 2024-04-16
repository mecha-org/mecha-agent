use crate::errors::{NatsClientError, NatsClientErrorCodes};
use anyhow::{bail, Result};
pub use async_nats::Subscriber;
pub use bytes::Bytes;
use events::Event;
use nkeys::KeyPair;
use std::{str::FromStr, sync::Arc};
use tokio::sync::broadcast::Sender;
use tracing::{debug, error, info, trace};

pub mod errors;
pub mod jetstream;
pub use async_nats;
pub use async_nats::jetstream::message::Message;
pub use async_nats::Event as NatsEvent;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Clone)]
pub struct NatsClient {
    pub address: String,
    pub user_public_key: String,
    pub client: Option<async_nats::Client>,
    user_key_pair: Arc<KeyPair>,
}

impl NatsClient {
    pub fn new(address: &str) -> Self {
        let user_key = KeyPair::new_user();
        Self {
            user_public_key: user_key.public_key(),
            address: String::from(address),
            user_key_pair: Arc::new(user_key),
            client: None,
        }
    }

    pub async fn connect(
        &mut self,
        token: &str,
        inbox_prefix: &str,
        event_tx: Sender<Event>,
    ) -> Result<bool> {
        trace!(
            func = "connect",
            package = PACKAGE_NAME,
            "nats client connecting, inbox_prefix - {}",
            inbox_prefix
        );
        // Connect to nats
        let key_pair = self.user_key_pair.clone();
        self.client = match async_nats::ConnectOptions::new()
            .event_callback(move |event: async_nats::Event| {
                let tx = event_tx.clone();
                async move {
                    match tx.clone().send(Event::Nats(event)) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(
                                func = "connect",
                                package = PACKAGE_NAME,
                                "error sending messaging service event on nats disconnect- {}",
                                e
                            );
                        }
                    }
                }
            })
            .jwt(String::from(token), move |nonce| {
                let signing_key = KeyPair::from_seed(&key_pair.seed().unwrap()).unwrap();
                async move { signing_key.sign(&nonce).map_err(async_nats::AuthError::new) }
            })
            .custom_inbox_prefix(inbox_prefix)
            .connect(&self.address)
            .await
        {
            Ok(c) => Some(c),
            Err(e) => {
                error!(
                    func = "connect",
                    package = PACKAGE_NAME,
                    "nats client connection error - {}",
                    e.to_string()
                );
                bail!(NatsClientError::new(
                    NatsClientErrorCodes::ClientConnectError,
                    format!(
                        "nats client connection error - {:?} - {}",
                        e.kind(),
                        e.to_string()
                    ),
                    true
                ))
            }
        };
        info!(
            func = "connect",
            package = PACKAGE_NAME,
            "nats client connected"
        );
        Ok(true)
    }

    pub fn get_connected_client(&self) -> Result<&async_nats::Client> {
        let client = match &self.client {
            Some(c) => c,
            None => {
                error!(
                    func = "get_connected_client",
                    package = PACKAGE_NAME,
                    "nats client uninitialized"
                );
                bail!(NatsClientError::new(
                    NatsClientErrorCodes::ClientUninitializedError,
                    format!("nats client uninitialized"),
                    false
                ))
            }
        };
        match client.connection_state() {
            async_nats::connection::State::Connected => (),
            async_nats::connection::State::Pending => {
                error!(
                    func = "get_connected_client",
                    package = PACKAGE_NAME,
                    "nats client connection state is pending"
                );
                bail!(NatsClientError::new(
                    NatsClientErrorCodes::ClientNotConnectedError,
                    format!("nats client is not connected, not ready to send or receive messages"),
                    true
                ))
            }
            async_nats::connection::State::Disconnected => {
                error!(
                    func = "get_connected_client",
                    package = PACKAGE_NAME,
                    "nats client connection state is disconnected"
                );
                bail!(NatsClientError::new(
                NatsClientErrorCodes::ClientDisconnectedError,
                format!("nats client state is disconnected, reconnect to continue sending, receiving messages"),
                false
            ))
            }
        };

        Ok(client)
    }

    pub async fn publish(&self, subject: &str, data: Bytes) -> Result<bool> {
        trace!(
            func = "publish",
            package = PACKAGE_NAME,
            "nats client publishing message to subject - {}",
            subject
        );
        let client = match self.get_connected_client() {
            Ok(c) => c,
            Err(e) => {
                error!(
                    func = "publish",
                    package = PACKAGE_NAME,
                    "nats client error - {}",
                    e.to_string()
                );
                bail!(e)
            }
        };

        // Set headers
        let version_detail = format!("mecha_agent@{}", env!("CARGO_PKG_VERSION"));
        let mut headers = async_nats::HeaderMap::new();
        headers.insert(
            "X-Agent",
            async_nats::HeaderValue::from_str(version_detail.as_str()).unwrap(),
        );
        debug!(
            func = "publish",
            package = PACKAGE_NAME,
            "nats client publishing message to subject - {}, headers - {:?}",
            subject,
            headers
        );
        match client
            .publish_with_headers(String::from(subject), headers, data.clone())
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = "publish",
                    package = PACKAGE_NAME,
                    "nats client error - {}",
                    e.to_string()
                );
                bail!(NatsClientError::new(
                    NatsClientErrorCodes::PublishError,
                    format!(
                        "error publishing message to sub - {}, error - {}",
                        subject, e
                    ),
                    true
                ))
            }
        }
        Ok(true)
    }

    pub async fn request(&self, subject: &str, data: Bytes) -> Result<Bytes> {
        trace!(
            func = "request",
            package = PACKAGE_NAME,
            "nats client requesting message to subject - {}",
            subject
        );

        let client = match self.get_connected_client() {
            Ok(c) => c,
            Err(e) => {
                error!(
                    func = "request",
                    package = PACKAGE_NAME,
                    "nats client error - {}",
                    e.to_string()
                );
                bail!(e)
            }
        };

        let response = match client.request(String::from(subject), data.clone()).await {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = "request",
                    package = PACKAGE_NAME,
                    "nats client error - {}",
                    e.to_string()
                );
                bail!(NatsClientError::new(
                    NatsClientErrorCodes::RequestError,
                    format!(
                        "error requesting message to sub - {}, error - {}",
                        subject, e
                    ),
                    true
                ))
            }
        };
        Ok(response.payload)
    }

    pub async fn subscribe(&self, subject: &str) -> Result<Subscriber> {
        trace!(
            func = "subscribe",
            package = PACKAGE_NAME,
            "nats client subscribing to subject - {}",
            subject
        );

        let client = match self.get_connected_client() {
            Ok(c) => c,
            Err(e) => {
                error!(
                    func = "subscribe",
                    package = PACKAGE_NAME,
                    "nats client error to get connect client - {}",
                    e.to_string()
                );
                bail!(e)
            }
        };

        let subscriber = match client.subscribe(String::from(subject)).await {
            Ok(s) => s,
            Err(e) => {
                error!(
                    func = "subscribe",
                    package = PACKAGE_NAME,
                    "nats client error on subscribe - {}",
                    e.to_string()
                );
                bail!(NatsClientError::new(
                    NatsClientErrorCodes::SubscribeError,
                    format!("error subscriber to sub - {}, error - {}", subject, e),
                    true
                ))
            }
        };
        info!(
            func = "subscribe",
            package = PACKAGE_NAME,
            "nats client subscribed to subject ",
        );
        Ok(subscriber)
    }
}
