use agent_settings::read_settings_yml;
use agent_settings::{messaging::MessagingSettings, AgentSettings};
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
use services_client::messaging::{AuthNonceRequest, AuthNonceResponse, IssueTokenRequest};
use services_client::ServicesClient;
use sha256::digest;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, error, info, trace};

use crate::errors::{MessagingError, MessagingErrorCodes};
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

#[derive(Clone)]
pub struct Messaging {
    settings: AgentSettings,
    nats_client: Option<NatsClient>,
}
impl Messaging {
    pub fn new(initialize_client: bool) -> Self {
        let settings = match read_settings_yml() {
            Ok(settings) => settings,
            Err(_) => AgentSettings::default(),
        };
        let nats_url = settings.messaging.system.url.clone();
        let nats_client = match initialize_client {
            true => Some(NatsClient::new(&nats_url)),
            false => None,
        };
        Self {
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
                false
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
        let auth_token = match authenticate(
            &self.settings,
            &machine_id,
            &self.nats_client.as_ref().unwrap().user_public_key,
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
        match event_tx.send(Event::Messaging(events::MessagingEvent::Connected)) {
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
                    true
                ));
            }
        }

        Ok(true)
    }
    pub async fn publish(&self, subject: &str, data: Bytes) -> Result<bool> {
        let fn_name = "publish";
        trace!(
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
                true,
            ))
        }

        let nats_client = self.nats_client.as_ref().unwrap();
        let is_published = match nats_client.publish(subject, data).await {
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
        trace!(
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
                false
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
                false
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
        trace!(
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
                false
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
    settings: &AgentSettings,
    machine_id: &String,
    nats_client_public_key: &String,
) -> Result<String> {
    let fn_name = "authenticate";
    // Step 2: Get Nonce from Server
    let nonce = match get_auth_nonce(&settings.messaging).await {
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
    let signed_nonce = match sign_nonce(&settings.provisioning.paths.machine.private_key, &nonce) {
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
        &machine_id,
        &nonce,
        &signed_nonce,
        &nats_client_public_key,
        &settings.messaging,
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
    Ok(token)
}

fn sign_nonce(private_key_path: &String, nonce: &str) -> Result<String> {
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
    Ok(encoded_signed_nonce)
}

async fn get_auth_nonce(settings: &MessagingSettings) -> Result<String> {
    let fn_name = "get_auth_nonce";
    let client = ServicesClient::new().await?;
    let result = match client
        .get_auth_nonce(AuthNonceRequest {
            agent_name: String::from("mecha_agent"),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting auth nonce - {}",
                e
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::GetAuthNonceError,
                format!("get auth nonce error - {}", e),
                true
            ))
        }
    };

    info!(
        func = "get_auth_nonce",
        package = PACKAGE_NAME,
        "auth nonce request completed"
    );
    Ok(result.nonce)
}

async fn get_auth_token(
    machine_id: &str,
    nonce: &str,
    signed_nonce: &str,
    nats_user_public_key: &str,
    settings: &MessagingSettings,
) -> Result<String> {
    let fn_name = "get_auth_token";
    let request_body = IssueTokenRequest {
        machine_id: machine_id.to_string(),
        r#type: String::from("Machine"),
        scope: String::from("User"),
        nonce: nonce.to_string(),
        signed_nonce: signed_nonce.to_string(),
        public_key: nats_user_public_key.to_string(),
    };
    let client = ServicesClient::new().await?;
    let result = match client.get_auth_token(request_body).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = fn_name,
                package = PACKAGE_NAME,
                "error getting auth token - {}",
                e
            );
            bail!(MessagingError::new(
                MessagingErrorCodes::GetAuthNonceError,
                format!("get auth token error - {}", e),
                true
            ))
        }
    };
    info!(
        func = fn_name,
        package = PACKAGE_NAME,
        "auth token request completed"
    );
    Ok(result.token)
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
                true
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
                true
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
