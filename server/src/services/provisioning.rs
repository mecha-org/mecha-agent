use provisioning::service::Provisioning;
use settings::{AgentSettings, read_settings_yml};
use tonic::{Request, Status, Response};
use anyhow::Result;
use crate::agent::{provisioning_service_server::ProvisioningService, Empty, ProvisioningCodeRequest, ProvisioningStatusResponse, ProvisioningCodeResponse};

#[derive(Debug, Default)]
pub struct ProvisioningServiceHandler {}

fn new_provisioning_service() -> Provisioning {
    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };

    Provisioning::new(settings.provisioning)
}

#[tonic::async_trait]
impl ProvisioningService for ProvisioningServiceHandler {
    async fn generate_code(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ProvisioningCodeResponse>, Status> {
        let provisioning_service = new_provisioning_service();
        let provisioning_code = provisioning_service.generate_code();

        match provisioning_code {
            Ok(v) => Ok(Response::new(ProvisioningCodeResponse {
                code:v
            })),
            Err(err) => Err(Status::from_error(err.into())),
        }
    }

    async fn provision_me(
        &self,
        request: Request<ProvisioningCodeRequest>,
    ) -> Result<Response<ProvisioningStatusResponse>, Status> {
        let provisioning_service = new_provisioning_service();
        let provisioning_code = &request.into_inner().code;

        if provisioning_code.is_empty() {
            return Err(Status::invalid_argument("code not specified in request"))
        }

        let provisioning_status = provisioning_service.provision_me(provisioning_code).await;

        match provisioning_status {
            Ok(v) => Ok(Response::new(ProvisioningStatusResponse {
                success: v,
            })),
            Err(err) => Err(Status::from_error(err.into())),
        }
    }
}
