use anyhow::{bail, Result};
use async_nats::jetstream::{
    consumer::{pull::Config, Consumer},
    stream::Stream,
};

use crate::errors::{NatsClientError, NatsClientErrorCodes};

#[derive(Clone, Debug)]
pub struct JetStreamClient {
    pub client: async_nats::Client,
}

impl JetStreamClient {
    pub fn new(client: async_nats::Client) -> Self {
        Self { client }
    }

    pub async fn get_stream(&self, stream_name: String) -> Result<Stream> {
        let jetstream = async_nats::jetstream::new(self.client.clone());
        let stream = match jetstream.get_stream(stream_name).await {
            Ok(s) => s,
            Err(e) => {
                bail!(NatsClientError::new(
                    NatsClientErrorCodes::GetStreamError,
                    format!("get stream error {:?}", e),
                    true
                ))
            }
        };
        Ok(stream)
    }
    pub async fn create_consumer(
        &self,
        stream: Stream,
        consumer: Option<String>,
    ) -> Result<Consumer<Config>> {
        let consumer = match stream
            .get_or_create_consumer(
                "consumer",
                async_nats::jetstream::consumer::pull::Config {
                    durable_name: consumer,
                    ..Default::default()
                },
            )
            .await
        {
            Ok(s) => s,
            Err(e) => bail!(NatsClientError::new(
                NatsClientErrorCodes::CreateConsumerError,
                format!("Error creating consumer - {}", e),
                true
            )),
        };
        Ok(consumer)
    }
}
