use anyhow::{bail, Result};
use tonic::transport::Channel;


#[allow(non_snake_case)]
pub mod provisioning {
    tonic::include_proto!("provisioning");
}

pub use provisioning::{
    provisioning_service_client::ProvisioningServiceClient,
    // provisioning_service_server::ProvisioningService,
    ProvisioningCodeRequest, ProvisioningCodeResponse, Empty,
    ProvisioningStatusResponse, DeProvisioningStatusResponse,
    PingResponse,
};

#[derive(Debug, Clone)]
pub struct ProvisionManagerClient {
    client: ProvisioningServiceClient<Channel>,
}

impl ProvisionManagerClient {

    pub async fn new() -> Result<Self> {
        let url = "http://localhost:3001".to_string();

        let client: ProvisioningServiceClient<Channel> = match ProvisioningServiceClient::connect(url).await {
            Ok(client) => client,
            Err(e) => {
                bail!("Error in ProvisioningServiceClient: {:?}", e);
            }
           
        };

        Ok(Self { client })
    }

    pub async fn generate_code(&mut self) -> Result<ProvisioningCodeResponse> {
        let request = tonic::Request::new(Empty {});

        let response = match self.client.generate_code(request).await {
            Ok(response) => {
                println!("grpc function: generate code response: {:?} ", response);
                response.into_inner()
            },
            Err(e) => {
                bail!("Error in getting code: {:?}", e);
            },
        };

        Ok(response)
    }

    pub async fn provision_by_code(&mut self, code: String) -> Result<ProvisioningStatusResponse> {

        let request: tonic::Request<ProvisioningCodeRequest> = tonic::Request::new(ProvisioningCodeRequest {code: code.clone() as String});

        let response = match self.client.provision_by_code(request).await {
            Ok(response) => {
                println!("grpc function: provision by code response: {:?} for code {:?}", response, code.clone());
                response.into_inner()
            },
            Err(e) => {
                bail!("Error in getting code: {:?}", e);
            },
        };

        Ok(response)
    }
 
    pub async fn ping(&mut self) ->  Result<PingResponse> {
        let request = tonic::Request::new(Empty {});

        let response = match self.client.ping(request).await {
            Ok(response) => {
                println!("grpc function: ping response: {:?} ", response);   
                response.into_inner()
            },
            Err(e) => {
                bail!(e);
            },
        };
        Ok(response)
    }

}
