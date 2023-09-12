use std::fs;
use anyhow::{Result, bail};

/*
 * Writes to path safely, creates director if required
 */
pub fn safe_write_to_path(path: &str, content: &[u8], ) -> Result<bool> {
    match mkdirp::mkdirp(path) {
        Ok(p) => p,
        Err(err) => bail!(err),
    };
    match fs::write(path, content) {
        Ok(()) => Ok(true),
        Err(err) => bail!(err),
    }
}
