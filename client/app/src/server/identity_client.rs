use anyhow::{bail, Result};
use tonic::{transport::Channel, Code, Status};

#[allow(non_snake_case)]
pub mod identity { 
    tonic::include_proto!("identity");
}

pub use identity::{ 
    identity_service_client::IdentityServiceClient, 
    GetMachineIdRequest, GetMachineIdResponse,
    GetMachineCertRequest, GetMachineCertResponse
};

#[derive(Debug, Clone)]
pub struct IdentityClient 
{ 
    client: IdentityServiceClient<Channel>
}
impl IdentityClient { 
    
    pub async fn new() -> Result<Self> { 
        let url = "http://localhost:3001".to_string();

        let client: IdentityServiceClient<Channel> = match IdentityServiceClient::connect(url).await { 
            Ok(client) => 
            client, Err(e) => 
            { bail!("IdentityClient-error:: {:?}", e); } 
        };
            
            Ok(Self { client }) 
    }

    pub async fn getting_machine_id(&mut self) -> Result<GetMachineIdResponse> {
        let request = tonic::Request::new( GetMachineIdRequest{});

        let response = match self.client.get_machine_id(request).await {
            Ok(response) => {
                response.into_inner()
            },
            Err(e) => {
                bail!("Error in getting code: {:?}", e);
            },
        };
        Ok(response)
    }

    pub async fn get_machine_cert_details(&mut self) -> Result<GetMachineCertResponse> {
        let request = tonic::Request::new(GetMachineCertRequest{});

        let response = match self.client.get_machine_cert(request).await {
            Ok(response) => {
                response.into_inner()
            },
            Err(e) => {
                bail!("Error in getting code: {:?}", e);
            },
        };
        Ok(response)
    }

}


