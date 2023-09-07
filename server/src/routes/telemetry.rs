use telemetry::service::{TelemetryErrorResponseCode, TelemetryService};

pub async fn send_user_metrics(
    content: String,
    telemetry_service: TelemetryService,
) -> Result<String, TelemetryErrorResponseCode> {
    let response = telemetry_service.user_metrics(content);
    match response {
        Ok(_v) => Ok("User metrics sent successfully".to_string()),
        Err(_e) => return Err(_e),
    }
}

pub async fn send_system_metrics(
    content: String,
    telemetry_service: TelemetryService,
) -> Result<String, TelemetryErrorResponseCode> {
    let response = telemetry_service.system_metrics(content);
    match response {
        Ok(_v) => Ok("System metrics sent successfully".to_string()),
        Err(_e) => return Err(_e),
    }
}
