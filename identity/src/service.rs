use anyhow::{bail, Result};
use crypto::MachineCert;
use fs::construct_dir_path;
use tracing::{error, info, trace};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub fn get_provision_status() -> Result<bool> {
    let fn_name = "get_provision_status";
    trace!(
        func = "get_provisioning_status",
        package = PACKAGE_NAME,
        "init",
    );

    let certificate_paths = match agent_settings::read_settings_yml() {
        Ok(v) => v.provisioning.paths,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to read settings.yml: {:?}",
                e
            );
            bail!(e)
        }
    };

    let machine_cert_path = match construct_dir_path(&certificate_paths.machine.cert) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to construct machine cert path: {:?}, err - {:?}",
                &certificate_paths.machine.cert,
                e
            );
            bail!(e)
        }
    };
    let machine_private_key = match construct_dir_path(&certificate_paths.machine.private_key) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to construct machine private key path: {:?}, err - {:?}",
                &certificate_paths.machine.private_key,
                e
            );
            bail!(e)
        }
    };

    if machine_cert_path.exists() && machine_private_key.exists() {
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "device is provisioned"
        );
        Ok(true)
    } else {
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "device is not provisioned"
        );
        Ok(false)
    }
}
pub fn get_machine_id() -> Result<String> {
    trace!(func = "get_machine_id", "init");
    let machine_id = match crypto::get_machine_id() {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "failed to get machine id: {:?}",
                e
            );
            bail!(e)
        }
    };
    Ok(machine_id)
}
pub fn get_machine_cert() -> Result<MachineCert> {
    trace!(func = "get_machine_cert", "init");
    let machine_cert = match crypto::get_machine_cert() {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "get_machine_cert",
                package = PACKAGE_NAME,
                "failed to get machine cert: {:?}",
                e
            );
            bail!(e)
        }
    };
    Ok(machine_cert)
}
