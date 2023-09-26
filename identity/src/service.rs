
use std::fs;
use anyhow::{bail, Result};
use serde::{Serialize, Deserialize};
use settings::AgentSettings;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Serialize, Deserialize, Debug)]
pub struct Identity {
    settings: AgentSettings,
}

impl Identity {
    pub fn new(settings: AgentSettings) -> Self {
        Self { settings: settings }
    }

    // pub fn sign_with_device_key(&self, data: Vec<u8>) -> Result<Vec<u8>> {
    //     let trace_id = find_current_trace_id();
    //     tracing::trace!(trace_id, task = "sign_with_device_key", "init",);

    //     let device_key_path = &self.settings.provisioning.paths.device.private_key;

    // }
}
