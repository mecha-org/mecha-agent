use agent_settings::constants;
use anyhow::{bail, Result};
use channel::recv_with_timeout;
use crypto::base64::b64_encode;
use crypto::x509::sign_with_private_key;
use events::Event;
use identity::handler::IdentityMessage;
use nats_client::jetstream::JetStreamClient;
use nats_client::{Bytes, NatsClient, Subscriber};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, error, info, trace};

use crate::errors::{MessagingError, MessagingErrorCodes};
use crate::handler::Settings;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
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
#[derive(Serialize)]
pub struct AuthNonceRequest {
    pub agent_name: String,
    pub agent_version: String,
}
#[derive(Clone)]
pub struct Messaging {
    nats_client: Option<NatsClient>,
    settings: Settings,
}
impl Messaging {
    pub fn new(initialize_client: bool, settings: Settings) -> Self {
        let nats_url = settings.messaging_url.clone();
        let nats_client = match initialize_client {
            true => Some(NatsClient::new(&nats_url)),
            false => None,
        };
        Self {
            nats_client,
            settings,
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
        event_type: events::MessagingEvent,
    ) -> Result<bool> {
        let fn_name = "connect";
        if self.nats_client.is_none() {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "messaging service initialized without nats client"
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
            ))
        }

        let machine_id = match get_machine_id(identity_tx.clone()).await {
            Ok(id) => id,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error getting machine id - {}",
                    e
                );
                bail!(e)
            }
        };

        debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "machine id - {}",
            machine_id
        );
        let data_dir = self.settings.data_dir.clone();
        let private_key_path = data_dir.to_owned() + constants::PRIVATE_KEY_PATH;
        let auth_token = match authenticate(
            &self.settings.service_url,
            &machine_id,
            &self.nats_client.as_ref().unwrap().user_public_key,
            &private_key_path,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error authenticating - {}",
                    e
                );
                bail!(e)
            }
        };
        let inbox_prefix = format!("inbox.{}", digest(&machine_id));
        match self
            .nats_client
            .as_mut()
            .unwrap()
            .connect(&auth_token, &inbox_prefix, event_tx.clone())
            .await
        {
            Ok(c) => c,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error connecting to nats - {}",
                    e
                );
                bail!(e)
            }
        };
        // Send broadcast message as messaging service is connected
        match event_tx.send(Event::Messaging(event_type)) {
            Ok(_) => {}
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error sending messaging service event - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::EventSendError,
                    format!("error sending messaging service event - {}", e),
                ));
            }
        }
        info!(
            func = fn_name,
            package = PACKAGE_NAME,
            "messaging service connected"
        );
        Ok(true)
    }
    pub async fn publish(
        &self,
        subject: &str,
        headers: Option<HashMap<String, String>>,
        data: Bytes,
    ) -> Result<bool> {
        let fn_name = "publish";
        debug!(
            func = fn_name,
            package = PACKAGE_NAME,
            "subject - {}",
            subject
        );
        if self.nats_client.is_none() {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "messaging service initialized without nats client"
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
            ))
        }

        let nats_client = self.nats_client.as_ref().unwrap();
        let is_published = match nats_client.publish(subject, headers, data).await {
            Ok(s) => s,
            Err(e) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "error publishing message - {}",
                    e
                );
                bail!(e)
            }
        };
        Ok(is_published)
    }

    pub async fn subscribe(&self, subject: &str) -> Result<Subscriber> {
        debug!(
            func = "subscribe",
            package = PACKAGE_NAME,
            "subject - {}",
            subject
        );

        if self.nats_client.is_none() {
            error!(
                func = "subscribe",
                package = PACKAGE_NAME,
                "messaging service initialized without nats client"
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
            ))
        }

        let nats_client = self.nats_client.as_ref().unwrap();
        let subscriber = match nats_client.subscribe(subject).await {
            Ok(s) => s,
            Err(e) => {
                error!(
                    func = "subscribe",
                    package = PACKAGE_NAME,
                    "error subscribing to subject - {}",
                    e
                );
                bail!(e)
            }
        };
        info!(
            fn_name = "subscribe",
            package = PACKAGE_NAME,
            "subscribed to subject - {}",
            subject
        );
        Ok(subscriber)
    }
    pub async fn init_jetstream(&self) -> Result<JetStreamClient> {
        if self.nats_client.is_none() {
            error!(
                func = "init_jetstream",
                package = PACKAGE_NAME,
                "messaging service initialized without nats client"
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
            ))
        }
        let nats_client = self.nats_client.as_ref().unwrap();
        let js_client = JetStreamClient::new(nats_client.client.clone().unwrap());
        info!(
            fn_name = "init_jetstream",
            package = PACKAGE_NAME,
            "initialized jetstream client"
        );
        Ok(js_client)
    }

    pub async fn request(&self, subject: &str, data: Bytes) -> Result<Bytes> {
        debug!(
            func = "request",
            package = PACKAGE_NAME,
            "subject - {}",
            subject
        );

        if self.nats_client.is_none() {
            error!(
                func = "request",
                package = PACKAGE_NAME,
                "messaging service initialized without nats client"
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::NatsClientNotInitialized,
                format!("messaging service initialized without nats client"),
            ))
        }

        let nats_client = self.nats_client.as_ref().unwrap();
        let response = match nats_client.request(subject, data).await {
            Ok(s) => s,
            Err(e) => {
                error!(
                    func = "request",
                    package = PACKAGE_NAME,
                    "error requesting message - {}",
                    e
                );
                bail!(e)
            }
        };
        info!(
            fn_name = "request",
            package = PACKAGE_NAME,
            "requested subject - {}",
            subject
        );
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
    service_url: &str,
    machine_id: &str,
    nats_client_public_key: &str,
    private_key_path: &str,
) -> Result<String> {
    let fn_name = "authenticate";
    let nonce_url = format!("{}{}", service_url, constants::NONCE_URL_QUERY_PATH);
    // Step 2: Get Nonce from Server
    let nonce = match get_auth_nonce(&nonce_url).await {
        Ok(n) => n,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting auth nonce - {}",
                e
            );
            bail!(e)
        }
    };
    debug!(func = fn_name, package = PACKAGE_NAME, "nonce - {}", nonce);
    // Step 3: Sign the nonce
    let signed_nonce = match sign_nonce(private_key_path, &nonce) {
        Ok(n) => n,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error signing nonce - {}",
                e
            );
            bail!(e)
        }
    };

    let token = match get_auth_token(
        service_url,
        MessagingScope::User,
        &machine_id,
        &nonce,
        &signed_nonce,
        &nats_client_public_key,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting auth token - {}",
                e
            );
            bail!(e)
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "authentication token obtained!"
    );
    println!("Auth token: -{:?}", token);
    Ok(token)
}

