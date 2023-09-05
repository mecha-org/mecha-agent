use std::fmt;

use crate::settings::MessagingAuthSettings;
use anyhow::Result;
use openssl::{base64, hash::MessageDigest, pkey::PKey, sign::Signer};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sentry_anyhow::capture_anyhow;
use serde::{Deserialize, Serialize};
use sha256::digest;
use tracing::error;
use tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthResponse {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Default, Clone, Copy)]
pub enum MessagingAuthErrorCodes {
    #[default]
    AuthCredentialGenerationError,
}

impl std::fmt::Display for MessagingAuthErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessagingAuthErrorCodes::AuthCredentialGenerationError => {
                write!(f, "Error while generating username and password")
            }
        }
    }
}

impl MessagingAuthServiceError {
    pub fn new(code: MessagingAuthErrorCodes, message: String, capture_error: bool) -> Self {
        let trace_id = find_current_trace_id();
        error!(
            target = "server",
            "Error: (code: {:?}, message: {})", code, message
        );
        if capture_error {
            let error = &anyhow::anyhow!(code).context(format!(
                "Error: (code: {:?}, messages: {} trace:{:?})",
                code, message, trace_id
            ));
            capture_anyhow(error);
        }
        Self { code, message }
    }
}
#[derive(Debug)]
pub struct MessagingAuthServiceError {
    pub code: MessagingAuthErrorCodes,
    pub message: String,
}

impl std::fmt::Display for MessagingAuthServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(code: {:?}, message: {})", self.code, self.message)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MessagingAuthService {
    settings: MessagingAuthSettings,
}

impl MessagingAuthService {
    pub fn new(settings: MessagingAuthSettings) -> Self {
        Self { settings: settings }
    }

    pub fn auth(&self) -> Result<AuthResponse> {
        let trace_id = find_current_trace_id();
        tracing::info!(trace_id, task = "auth", "init",);
        let username = generate_username().unwrap();
        let key_file_path = &self.settings.signed_key.privatekey;
        let password_value = generate_password(key_file_path, username.as_bytes());
        let password = match password_value {
            Ok(password) => password,
            Err(e) => {
                let error = MessagingAuthServiceError::new(
                    MessagingAuthErrorCodes::AuthCredentialGenerationError,
                    format!("Error: {:?}", e),
                    true,
                );
                return Err(anyhow::anyhow!(error));
            }
        };

        let auth_response = AuthResponse {
            username: username,
            password: password,
        };
        Ok(auth_response)
    }
}

fn generate_username() -> Result<String, Box<dyn std::error::Error>> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "generate_username", "init");
    let username: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let username_hash = digest(username);
    tracing::info!(
        trace_id,
        task = "generate_username",
        "username generated and hashed successfully"
    );
    Ok(username_hash)
}
fn generate_password(private_key_path: &str, data: &[u8]) -> Result<String> {
    let trace_id = find_current_trace_id();
    tracing::info!(trace_id, task = "generate_password", "init");
    let file_result = std::fs::read(private_key_path);
    let file = match file_result {
        Ok(file) => file,
        Err(e) => {
            tracing::error!(
                trace_id,
                task = "generate_password",
                "Error while reading private key file: {:?}",
                e
            );
            return Err(e.into());
        }
    };
    let private_key = PKey::private_key_from_pem(&file)?;
    let mut signer = Signer::new(MessageDigest::sha256(), &private_key)?;
    signer.update(data)?;
    let signature = signer.sign_to_vec()?;
    let base64_signature = base64::encode_block(&signature);
    tracing::info!(
        trace_id,
        task = "generate_password",
        "password generated and hashed successfully"
    );

    Ok(base64_signature)
}
