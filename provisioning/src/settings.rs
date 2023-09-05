use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProvisioningSettings {
    pub server_base_url: String,
    pub keys: Keys,
    pub openssl: OpenSsl,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenSsl {
    pub version: String,
    pub engine: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Keys {
    pub device: DeviceKeys,
    pub server: ServerKeys,
    pub intermediate: IntermediateKeys,
    pub root: RootKeys,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceKeys {
    pub privatekey: String,
    pub cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerKeys {
    pub cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IntermediateKeys {
    pub cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RootKeys {
    pub cert: String,
}
