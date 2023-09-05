use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub success: bool,
    pub status: String,
    pub status_code: i16,
    pub message: Option<String>,
    pub error_code: Option<String>,
    pub sub_errors: Option<String>,
    pub payload: Value,
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self {
            success: true,
            status: String::from("OK"),
            status_code: 200,
            message: None,
            error_code: None,
            sub_errors: None,
            payload: json!({}),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub success: bool,
    pub status: String,
    pub status_code: i16,
    pub message: Option<String>,
    pub error_code: String,
    pub sub_errors: Option<String>,
    pub payload: Option<Value>,
}

impl Default for ErrorResponse {
    fn default() -> Self {
        Self {
            success: false,
            status: String::from("500 INTERNAL_SERVER_ERROR"),
            status_code: 500,
            error_code: String::from("500"),
            message: None,
            sub_errors: None,
            payload: None,
        }
    }
}
