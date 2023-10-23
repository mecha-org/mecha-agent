use crate::agent::{
    device_setting_service_server::DeviceSettingService, DeviceSettingResponse, GetSettingRequest,
};
use anyhow::Result;
use device_settings::services::DeviceSettings;
use settings::{read_settings_yml, AgentSettings};
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct DeviceSettingServiceHandler {}

fn new_device_setting_service() -> DeviceSettings {
    let settings: AgentSettings = match read_settings_yml() {
        Ok(settings) => settings,
        Err(_) => AgentSettings::default(),
    };

    DeviceSettings::new(settings)
}

#[tonic::async_trait]
impl DeviceSettingService for DeviceSettingServiceHandler {
    async fn get_settings(
        &self,
        request: Request<GetSettingRequest>,
    ) -> Result<Response<DeviceSettingResponse>, Status> {
        let device_setting_service = new_device_setting_service();
        let settings_key = &request.into_inner().key;
        let settings = device_setting_service
            .get_settings(settings_key.to_string())
            .await;

        match settings {
            Ok(v) => Ok(Response::new(DeviceSettingResponse { code: v })),
            Err(err) => Err(Status::from_error(err.into())),
        }
    }
}
