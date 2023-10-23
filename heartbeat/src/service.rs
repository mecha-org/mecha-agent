use std::time::Duration;

use anyhow::{bail, Result};
use messaging::{
    service::{Messaging, MessagingScope},
    Bytes,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use settings::AgentSettings;
use sha256::digest;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

use crate::errors::{HeatbeatError, HeatbeatErrorCodes};

#[derive(Serialize, Deserialize, Debug)]
pub struct HeartbeatPublishPayload {
    pub time: String,
    pub device_id: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Heatbeat {
    settings: AgentSettings,
}

impl Heatbeat {
    pub fn new(settings: AgentSettings) -> Self {
        Self {
            settings: settings.clone(),
        }
    }
    pub async fn start(&self) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::info!(
            task = "start",
            trace_id = trace_id,
            "starting heartbeat service"
        );

        //initiate messaging service and publish a message
        let mut messaging_client = Messaging::new(MessagingScope::System, true);
        let _ = match messaging_client.connect().await {
            Ok(s) => s,
            Err(e) => bail!(HeatbeatError::new(
                HeatbeatErrorCodes::InitMessagingClientError,
                format!("error initializing messaging client - {}", e),
                true
            )),
        };

        let subject_name_result =
            crypto::x509::get_subject_name(&self.settings.provisioning.paths.device.cert);
        let subject_name = match subject_name_result {
            Ok(v) => v,
            Err(e) => bail!(e),
        };

        // generate sha256 digest of subject name
        let topic_to_suscribe = format!("device.{}.heartbeat", digest(subject_name.to_string()));

        // subscribe to the system topic every 1 minutes
        let result: tokio::task::JoinHandle<std::result::Result<bool, anyhow::Error>> =
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));
                loop {
                    interval.tick().await; // This should go first.
                    let current_utc_time = chrono::Utc::now();
                    let formatted_utc_time =
                        current_utc_time.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
                    let publish_payload = HeartbeatPublishPayload {
                        time: formatted_utc_time,
                        device_id: subject_name.to_string(),
                    };

                    let payload_payload_json = json!(publish_payload);
                    let _ = match messaging_client
                        .publish(
                            &topic_to_suscribe,
                            Bytes::from(payload_payload_json.to_string()),
                        )
                        .await
                    {
                        Ok(s) => s,
                        Err(e) => bail!(e),
                    };
                }
            });
        Ok(result.is_finished())
    }
}
