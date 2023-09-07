use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProvisioningSettings {
    pub keys: Keys,
    pub openssl: OpenSsl,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenSsl {
    version: String,
    engine: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Keys {
    device: DeviceKeys,
    server: ServerKeys,
    intermediate: IntermediateKeys,
    root: RootKeys,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceKeys {
    privatekey: String,
    cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerKeys {
    cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IntermediateKeys {
    cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RootKeys {
    cert: String,
}
