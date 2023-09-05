use crate::server::{ErrorResponse, SuccessResponse};
use anyhow::Error;
use axum::{http::StatusCode, Json};
use messaging_auth::service::{MessagingAuthErrorCodes, MessagingAuthServiceError};
use provisioning::service::{ProvisioningErrorCodes, ProvisioningServiceError};

pub fn error_provisioning_handler(
    e: Error,
) -> Result<(StatusCode, Json<SuccessResponse>), Json<ErrorResponse>> {
    let default_error_response = ErrorResponse::default();
    match e.downcast_ref::<ProvisioningServiceError>() {
        Some(err) => {
            match err.code {
                ProvisioningErrorCodes::ManifestationNotFound => {
                    return Err(Json(ErrorResponse {
                        status: "400 BAD REQUEST".to_owned(),
                        status_code: 400,
                        error_code: "400".to_owned(),
                        message: Option::from("Manifestation not found".to_owned()),
                        ..default_error_response
                    }));
                }
                ProvisioningErrorCodes::CertificateGenerationFailed => {
                    return Err(Json(ErrorResponse {
                        status: "500 INTERNAL SERVER ERROR".to_owned(),
                        status_code: 500,
                        error_code: "500".to_owned(),
                        message: Option::from("Error while generating certificate".to_owned()),
                        ..default_error_response
                    }));
                }
                ProvisioningErrorCodes::CsrSignedFailed => {
                    return Err(Json(ErrorResponse {
                        status: "500 INTERNAL SERVER ERROR".to_owned(),
                        status_code: 500,
                        error_code: "500".to_owned(),
                        message: Option::from("Error while signing csr".to_owned()),
                        ..default_error_response
                    }));
                }
                ProvisioningErrorCodes::CertificateWriteFailed => {
                    return Err(Json(ErrorResponse {
                        status: "500 INTERNAL SERVER ERROR".to_owned(),
                        status_code: 500,
                        error_code: "500".to_owned(),
                        message: Option::from(
                            "Error while writing certificates to path specified in settings"
                                .to_owned(),
                        ),
                        ..default_error_response
                    }));
                }
                // Add more error code matches here as needed
                _ => {
                    return Err(Json(ErrorResponse {
                        message: Option::from("Encountered an unknown error".to_owned()),
                        ..default_error_response
                    }))
                }
            }
        }
        None => {
            return Err(Json(ErrorResponse {
                message: Option::from("Encountered an unknown error".to_owned()),
                ..default_error_response
            }))
        }
    }
}

pub fn error_messaging_auth(
    e: Error,
) -> Result<(StatusCode, Json<SuccessResponse>), Json<ErrorResponse>> {
    let default_error_response = ErrorResponse::default();
    match e.downcast_ref::<MessagingAuthServiceError>() {
        Some(err) => {
            match err.code {
                MessagingAuthErrorCodes::AuthCredentialGenerationError => {
                    return Err(Json(ErrorResponse {
                        status: "500 INTERNAL SERVER ERROR".to_owned(),
                        status_code: 500,
                        error_code: "500".to_owned(),
                        message: Option::from("Error while generating auth credentials".to_owned()),
                        ..default_error_response
                    }));
                }

                // Add more error code matches here as needed
                _ => {
                    return Err(Json(ErrorResponse {
                        message: Option::from("Encountered an unknown error".to_owned()),
                        ..default_error_response
                    }))
                }
            }
        }
        None => {
            return Err(Json(ErrorResponse {
                message: Option::from("Encountered an unknown error".to_owned()),
                ..default_error_response
            }))
        }
    }
}
