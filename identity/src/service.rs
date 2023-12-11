use anyhow::{bail, Result};
use crypto::MachineCert;
use std::path::PathBuf;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

pub fn get_provision_status() -> Result<bool> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "get_provisioning_status", "init",);

    let certificate_paths = match agent_settings::read_settings_yml() {
        Ok(v) => v.provisioning.paths,
        Err(e) => bail!(e),
    };

    let device_cert_path = PathBuf::from(certificate_paths.device.cert);
    let device_private_key = PathBuf::from(certificate_paths.device.private_key);

    if device_cert_path.exists() && device_private_key.exists() {
        tracing::info!(
            trace_id,
            task = "get_provisioning_status",
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
pub fn get_machine_id() -> Result<String> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "get_machine_id", "init");
    let machine_id = match crypto::get_machine_id() {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    Ok(machine_id)
}
pub fn get_machine_cert() -> Result<MachineCert> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "get_machine_cert", "init");
    let machine_cert = match crypto::get_machine_cert() {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    Ok(machine_cert)
}
