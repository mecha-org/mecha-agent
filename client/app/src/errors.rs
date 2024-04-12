use std::fmt;

use tracing::error;

/// # connect screen error codes
///
/// Implements standard errors for the connect screens
#[derive(Debug, Default, Clone, Copy)]
pub enum ScreenErrorCodes {
    #[default]
    UnknownError,
    SettingsReadError,
    SettingsParseError,
    ThemeReadError,
    ThemeParseError,
    FindLoginManagerUrlError,
    LoginManagerStreamConnectError,
    StreamWriteUsernameError,
    StreamReadEnterPasswordError,
    StreamWritePasswordError,
    StreamReadCaptchaError,
    StreamWriteCaptchaError,
    StreamReadAuthResponseError,
}

impl fmt::Display for ScreenErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScreenErrorCodes::UnknownError => write!(f, "UnknownError"),
            ScreenErrorCodes::SettingsReadError => write!(f, "SettingsReadError"),
            ScreenErrorCodes::SettingsParseError => write!(f, "SettingsParseError"),
            ScreenErrorCodes::ThemeReadError => write!(f, "ThemeReadError"),
            ScreenErrorCodes::ThemeParseError => write!(f, "ThemeParseError"),
            ScreenErrorCodes::FindLoginManagerUrlError => write!(f, "FindLoginManagerUrlError"),
            ScreenErrorCodes::LoginManagerStreamConnectError => {
                write!(f, "LoginManagerStreamConnectError")
            }
            ScreenErrorCodes::StreamWriteUsernameError => {
                write!(f, "StreamWriteUsernameError")
            }
            ScreenErrorCodes::StreamReadEnterPasswordError => {
                write!(f, "StreamReadEnterPasswordError")
            }
            ScreenErrorCodes::StreamWritePasswordError => {
                write!(f, "StreamWritePasswordError")
            }
            ScreenErrorCodes::StreamReadCaptchaError => {
                write!(f, "StreamReadCaptchaError")
            }
            ScreenErrorCodes::StreamWriteCaptchaError => {
                write!(f, "StreamWriteCaptchaError")
            }
            ScreenErrorCodes::StreamReadAuthResponseError => {
                write!(f, "StreamReadAuthResponseError")
            }
        }
    }
}

/// # ScreenError
///
/// Implements a standard error type for all connect screen related errors
/// includes the error code (`ScreenErrorCodes`) and a message
#[derive(Debug, Default)]
pub struct ScreenError {
    pub code: ScreenErrorCodes,
    pub message: String,
}

impl ScreenError {
    pub fn new(code: ScreenErrorCodes, message: String) -> Self {
        error!("error: (code: {:?}, message: {})", code, message);
        Self { code, message }
    }
}

impl std::fmt::Display for ScreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(code: {:?}, message: {})", self.code, self.message)
    }
}
