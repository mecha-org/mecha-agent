use agent_settings::{read_settings_yml, AgentSettings};
use anyhow::{bail, Result};
use messaging::{
    messaging_service_client::MessagingServiceClient, AuthNonceRequest, AuthNonceResponse,
    IssueTokenRequest, IssueTokenResponse,
};
use provisioning::{
    CertSignRequest, CertSignResponse, Empty, FindManifestRequest, ManifestDetailsResponse,
};

use tonic::transport::Channel;

use crate::provisioning::provisioning_service_client::ProvisioningServiceClient;

pub mod provisioning {
    tonic::include_proto!("provisioning");
}
pub mod messaging {
    tonic::include_proto!("messaging");
}
pub struct ServicesClient {
    client: Channel,
}
impl ServicesClient {
    pub fn new() -> Self {
        let settings = match read_settings_yml() {
            Ok(s) => s,
            Err(e) => {
                println!("error while reading settings.yml: {:?}", e);
                AgentSettings::default()
            }
        };
        let client = Channel::from_static("http://localhost:3000").connect_lazy();
        Self { client }
    }
    pub async fn ping(&self) -> Result<bool> {
        let mut client = ProvisioningServiceClient::new(self.client.to_owned());
        let request = tonic::Request::new(Empty {});
        match client.ping(request).await {
            Ok(_res) => (),
            Err(e) => {
                let e = anyhow::Error::from(e);
                bail!(e);
            }
        };
        Ok(true)
    }

    pub async fn find_manifest(
        &self,
        request: FindManifestRequest,
    ) -> Result<ManifestDetailsResponse> {
        let mut client = ProvisioningServiceClient::new(self.client.to_owned());
        let request = tonic::Request::new(request);
        match client.find_manifest(request).await {
            Ok(res) => return Ok(res.into_inner()),
            Err(e) => {
                bail!("error while calling find_manifest: {:?}", e);
            }
        };
    }
    pub async fn cert_sign(&self, request: CertSignRequest) -> Result<CertSignResponse> {
        let mut client = ProvisioningServiceClient::new(self.client.to_owned());
        let request = tonic::Request::new(request);
        match client.cert_sign(request).await {
            Ok(res) => return Ok(res.into_inner()),
            Err(e) => {
                bail!("error while calling find_manifest: {:?}", e);
            }
        };
    }
    pub async fn get_auth_nonce(&self, request: AuthNonceRequest) -> Result<AuthNonceResponse> {
        println!("get_auth_nonce*****: {:?}", request);
        let mut client = MessagingServiceClient::new(self.client.to_owned());
        let request = tonic::Request::new(request);
        match client.auth_nonce(request).await {
            Ok(res) => return Ok(res.into_inner()),
            Err(e) => {
                let e = anyhow::Error::from(e);
                bail!(e);
            }
        };
    }
    pub async fn get_auth_token(&self, request: IssueTokenRequest) -> Result<IssueTokenResponse> {
        let mut client = MessagingServiceClient::new(self.client.to_owned());
        let request = tonic::Request::new(request);
        match client.issue_token(request).await {
            Ok(res) => return Ok(res.into_inner()),
            Err(e) => {
                let e = anyhow::Error::from(e);
                bail!(e);
            }
        };
    }
}
