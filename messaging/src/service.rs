use agent_settings::read_settings_yml;
use agent_settings::{messaging::MessagingSettings, AgentSettings};
use anyhow::{bail, Result};
use crypto::base64::b64_encode;
use crypto::x509::sign_with_private_key;
use events::Event;
use identity::handler::IdentityMessage;
use nats_client::jetstream::JetStreamClient;
use nats_client::{Bytes, NatsClient, Subscriber};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sha256::digest;
use tokio::sync::{broadcast, mpsc, oneshot};
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
    machine_id: String,
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
        let settings = match read_settings_yml() {
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
    pub async fn connect(
        &mut self,
        identity_tx: &mpsc::Sender<IdentityMessage>,
        event_tx: broadcast::Sender<Event>,
    ) -> Result<bool> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "connect", "init");
        let (tx, rx) = oneshot::channel();
        if self.nats_client.is_none() {
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
                true
            ))
        }
        // let (identity_tx, identity_rx) = oneshot::channel();
        let _ = identity_tx
            .clone()
            .send(IdentityMessage::GetProvisionStatus { reply_to: tx })
            .await;
        match rx.await {
            Ok(provision_status_result) => {
                if provision_status_result.is_ok() {
                    match provision_status_result {
                        Ok(result) => {
                            if !result {
                                return Ok(false);
                            }
                        }
                        Err(err) => {
                            bail!(err);
                        }
                    }
                } else {
                    bail!("Error getting provision status");
                }
            }
            Err(err) => {
                bail!("Error getting provision status: {:?}", err);
            }
        }
        let machine_id = match get_machine_id(identity_tx.clone()).await {
            Ok(id) => id,
            Err(e) => bail!(e),
        };

        let auth_token = match authenticate(
            &self.settings,
            &machine_id,
            &self.nats_client.as_ref().unwrap().user_public_key,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => bail!(e),
        };
        let inbox_prefix = format!("inbox.{}", digest(&machine_id));
        match self
            .nats_client
            .as_mut()
            .unwrap()
            .connect(&auth_token, &inbox_prefix)
            .await
        {
            Ok(c) => c,
            Err(e) => bail!(e),
        };
        // Send broadcast message as messaging service is connected
        let _ = event_tx.send(Event::Messaging(events::MessagingEvent::Connected));
        Ok(true)
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
            Ok(s) => {
                println!("published to subject: {}", subject);
                s
            }
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
    pub async fn init_jetstream(&self) -> Result<JetStreamClient> {
        if self.nats_client.is_none() {
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
                true
            ))
        }
        let nats_client = self.nats_client.as_ref().unwrap();
        let js_client = JetStreamClient::new(nats_client.client.clone().unwrap());
        Ok(js_client)
    }

    pub async fn request(&self, subject: &str, data: Bytes) -> Result<Bytes> {
        let trace_id = find_current_trace_id();
        tracing::trace!(trace_id, task = "request", "init");

        if self.nats_client.is_none() {
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
                true
            ))
        }

        let nats_client = self.nats_client.as_ref().unwrap();
        let response = match nats_client.request(subject, data).await {
            Ok(s) => s,
            Err(e) => bail!(e),
        };

        Ok(response)
    }
}
/**
 * Performs messaging authentication and returns the JWT. Auth procedure is
 * 1. Requests nonce from the server
 * 2. Signs the nonce using the Device Key
 * 3. Requests the token from the server
 */
pub async fn authenticate(
    settings: &AgentSettings,
    machine_id: &String,
    nats_client_public_key: &String,
) -> Result<String> {
    // Step 2: Get Nonce from Server
    let nonce = match get_auth_nonce(&settings.messaging).await {
        Ok(n) => n,
        Err(e) => bail!(e),
    };

    // Step 3: Sign the nonce
    let nonce_sign = match sign_nonce(&settings.provisioning.paths.device.private_key, &nonce) {
        Ok(n) => n,
        Err(e) => bail!(e),
    };

    let token = match get_auth_token(
        MessagingScope::User,
        &machine_id,
        &nonce,
        &nonce_sign,
        &nats_client_public_key,
        &settings.messaging,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => bail!(e),
    };
    Ok(token)
}

fn sign_nonce(private_key_path: &String, nonce: &str) -> Result<String> {
    let signed_nonce = match sign_with_private_key(&private_key_path, nonce.as_bytes()) {
        Ok(s) => s,
        Err(e) => bail!(e),
    };

    let encoded_signed_nonce = b64_encode(signed_nonce);
    Ok(encoded_signed_nonce)
}

async fn get_auth_nonce(settings: &MessagingSettings) -> Result<String> {
    let trace_id = find_current_trace_id();
    tracing::trace!(trace_id, task = "request_nonce", "init");
    let url = format!(
        "{}{}",
        &settings.service_urls.base_url, &settings.service_urls.get_nonce
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
    scope: MessagingScope,
    machine_id: &str,
    nonce: &str,
    signed_nonce: &str,
    nats_user_public_key: &str,
    settings: &MessagingSettings,
) -> Result<String> {
    let request_body = GetAuthTokenRequest {
        machine_id: machine_id.to_string(),
        _type: MessagingAuthTokenType::Device,
        scope: scope,
        nonce: nonce.to_string(),
        signed_nonce: signed_nonce.to_string(),
        public_key: nats_user_public_key.to_string(),
    };

    let url = format!(
        "{}{}",
        &settings.service_urls.base_url, &settings.service_urls.issue_auth_token
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

pub async fn get_machine_id(identity_tx: mpsc::Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    let _ = identity_tx
        .clone()
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await;
    let mut machine_id = String::new();
    match rx.await {
        Ok(provision_status_result) => {
            if provision_status_result.is_ok() {
                match provision_status_result {
                    Ok(result) => machine_id = result,
                    Err(err) => {
                        bail!(err);
                    }
                }
            } else {
                bail!("Error getting machine id");
            }
        }
        Err(err) => {
            bail!("Error getting machine id: {:?}", err);
        }
    }
    Ok(machine_id)
}
