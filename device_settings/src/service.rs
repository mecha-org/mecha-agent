use anyhow::{bail, Result};
use futures::StreamExt;
use key_value_store::KevValueStoreClient;
use messaging::service::{Messaging, MessagingScope};
use nats_client::jetstream::JetStreamClient;
use serde::{Deserialize, Serialize};
use settings::AgentSettings;

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceSettings {
    settings: AgentSettings,
}

impl DeviceSettings {
    pub fn new(settings: AgentSettings) -> Self {
        Self {
            settings: settings.clone(),
        }
    }

    pub async fn start(&self) -> Result<bool> {
        println!("start_device_settings_service");
        //initiate messaging client to subscribe topic
        let mut messaging_client: Messaging = Messaging::new(MessagingScope::System, true);
        let _ = match messaging_client.connect().await {
            Ok(s) => s,
            Err(e) => bail!(e),
        };
        let cloned_messaging_client = messaging_client.clone();

        let nats_client = match cloned_messaging_client.get_nats_client().await {
            Ok(s) => s,
            Err(e) => bail!(e),
        };
        //create consumer
        let jet_stream_client = JetStreamClient::new(nats_client.client.clone().unwrap());
        let consumer = match jet_stream_client
            .create_consumer("stream".to_string(), None, "subject".to_string())
            .await
        {
            Ok(s) => s,
            Err(e) => bail!(e),
        };

        println!("conumer wating for messages");
        let key_value_store =
            match KevValueStoreClient::new("mention_storage_file_path".to_string()) {
                Ok(s) => s,
                Err(e) => bail!(e),
            };
        let mut messages = consumer.messages().await?.take(1000);
        while let Some(Ok(message)) = messages.next().await {
            println!("got message {:?}", message);
            match key_value_store.set("mecha", "Systems") {
                Ok(s) => println!("set result {:?}", s),
                Err(err) => println!("set error {:?}", err),
            };
            match key_value_store.get("minesh") {
                Ok(s) => println!("get result {:?}", s),
                Err(err) => println!("get error {:?}", err),
            }

            match message.ack().await {
                Ok(res) => println!("ack result {:?}", res),
                Err(err) => print!("ack error {:?}", err),
            };
        }

        Ok(true)
    }
}
