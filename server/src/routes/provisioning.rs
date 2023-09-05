use std::fmt::Debug;

use axum::http::StatusCode;

use provisioning::service::{ProvisioningErrorResponseCode, ProvisioningService};
use serde::{Deserialize, Serialize};
use tonic::Code;

use crate::settings::read_yml;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryParams {
    pub code: String,
}
pub async fn generate_provisioning_code(
) -> Result<(StatusCode, String), ProvisioningErrorResponseCode> {
    let settings = read_yml().unwrap();
    let provisioning_service = ProvisioningService::new(settings.provisioning);
    let response = provisioning_service.new_request();
    match response {
        Ok(v) => Ok((StatusCode::OK, v)),
        Err(_e) => {
            return Err(ProvisioningErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Provisioning code generation failed : {}", _e),
            })
        }
    }
}

pub async fn request_manifest_handler(
    code: String,
) -> Result<(StatusCode, bool), ProvisioningErrorResponseCode> {
    let settings = read_yml().unwrap();
    let provisioning_service = ProvisioningService::new(settings.provisioning);
    let response = provisioning_service.request_manifest(&code).await;
    match response {
        Ok(v) => Ok((StatusCode::OK, v)),
        Err(_e) => {
            return Err(ProvisioningErrorResponseCode {
                code: Code::InvalidArgument,
                message: format!("Provisioning code generation failed : {}", _e),
            })
        }
    }
}
