// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;
use utils::{
    errors::ProvisioningErrorCodes,
    identity_manager::{GetMachineIdResponse, GetProvisionStatusResponse, IdentityClient},
    provision_manager::{
        PingResponse, ProvisionManagerClient, ProvisioningCodeResponse, ProvisioningStatusResponse,
    },
    settings_manager::{GetSettingsResponse, SettingsClient},
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(#[from] Box<dyn std::error::Error>),
    #[error("Other error: {0}")]
    Other(String),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

fn check_error(input: String) -> Result<String, ()> {
    let error_string = input.to_string().to_lowercase();

    if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::UnknownError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Unknown Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::UnauthorizedError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Unauthorized Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::NotFoundError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("NotFound Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::BadRequestError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Bad Request Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::UnreachableError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Unreachable Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::InternalServerError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Internal Server Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::CSRSignReadFileError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("CSRSign ReadFile Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::CertificateWriteError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("CertificateWrite Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::SendEventError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("SendEvent Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::SettingsDatabaseDeleteError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Settings Database Delete Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::ParseResponseError
            .to_string()
            .to_lowercase()
            .to_lowercase(),
    ) {
        Ok("Parse Response Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::ChannelSendMessageError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Channel Send Message Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::ChannelReceiveMessageError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Channel Receive Message Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::MachineMismatchError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Machine Mismatch Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::ExtractMessagePayloadError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Extract Message Payload Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::DeprovisioningError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Deprovisioning Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::SubscribeToNatsError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Subscribe ToNats Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::PayloadDeserializationError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("Payload Deserialization Error".to_string())
    } else if error_string.to_lowercase().contains(
        &ProvisioningErrorCodes::InvalidMachineIdError
            .to_string()
            .to_lowercase(),
    ) {
        Ok("InvalidMachineIdError Error".to_string())
    } else {
        Err(())
    }
}

#[tauri::command]
async fn get_ping_status() -> Result<PingResponse, Error> {
    let mut service_client = match ProvisionManagerClient::new().await {
        Ok(r) => r,
        Err(e) => {
            println!(
                "get_ping_status::service_client::init::error::  {:?} ",
                &e.to_string()
            );
            return Err(Error::Io(e.into()));
        }
    };

    let response = match service_client.ping().await {
        Ok(response) => response.into(),
        Err(e) => {
            println!(
                "get_ping_status::service_client::ping::error::  {:?} ",
                &e.to_string()
            );
            return Err(Error::Io(e.into()));
        }
    };

    Ok(response)
}

#[tauri::command]
async fn get_machine_provision_status() -> Result<GetProvisionStatusResponse, Error> {
    let mut service_client = match IdentityClient::new().await {
        Ok(r) => r,
        Err(e) => {
            println!("get_machine_provision_status error {:?}: ", e);
            return Err(Error::Io(e.into()));
        }
    };

    let response = match service_client.get_machine_provision_status().await {
        Ok(response) => response.into(),
        Err(e) => {
            return Err(Error::Io(e.into()));
        }
    };

    Ok(response)
}

#[tauri::command]
async fn generate_code() -> anyhow::Result<ProvisioningCodeResponse, Error> {
    let mut provision_manager_client = match ProvisionManagerClient::new().await {
        Ok(r) => r,
        Err(e) => {
            return Err(Error::Io(e.into()));
        }
    };

    let response = match provision_manager_client.generate_code().await {
        Ok(response) => response,
        Err(err) => {
            println!("generate_code error :: {:?} ", err);
            return Err(Error::Io(err.into()));
        }
    };

    Ok(response)
}

#[tauri::command]
async fn provision_code(code: String) -> Result<ProvisioningStatusResponse, Error> {
    println!("INSIDE provision_code: {:?}", code.to_string());
    let mut provision_manager_client = match ProvisionManagerClient::new().await {
        Ok(r) => r,
        Err(e) => {
            return Err(Error::Io(e.into()));
        }
    };

    let response = match provision_manager_client
        .provision_by_code(code.to_owned())
        .await
    {
        Ok(response) => response,
        Err(e) => {
            println!("provision_code error: {:?}", e);
            let error = check_error(e.to_string());
            println!("provision_code final error: {:?}", error);
            return Err(Error::Other(error.unwrap()));
        }
    };

    Ok(response)
}

#[tauri::command]
async fn get_machine_id() -> Result<GetMachineIdResponse, Error> {
    let mut service_client = match IdentityClient::new().await {
        Ok(r) => r,
        Err(e) => {
            return Err(Error::Io(e.into()));
        }
    };

    let response: GetMachineIdResponse = match service_client.getting_machine_id().await {
        Ok(response) => response.into(),
        Err(e) => {
            return Err(Error::Io(e.into()));
        }
    };

    Ok(response)
}

#[tauri::command]
async fn get_machine_info(key: String) -> Result<GetSettingsResponse, Error> {
    let request = SettingsClient::new().await;
    let mut service_client = match request {
        Ok(r) => r,
        Err(e) => {
            println!(
                "get_machine_info::service_client::new::error::  {:?} ",
                &e.to_string()
            );
            return Err(Error::Io(e.into()));
        }
    };

    let response: GetSettingsResponse = match service_client.get_settings_data(key.clone()).await {
        Ok(response) => response.into(),
        Err(e) => {
            println!(
                "get_machine_info::service_client::get_settings_data::error::  {:?} ",
                &e.to_string()
            );
            return Err(Error::Io(e.into()));
        }
    };
    Ok(response)
}

#[tauri::command]
fn exit_app() {
    std::process::exit(0x0);
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_ping_status,
            get_machine_provision_status,
            generate_code,
            provision_code,
            get_machine_id,
            get_machine_info,
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
