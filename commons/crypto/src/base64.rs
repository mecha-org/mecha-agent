
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use anyhow::Result;

pub fn b64_encode<T: AsRef<[u8]>>(input: T) -> String {
    URL_SAFE_NO_PAD.encode(input)
}

pub fn b64_decode<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(input).map_err(|e| e.into())
}