fn sign_nonce(private_key_path: &str, nonce: &str) -> Result<String> {
    let signed_nonce = match sign_with_private_key(&private_key_path, nonce.as_bytes()) {
        Ok(s) => s,
        Err(e) => {
            error!(
                func = "sign_nonce",
                package = PACKAGE_NAME,
                "error signing nonce with private key, path - {}, error - {}",
                private_key_path,
                e
            );
            bail!(e)
        }
    };

    let encoded_signed_nonce = b64_encode(signed_nonce);
    info!(
        func = "sign_nonce",
        package = PACKAGE_NAME,
        "nonce encoded!"
    );
    Ok(encoded_signed_nonce)
}

async fn get_auth_nonce(nonce_url: &str) -> Result<String> {
    let fn_name = "get_auth_nonce";
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "url - {}",
        nonce_url
    );
    // Construct request body
    let request_body: AuthNonceRequest = AuthNonceRequest {
        agent_name: "mecha_agent".to_string(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let client = reqwest::Client::new();
    let nonce_result = client
        .post(nonce_url)
        .json(&request_body)
        .header("CONTENT_TYPE", "application/json")
        .send()
        .await;

    let nonce_response = match nonce_result {
        Ok(nonce) => nonce,
        Err(e) => match e.status() {
            Some(StatusCode::INTERNAL_SERVER_ERROR) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth nonce returned internal server error - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthNonceServerError,
                    format!("get auth nonce returned internal server error - {}", e),
                ))
            }
            Some(StatusCode::BAD_REQUEST) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth nonce returned bad request - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthNonceBadRequestError,
                    format!("get auth nonce returned bad request - {}", e),
                ))
            }
            Some(StatusCode::NOT_FOUND) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth nonce returned not found - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthNonceNotFoundError,
                    format!("get auth nonce not found - {}", e),
                ))
            }
            Some(_) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth nonce returned unknown error - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth nonce returned unknown error - {}", e),
                ))
            }

            None => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth nonce returned unknown error - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth nonce returned error unmatched - {}", e),
                ))
            }
        },
    };

    // parse the manifest lookup result
    let nonce_response = match nonce_response
        .json::<MessagingServerResponseGeneric<String>>()
        .await
    {
        Ok(m) => m,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error parsing nonce response - {}",
                e
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::AuthNonceResponseParseError,
                format!("error parsing nonce response - {}", e),
            ))
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "auth nonce request completed"
    );
    Ok(nonce_response.payload)
}

