use crate::{server::{identity_client::{GetMachineCertResponse, GetMachineIdResponse, IdentityClient}, settings_client::{GetSettingsResponse, SettingsClient}}, settings::{self, PagesSettings}};
use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub struct MachineInformation {
    pub machine_id: String,
    pub name: String,
    pub icon: Option<String>,
}
#[derive(Debug, Clone)]
pub struct MachineInformationResponse {
    pub machine_information : MachineInformation,
}

impl MachineInformation {
    pub fn new() -> Self {
        Self {
            machine_id: String::from(""),
            name: String::from(""),
            icon: Some(String::from("")),
        }
    }
}

pub async fn get_machine_id() -> Result<GetMachineIdResponse> {
    let request = IdentityClient::new().await;

    let mut service_client = match request {
        Ok(r) => r,
        Err(e) => {
            bail!(e);
        }
    };
        
    let response: GetMachineIdResponse = match service_client.getting_machine_id().await {
        Ok(response) => {
            response.into()
        },
        Err(e) => {
            bail!(e);
        },
    };

    Ok(response)
}

pub async fn get_machine_name_or_icon(key: String) -> Result<GetSettingsResponse> {
    let request = SettingsClient::new().await;

    let mut service_client = match request {
        Ok(r) => r,
        Err(e) => {
            bail!(e);
        }
    };
        
    let response: GetSettingsResponse = match service_client.get_settings_data(key).await {
        Ok(response) => {
            response.into()
        },
        Err(e) => {
            bail!(e);
        },
    };

    Ok(response)
}

pub async fn get_machine_cert_details() -> Result<GetMachineCertResponse>  {
    let identity_client_response = IdentityClient::new().await;
    let mut identity_client: IdentityClient = match identity_client_response {
        Ok(result) => {
            println!("identity_client_response : {:?} ", result);
            result.into()
        },
        Err(e) => {
            bail!(e);
        },
        
    };

    let response : GetMachineCertResponse = match identity_client.get_machine_cert_details().await {
        Ok(response) => {
            response.into()
        },
        Err(e) => {
            bail!(e);
        }
    };   
    Ok(response)
}

pub async fn get_machine_info() -> Result<MachineInformation> {
 
    let machine_id_response = get_machine_id().await;

    let machine_name_response = get_machine_name_or_icon(String::from("identity.machine.name")).await;

    let machine_icon_response = get_machine_name_or_icon(String::from("identity.machine.icon")).await;

    // let machine_cert_details = get_machine_cert_details().await;

    let response = MachineInformation {
        machine_id: match machine_id_response{
            Ok(resp) => {
                let mut machine_id= String::from("-");
                if resp.machine_id != "" { machine_id = resp.machine_id }
                machine_id
            },
            Err(e) => {
                bail!(e);
            },
        },
        name: match machine_name_response {
            Ok(resp) => {
                let mut machine_name= String::from("-");
                if resp.value != "" { machine_name = resp.value }
                machine_name
            },
            Err(e) => {
                bail!(e);
            }
        },
        icon: match machine_icon_response {
            Ok(resp) => {
                let mut machine_icon = PagesSettings::default().device_info.user_profile_img;
                if resp.value != "" { machine_icon = Some(resp.value) }
                machine_icon
            },
            Err(e) => {
                bail!(e);
            }
        }
    };
    Ok(response)
}