use std::fmt::Debug;

use axum::{http::StatusCode, Json};
use prost_types::Any;
use provisioning::service::{ProvisioningErrorResponseCode, ProvisioningService};
use serde_json::json;
use tonic::Code;

use crate::settings::read_yml;

pub async fn generate_provisioning_code() -> Result<String, ProvisioningErrorResponseCode> {
    let settings = read_yml().unwrap();
    let provisioning_service = ProvisioningService::new(settings.provisioning);
    let response = provisioning_service.new_request();
    match response {
        Ok(v) => Ok(v),
        Err(_e) => {
            return Err(ProvisioningErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Provisioning code generation failed : {}", _e),
            })
        }
    }
}

pub async fn manifest_handler() -> Result<String, ProvisioningErrorResponseCode> {
    let settings = read_yml().unwrap();
    let provisioning_service = ProvisioningService::new(settings.provisioning);
    let response = provisioning_service.manifest_request().await;
    match response {
        Ok(v) => Ok(v),
        Err(e) => {
            return Err(ProvisioningErrorResponseCode {
                code: e.code,
                message: e.message,
            })
        }
    }
}
