use crate::errors::{NatsClientError, NatsClientErrorCodes};
use anyhow::{bail, Result};
pub use async_nats::Subscriber;
pub use bytes::Bytes;
use nkeys::KeyPair;
use std::sync::Arc;
use tracing::info;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

pub mod errors;
pub mod jetstream;
pub use async_nats::jetstream::message::Message;

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

    pub async fn connect(&mut self, token: &str, inbox_prefix: &str) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, target = "nats_client", task = "connect", "init");

        info!(
            target = "nats_client",
            task = "connect",
            "connecting to nats"
        );
        // Connect to nats
        let key_pair = self.user_key_pair.clone();
        self.client = match async_nats::ConnectOptions::new()
            .jwt(String::from(token), move |nonce| {
                let signing_key = KeyPair::from_seed(&key_pair.seed().unwrap()).unwrap();
                async move { signing_key.sign(&nonce).map_err(async_nats::AuthError::new) }
            })
            .custom_inbox_prefix(inbox_prefix)
            .connect(&self.address)
            .await
        {
            Ok(c) => Some(c),
            Err(e) => bail!(NatsClientError::new(
                NatsClientErrorCodes::ClientConnectError,
                format!(
                    "nats client connection error - {:?} - {}",
                    e.kind(),
                    e.to_string()
                ),
                true
            )),
        };
        info!(
            target = "nats_client",
            task = "connect",
            "nats client connected"
        );

        Ok(true)
    }

    pub fn get_connected_client(&self) -> Result<&async_nats::Client> {
        let client = match &self.client {
            Some(c) => c,
            None => bail!(NatsClientError::new(
                NatsClientErrorCodes::ClientUninitializedError,
                format!("nats client uninitialized"),
                true
            )),
        };
        match client.connection_state() {
            async_nats::connection::State::Connected => (),
            async_nats::connection::State::Pending => bail!(NatsClientError::new(
                NatsClientErrorCodes::ClientNotConnectedError,
                format!("nats client is not connected, not ready to send or receive messages"),
                true
            )),
            async_nats::connection::State::Disconnected => bail!(NatsClientError::new(
                NatsClientErrorCodes::ClientDisconnectedError,
                format!("nats client state is disconnected, reconnect to continue sending, receiving messages"),
                true
            )),
        };

        Ok(client)
    }

    pub async fn publish(&self, subject: &str, data: Bytes) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, target = "nats_client", task = "publish", "init");
        let client = match self.get_connected_client() {
            Ok(c) => c,
            Err(e) => bail!(e),
        };

        tracing::trace!(
            trace_id,
            target = "nats_client",
            task = "publish",
            "nats client is in connected status"
        );
        match client.publish(String::from(subject), data.clone()).await {
            Ok(v) => v,
            Err(e) => {
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
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, target = "nats_client", task = "request", "init");

        let client = match self.get_connected_client() {
            Ok(c) => c,
            Err(e) => bail!(e),
        };

        tracing::trace!(
            trace_id,
            target = "nats_client",
            task = "request",
            "nats client is in connected status"
        );

        let response = match client.request(String::from(subject), data.clone()).await {
            Ok(v) => v,
            Err(e) => bail!(NatsClientError::new(
                NatsClientErrorCodes::RequestError,
                format!(
                    "error requesting message to sub - {}, error - {}",
                    subject, e
                ),
                true
            )),
        };
        Ok(response.payload)
    }

    pub async fn subscribe(&self, subject: &str) -> Result<Subscriber> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, target = "nats_client", task = "publish", "init");

        let client = match self.get_connected_client() {
            Ok(c) => c,
            Err(e) => bail!(e),
        };

        let subscriber = match client.subscribe(String::from(subject)).await {
            Ok(s) => s,
            Err(e) => bail!(NatsClientError::new(
                NatsClientErrorCodes::SubscribeError,
                format!("error subscriber to sub - {}, error - {}", subject, e),
                true
            )),
        };

        Ok(subscriber)
    }
}
