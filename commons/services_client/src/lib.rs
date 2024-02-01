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
use tracing::{info, warn};
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
    pub async fn new() -> Result<Self> {
        let settings = match read_settings_yml() {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    func = "new",
                    package = env!("CARGO_PKG_NAME"),
                    "error reading settings - {}",
                    e
                );
                AgentSettings::default()
            }
        };
        let channel = Channel::from_shared(settings.services.url).unwrap();
        let client = match channel.connect().await {
            Ok(c) => c,
            Err(e) => {
                let e = anyhow::Error::from(e);
                bail!(e);
            }
        };
        Ok(Self { client })
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
        let fn_name = "find_manifest";
        info!(
            func = fn_name,
            package = env!("CARGO_PKG_NAME"),
            "request: {:?}",
            request
        );
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