async fn get_auth_token(
    service_url: &str,
    scope: MessagingScope,
    machine_id: &str,
    nonce: &str,
    signed_nonce: &str,
    nats_user_public_key: &str,
) -> Result<String> {
    let fn_name = "get_auth_token";
    let request_body = GetAuthTokenRequest {
        machine_id: machine_id.to_string(),
        _type: MessagingAuthTokenType::Device,
        scope: scope,
        nonce: nonce.to_string(),
        signed_nonce: signed_nonce.to_string(),
        public_key: nats_user_public_key.to_string(),
    };

    // Format the url to get the auth token
    let url = format!("{}{}", service_url, constants::ISSUE_TOKEN_URL_QUERY_PATH);
    debug!(
        func = fn_name,
        package = PACKAGE_NAME,
        "url - {}, request_body {:?}",
        url,
        request_body
    );
    let client = reqwest::Client::new();
    let get_auth_token_response = client
        .post(url)
        .header("CONTENT_TYPE", "application/json")
        .json(&request_body)
        .send()
        .await;

    // Check if the response is ok
    let auth_token_response = match get_auth_token_response {
        Ok(token) => token,
        Err(e) => match e.status() {
            Some(StatusCode::INTERNAL_SERVER_ERROR) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth token returned internal server error - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthTokenServerError,
                    format!("get auth token returned internal server error - {}", e),
                ))
            }
            Some(StatusCode::BAD_REQUEST) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth token returned bad request error - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthTokenBadRequestError,
                    format!("get auth token returned bad request error - {}", e),
                ))
            }
            Some(StatusCode::NOT_FOUND) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth token returned not found error - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::GetAuthTokenNotFoundError,
                    format!("get auth token returned not found error - {}", e),
                ))
            }
            Some(_) => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth token returned unknown error - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth token returned unknown error - {}", e),
                ))
            }

            None => {
                error!(
                    func = fn_name,
                    package = PACKAGE_NAME,
                    "get auth token returned error unmatched - {}",
                    e
                );
                bail!(MessagingError::new(
                    MessagingErrorCodes::UnknownError,
                    format!("get auth token returned error unmatched - {}", e),
                ))
            }
        },
    };

    // Parse the auth token result
    let auth_token = match auth_token_response
        .json::<MessagingServerResponseGeneric<String>>()
        .await
    {
        Ok(m) => m,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error while parsing auth token response - {}",
                e
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::AuthTokenResponseParseError,
                format!("error while parsing auth token response - {}", e),
            ))
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "auth token request completed"
    );
    Ok(auth_token.payload)
}

pub async fn get_machine_id(identity_tx: mpsc::Sender<IdentityMessage>) -> Result<String> {
    let (tx, rx) = oneshot::channel();
    match identity_tx
        .clone()
        .send(IdentityMessage::GetMachineId { reply_to: tx })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error sending get machine id message - {}",
                e
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::ChannelSendMessageError,
                format!("error sending get machine id message - {}", e),
            ));
        }
    }
    let machine_id = match recv_with_timeout(rx).await {
        Ok(id) => id,
        Err(e) => {
            error!(
                func = "get_machine_id",
                package = PACKAGE_NAME,
                "error receiving get machine id message - {}",
                e
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::ChannelReceiveMessageError,
                format!("error receiving get machine id message - {}", e),
            ));
        }
    };
    info!(
        func = "get_machine_id",
        package = PACKAGE_NAME,
        "get machine id request completed",
    );
    Ok(machine_id)
}
