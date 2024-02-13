use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
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
                machine: MachineCertificate {
                    private_key: String::from("~/.mecha/agent/.keys/machine.key"),
                    csr: String::from("~/.mecha/agent/certs/machine.csr"),
                    cert: String::from("~/.mecha/agent/certs/machine.pem"),
                },
                server: ServerCertificate {
                    cert: String::from("~/.mecha/agent/certs/server.pem"),
                },
                ca_bundle: CABundle {
                    cert: String::from("~/.mecha/agent/certs/ca_bundle.pem"),
                },
                root: RootCertificate {
                    cert: String::from("~/.mecha/agent/certs/root.pem"),
                },
            },
            openssl: OpenSslSettings { engine: None },
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OpenSslSettings {
    pub engine: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CertificatePaths {
    pub machine: MachineCertificate,
    pub server: ServerCertificate,
    pub ca_bundle: CABundle,
    pub root: RootCertificate,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MachineCertificate {
    pub private_key: String,
    pub csr: String,
    pub cert: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ServerCertificate {
    pub cert: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CABundle {
    pub cert: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RootCertificate {
    pub cert: String,
}
