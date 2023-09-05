use axum::{http::StatusCode, Json};
use messaging_auth::service::MessagingAuthService;
use serde_json::json;

use crate::{
    errors::error_messaging_auth,
    server::{ErrorResponse, SuccessResponse},
    settings::read_yml,
};

pub async fn auth_handler() -> Result<(StatusCode, Json<SuccessResponse>), Json<ErrorResponse>> {
    let settings = read_yml().unwrap();
    let provisioning_service = MessagingAuthService::new(settings.messaging_auth);
    let response = provisioning_service.auth();
    match response {
        Ok(v) => Ok((
            StatusCode::OK,
            Json(SuccessResponse {
                success: true,
                payload: json!(v),
                ..Default::default()
            }),
        )),
        Err(e) => error_messaging_auth(e),
    }
}
