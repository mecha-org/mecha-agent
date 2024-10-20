use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum AppServicesErrorCodes {
    #[default]
    UnknownError,
    RequestPayloadParseError,
    SendReconnectMessagingMessageError,
    RecvReconnectMessageError,
    GetSubscriberSendMessageError,
    RecvSubscriberError,
    ReqIdParseError,
    ServiceSettingsParseError,
    MessageHeaderEmptyError,
    AckHeaderNotFoundError,
    PortParseError,
    ChannelSendGetSubscriberMessageError,
    ChannelReceiveSubscriberMessageError,
    TcpStreamConnectError,
    TcpStreamConnectTimeoutError,
}

impl fmt::Display for AppServicesErrorCodes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppServicesErrorCodes::UnknownError => {
                write!(f, "AppServicesErrorCodes: UnknownError")
            }
            AppServicesErrorCodes::AckHeaderNotFoundError => {
                write!(f, "AppServicesErrorCodes: AckHeaderNotFoundError")
            }
            AppServicesErrorCodes::MessageHeaderEmptyError => {
                write!(f, "AppServicesErrorCodes: MessageHeaderEmptyError")
            }
            AppServicesErrorCodes::SendReconnectMessagingMessageError => {
                write!(
                    f,
                    "AppServicesErrorCodes: SendReconnectMessagingMessageError"
                )
            }
            AppServicesErrorCodes::RecvReconnectMessageError => {
                write!(f, "AppServicesErrorCodes: RecvReconnectMessageError")
            }
            AppServicesErrorCodes::RequestPayloadParseError => {
                write!(f, "AppServicesErrorCodes: RequestPayloadParseError")
            }
            AppServicesErrorCodes::GetSubscriberSendMessageError => {
                write!(f, "AppServicesErrorCodes: GetSubscriberSendMessageError")
            }
            AppServicesErrorCodes::RecvSubscriberError => {
                write!(f, "AppServicesErrorCodes: RecvSubscriberError")
            }
            AppServicesErrorCodes::ReqIdParseError => {
                write!(f, "AppServicesErrorCodes: ReqIdParseError")
            }
            AppServicesErrorCodes::ServiceSettingsParseError => {
                write!(f, "AppServicesErrorCodes: ServiceSettingsParseError")
            }
            AppServicesErrorCodes::PortParseError => {
                write!(f, "AppServicesErrorCodes: PortParseError")
            }
            AppServicesErrorCodes::ChannelSendGetSubscriberMessageError => {
                write!(
                    f,
                    "AppServicesErrorCodes: ChannelSendGetSubscriberMessageError"
                )
            }
            AppServicesErrorCodes::ChannelReceiveSubscriberMessageError => {
                write!(
                    f,
                    "AppServicesErrorCodes: ChannelReceiveSubscriberMessageError"
                )
            }
            AppServicesErrorCodes::TcpStreamConnectError => {
                write!(f, "AppServicesErrorCodes: TcpStreamConnectError")
            }
            AppServicesErrorCodes::TcpStreamConnectTimeoutError => {
                write!(f, "AppServicesErrorCodes: TcpStreamConnectTimeoutError")
            }
        }
    }
}

#[derive(Debug)]
pub struct AppServicesError {
    pub code: AppServicesErrorCodes,
    pub message: String,
}

impl std::fmt::Display for AppServicesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AppServicesErrorCodes:(code: {:?}, message: {})",
            self.code, self.message
        )
    }
}

impl AppServicesError {
    pub fn new(code: AppServicesErrorCodes, message: String) -> Self {
        Self { code, message }
    }
}
