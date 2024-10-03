pub const PRIVATE_KEY_PATH: &str = "/openssl/ecdsa/key.pem";
pub const CSR_PATH: &str = "/openssl/ecdsa/csr.pem";
pub const CERT_PATH: &str = "/agent/certs/machine.pem";
pub const CA_BUNDLE_PATH: &str = "/agent/certs/ca_bundle.pem";
pub const ROOT_CERT_PATH: &str = "/agent/certs/root.pem";

pub const PING_QUERY_PATH: &str = "/v1/ping";
pub const FIND_MANIFEST_URL_QUERY_PATH: &str = "/v1/provisioning/manifest/find?code=";
pub const CERT_SIGN_URL_QUERY_PATH: &str = "/v1/provisioning/cert/sign";
pub const NONCE_URL_QUERY_PATH: &str = "/v1/messaging/get_nonce";
pub const ISSUE_TOKEN_URL_QUERY_PATH: &str = "/v1/messaging/issue_token";

pub const DB_PATH: &str = "/db";

pub const RSA_KEY_SIZE: usize = 2048;
