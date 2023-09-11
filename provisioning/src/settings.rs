use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProvisioningSettings {
    pub server_url: String,
    pub paths: CertificatePaths,
    pub openssl: OpenSslSettings,
}

impl Default for ProvisioningSettings {
    fn default() -> Self {
        Self {
            server_url: String::from("http://localhost:3000"),
            paths: CertificatePaths {
                device: DeviceCertificate {
                    private_key: String::from("~/.mecha/agent/.keys/device.key"),
                    csr: String::from("~/.mecha/agent/certs/device.csr"),
                    cert: String::from("~/.mecha/agent/certs/device.pem"),
                },
                server: ServerCertificate { cert: String::from("~/.mecha/agent/certs/server.pem") },
                intermediate: IntermediateCertificate { cert: String::from("~/.mecha/agent/certs/intermediate.pem") },
                root: RootCertificate { cert: String::from("~/.mecha/agent/certs/root.pem") }
            },
            openssl: OpenSslSettings { engine: None }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenSslSettings {
    pub engine: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CertificatePaths {
    pub device: DeviceCertificate,
    pub server: ServerCertificate,
    pub intermediate: IntermediateCertificate,
    pub root: RootCertificate,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceCertificate {
    pub private_key: String,
    pub csr: String,
    pub cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerCertificate {
    pub cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IntermediateCertificate {
    pub cert: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RootCertificate {
    pub cert: String,
}
