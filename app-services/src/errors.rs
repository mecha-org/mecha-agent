use std::fmt;

#[derive(Debug, Default, Clone, Copy)]
pub enum AppServicesErrorCodes {
    #[default]
    UnknownError,
    PullMessagesError,
    CreateConsumerError,
    ChannelSendMessageError,
    ChannelReceiveMessageError,
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
            AppServicesErrorCodes::PullMessagesError => {
                write!(f, "AppServicesErrorCodes: PullMessagesError")
            }
            AppServicesErrorCodes::CreateConsumerError => {
                write!(f, "AppServicesErrorCodes: CreateConsumerError")
            }
            AppServicesErrorCodes::ChannelSendMessageError => {
                write!(f, "AppServicesErrorCodes: ChannelSendMessageError",)
            }
            AppServicesErrorCodes::ChannelReceiveMessageError => {
                write!(f, "AppServicesErrorCodes: ChannelReceiveMessageError")
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
