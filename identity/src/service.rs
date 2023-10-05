use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use settings::AgentSettings;
use std::path::PathBuf;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Serialize, Deserialize, Debug)]
pub struct Identity {
    settings: AgentSettings,
}

impl Identity {
    pub fn new(settings: AgentSettings) -> Self {
        Self { settings: settings }
    }
    pub fn is_device_provisioned(&self) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "is_device_provisioned", "init",);

        let certificate_paths = match settings::read_settings_yml() {
            Ok(v) => v.provisioning.paths,
            Err(e) => bail!(e),
        };

        let device_cert_path = PathBuf::from(certificate_paths.device.cert);
        let device_private_key = PathBuf::from(certificate_paths.device.private_key);

        if device_cert_path.exists() && device_private_key.exists() {
            tracing::info!(
                trace_id,
                task = "is_device_provisioned",
                "device is provisioned"
            );
            Ok(true)
        } else {
            tracing::info!(
                trace_id,
                task = "is_device_provisioned",
                "device is not provisioned"
            );
            Ok(false)
        }
    }
    // pub fn sign_with_device_key(&self, data: Vec<u8>) -> Result<Vec<u8>> {
    //     let trace_id = find_current_trace_id();
    //     tracing::trace!(trace_id, task = "sign_with_device_key", "init",);

    //     let device_key_path = &self.settings.provisioning.paths.device.private_key;

    // }
}
