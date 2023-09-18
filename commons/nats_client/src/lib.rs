use std::sync::Arc;
use bytes::Bytes;
use anyhow::{bail, Result};
use nkeys::KeyPair;
use tracing::info;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;
use crate::{errors::{NatsClientError, NatsClientErrorCodes}, jwt::create_dummy_jwt};

pub mod errors;
pub mod jwt;

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

    pub async fn connect(&mut self, token: &str) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, target = "nats_client", task = "connect", "init");

        info!(target = "nats_client",  task = "connect", "connecting to nats");

        // connect to nats
        let key_pair = self.user_key_pair.clone();
        let dummy_jwt = create_dummy_jwt(&key_pair).unwrap();

        self.client = match async_nats::ConnectOptions::new()
        .jwt(String::from(dummy_jwt), move |nonce| {
            let signing_key = KeyPair::from_seed(&key_pair.seed().unwrap()).unwrap();
            async move { signing_key.sign(&nonce).map_err(async_nats::AuthError::new) }
        })
        .connect(&self.address)
        .await {
            Ok(c) => Some(c),
            Err(e) => bail!(NatsClientError::new(
                NatsClientErrorCodes::UnknownError,
                format!("nats client unknown error - {}", e),
                true
            )),
        };

        info!(target = "nats_client",  task = "connect", "nats client connected");

        Ok(true)
    }

    // pub async fn publish(&self, subject: &str, data: Bytes) -> Result<Bool> {
    //     let client = match self.client {
    //         Some(_) => todo!(),
    //         None => bail!(NatsClientError::new(
    //             NatsClientErrorCodes::ClientUninitialized,
    //             format!("nats client uninitialized"),
    //             true
    //         )),
    //     };
    //     self.client.publish(subject.clone(), data.clone()).await?;
    //     Ok(true)
    // }

}
