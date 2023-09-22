use anyhow::{bail, Result};
use crypto::base64::b64_encode;
use crypto::x509::{get_subject_name, sign_with_private_key};
use nats_client::{Bytes, NatsClient, Subscriber};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use settings::AgentSettings;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

use crate::errors::{MessagingError, MessagingErrorCodes};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessagingServerResponseGeneric<T> {
    pub success: bool,
    pub status: String,
    pub status_code: i16,
    pub message: Option<String>,
    pub error_code: Option<String>,
    pub sub_errors: Option<String>,
    pub payload: T,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessagingAuthTokenRequest {
    id: String,
    #[serde(rename = "type")]
    _type: MessagingAuthTokenType,
    scope: MessagingScope,
    nonce: String,
    signed_noce: String,
    public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessagingScope {
    #[serde(rename = "sys")]
    System,
    #[serde(rename = "user")]
    User,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessagingAuthTokenType {
    #[serde(rename = "device")]
    Device,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetAuthTokenRequest {
    id: String,
    #[serde(rename = "type")]
    _type: MessagingAuthTokenType,
    scope: MessagingScope,
    nonce: String,
    signed_nonce: String,
    public_key: String,
}
#[derive(Clone)]
pub struct Messaging {
    scope: MessagingScope,
    settings: AgentSettings,
    nats_client: Option<NatsClient>,
}

impl Messaging {
    pub fn new(scope: MessagingScope, initialize_client: bool) -> Self {
        let settings = match settings::read_settings_yml() {
            Ok(settings) => settings,
            Err(_) => AgentSettings::default(),
        };
        let nats_url = match scope {
            MessagingScope::System => settings.messaging.system.url.clone(),
            MessagingScope::User => settings.messaging.user.url.clone(),
        };
        let nats_client = match initialize_client {
            true => Some(NatsClient::new(&nats_url)),
            false => None,
        };
        Self {
            scope,
            settings,
            nats_client,
        }
    }

    /**
     * The Messaging Service connection will perform the following
     * 1. Authenticate
     * 2. Create NATs Client
     * 3. Connect NATs client with token
     * 4. Check for connection event, re-connect if necessary
     */
    pub async fn connect(&mut self) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "connect", "init");

        if self.nats_client.is_none() {
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
                true
            ))
        }

        let auth_token = match self.authenticate().await {
            Ok(t) => t,
            Err(e) => bail!(e),
        };

        match self
            .nats_client
            .as_mut()
            .unwrap()
            .connect(&auth_token)
            .await
        {
            Ok(c) => c,
            Err(e) => bail!(e),
        };

        Ok(true)
    }

    /**
     * Performs messaging authentication and returns the JWT. Auth procedure is
     * 1. Requests nonce from the server
     * 2. Signs the nonce using the Device Key
     * 3. Requests the token from the server
     */
    async fn authenticate(&self) -> Result<String> {
        // Step 1: Get Device ID
        //TODO: Path Check
        let device_id = match get_subject_name("device.pem") {
            Ok(s) => s,
            Err(e) => bail!(e),
        };

        // Step 2: Get Nonce from Server
        let nonce = match self.get_auth_nonce().await {
            Ok(n) => n,
            Err(e) => bail!(e),
        };

        // Step 3: Sign the nonce
        let nonce_sign = match self.sign_nonce(&nonce) {
            Ok(n) => n,
            Err(e) => bail!(e),
        };

        // Step 4: Get NATS nkey public key
        let nats_client_public_key = &self.nats_client.as_ref().unwrap().user_public_key;
        let token = match self
            .get_auth_token(
                self.scope.clone(),
                &device_id,
                &nonce,
                &nonce_sign,
                &nats_client_public_key,
            )
            .await
        {
            Ok(t) => t,
            Err(e) => bail!(e),
        };
        Ok(token)
    }

    fn sign_nonce(&self, nonce: &str) -> Result<String> {
        let private_key_path = &self.settings.provisioning.paths.device.private_key;
        let signed_nonce = match sign_with_private_key(&private_key_path, nonce.as_bytes()) {
            Ok(s) => s,
            Err(e) => bail!(e),
        };

        let encoded_signed_nonce = b64_encode(signed_nonce);
        Ok(encoded_signed_nonce)
    }

    async fn get_auth_nonce(&self) -> Result<String> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "request_nonce", "init");
        let url = format!(
            "{}{}",
            &self.settings.messaging.service_urls.base_url,
            &self.settings.messaging.service_urls.get_nonce
        );
        let client = reqwest::Client::new();
        let nonce_result = client
            .get(url)
            .header("CONTENT_TYPE", "application/json")
            .send()
            .await;

        let nonce_response = match nonce_result {
            Ok(nonce) => nonce,
            Err(e) => match e.status() {
                Some(StatusCode::INTERNAL_SERVER_ERROR) => bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth nonce returned server error - {}", e),
                    true
                )),
                Some(StatusCode::BAD_REQUEST) => bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthNonceBadRequestError,
                    format!("get auth nonce returned bad request - {}", e),
                    true // Not reporting bad request errors
                )),
                Some(StatusCode::NOT_FOUND) => bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthNonceNotFoundError,
                    format!("get auth nonce not found - {}", e),
                    false // Not reporting not found errors
                )),
                Some(_) => bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth nonce returned unknown error - {}", e),
                    true
                )),

                None => bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth nonce returned unknown error - {}", e),
                    true
                )),
            },
        };

        // parse the manifest lookup result
        let manifest_response = match nonce_response
            .json::<MessagingServerResponseGeneric<String>>()
            .await
        {
            Ok(m) => m,
            Err(e) => bail!(MessagingError::new(
                MessagingErrorCodes::AuthNonceResponseParseError,
                format!("error parsing lookup manifest response - {}", e),
                true
            )),
        };

        Ok(manifest_response.payload)
    }

    async fn get_auth_token(
        &self,
        scope: MessagingScope,
        id: &str,
        nonce: &str,
        signed_nonce: &str,
        nats_user_public_key: &str,
    ) -> Result<String> {
        let request_body = GetAuthTokenRequest {
            id: id.to_string(),
            _type: MessagingAuthTokenType::Device,
            scope: scope,
            nonce: nonce.to_string(),
            signed_nonce: signed_nonce.to_string(),
            public_key: nats_user_public_key.to_string(),
        };

        let url = format!(
            "{}{}",
            &self.settings.messaging.service_urls.base_url,
            &self.settings.messaging.service_urls.issue_auth_token
        );
        let client = reqwest::Client::new();
        let get_auth_token_response = client
            .post(url)
            .header("CONTENT_TYPE", "application/json")
            .json(&request_body)
            .send()
            .await;

        let auth_token_response = match get_auth_token_response {
            Ok(token) => token,
            Err(e) => match e.status() {
                Some(StatusCode::INTERNAL_SERVER_ERROR) => bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth nonce returned server error - {}", e),
                    true
                )),
                Some(StatusCode::BAD_REQUEST) => bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthNonceBadRequestError,
                    format!("get auth nonce returned bad request - {}", e),
                    true // Not reporting bad request errors
                )),
                Some(StatusCode::NOT_FOUND) => bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthNonceNotFoundError,
                    format!("get auth nonce not found - {}", e),
                    false // Not reporting not found errors
                )),
                Some(_) => bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth nonce returned unknown error - {}", e),
                    true
                )),

                None => bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth nonce returned unknown error - {}", e),
                    true
                )),
            },
        };
        // parse the auth token result
        match auth_token_response
            .json::<MessagingServerResponseGeneric<String>>()
            .await
        {
            Ok(m) => return Ok(m.payload),
            Err(e) => bail!(MessagingError::new(
                MessagingErrorCodes::AuthTokenResponseParseError,
                format!("error while parsing auth token response - {}", e),
                true
            )),
        };
    }

    pub async fn publish(&self, subject: &str, data: Bytes) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "connect", "init");

        if self.nats_client.is_none() {
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
                true
            ))
        }

        let nats_client = self.nats_client.as_ref().unwrap();
        let is_published = match nats_client.publish(subject, data).await {
            Ok(s) => s,
            Err(e) => bail!(e),
        };

        Ok(is_published)
    }

    pub async fn subscribe(&self, subject: &str) -> Result<Subscriber> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "connect", "init");

        if self.nats_client.is_none() {
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
                true
            ))
        }

        let nats_client = self.nats_client.as_ref().unwrap();
        let subscriber = match nats_client.subscribe(subject).await {
            Ok(s) => s,
            Err(e) => bail!(e),
        };

        Ok(subscriber)
    }
}
