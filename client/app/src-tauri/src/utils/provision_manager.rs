use anyhow::{bail, Result};
use tonic::transport::Channel;

#[allow(non_snake_case)]
pub mod provisioning {
    tonic::include_proto!("provisioning");
}

pub use provisioning::{
    provisioning_service_client::ProvisioningServiceClient,
    Empty,
    PingResponse,
    ProvisioningCodeRequest,
    ProvisioningCodeResponse,
    ProvisioningStatusResponse,
    // DeProvisioningStatusResponse,
    // provisioning_service_server::ProvisioningService,
};

#[derive(Debug, Clone)]
pub struct ProvisionManagerClient {
    client: ProvisioningServiceClient<Channel>,
}

impl ProvisionManagerClient {
    pub async fn new() -> Result<Self> {
        let url = "http://localhost:3001".to_string();

        let client: ProvisioningServiceClient<Channel> =
            match ProvisioningServiceClient::connect(url).await {
                Ok(client) => client,
                Err(e) => {
                    bail!(e);
                }
            };

        Ok(Self { client })
    }

    pub async fn generate_code(&mut self) -> Result<ProvisioningCodeResponse> {
        let request = tonic::Request::new(Empty {});

        let response = match self.client.generate_code(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                bail!(e);
            }
        };

        Ok(response)
    }

    pub async fn provision_by_code(&mut self, code: String) -> Result<ProvisioningStatusResponse> {
        let request: tonic::Request<ProvisioningCodeRequest> =
            tonic::Request::new(ProvisioningCodeRequest {
                code: code.clone() as String,
            });

        let response = match self.client.provision_by_code(request).await {
            Ok(response) => {
                // println!("provisioning_response : {:?}", response);
                response.into_inner()
            }
            Err(e) => {
                bail!(e);
            }
        };

        Ok(response)
    }

    pub async fn ping(&mut self) -> Result<PingResponse> {
        let request = tonic::Request::new(Empty {});

        let response = match self.client.ping(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                bail!(e);
            }
        };
        Ok(response)
    }
}
