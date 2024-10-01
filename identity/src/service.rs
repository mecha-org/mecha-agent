use crate::errors::{IdentityError, IdentityErrorCodes};
use agent_settings::constants;
use anyhow::{bail, Result};
use crypto::MachineCertDetails;
use tracing::{error, trace};

const PACKAGE_NAME: &str = env!("CARGO_CRATE_NAME");
pub fn get_provision_status(data_dir: &str) -> Result<bool> {
    let fn_name = "get_provision_status";
    trace!(
        func = "get_provisioning_status",
        package = PACKAGE_NAME,
        "init",
    );
    let machine_cert_path = data_dir.to_owned() + constants::CERT_PATH;
    let machine_id = match crypto::get_machine_id(&machine_cert_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to get machine id: {:?}",
                e
            );
            bail!(IdentityError::new(
                IdentityErrorCodes::GetMachineIdError,
                format!("failed to get machine id: {:?}", e)
            ));
        }
    };
    Ok(!machine_id.is_empty()) // if machine_id is not empty, then the machine is provisioned
}
pub fn get_machine_id(data_dir: &str) -> Result<String> {
    let fn_name = "get_machine_id";
    let public_key_path = data_dir.to_owned() + constants::CERT_PATH;
    let machine_id = match crypto::get_machine_id(&public_key_path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "failed to get machine id: {:?}",
                e
            );
            bail!(IdentityError::new(
                IdentityErrorCodes::GetMachineIdError,
                format!("failed to get machine id: {:?}", e)
            ));
        }
    };
    Ok(machine_id)
}
pub fn get_machine_cert(data_dir: &str) -> Result<MachineCertDetails> {
    let fn_name = "get_machine_cert";
    let public_key_path = data_dir.to_owned() + constants::CERT_PATH;
    let ca_bundle_path = data_dir.to_owned() + constants::CA_BUNDLE_PATH;
    let root_cert_path = data_dir.to_owned() + constants::ROOT_CERT_PATH;
    let machine_cert =
        match crypto::get_machine_cert(&public_key_path, &ca_bundle_path, &root_cert_path) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "failed to get machine cert: {:?}",
                    e
                );
                bail!(IdentityError::new(
                    IdentityErrorCodes::GetMachineCertError,
                    format!("failed to get machine cert: {:?}", e)
                ));
            }
        };
    Ok(machine_cert)
}
