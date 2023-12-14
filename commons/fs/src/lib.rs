use anyhow::{bail, Result};
use dirs::home_dir;
use std::{fs::File, io::Write, path::PathBuf};

/*
 * Writes to path safely, creates director if required
 */
pub fn safe_write_to_path(path: &str, content: &[u8]) -> Result<bool> {
    let path_buf = match construct_dir_path(path) {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    // Extract the file name (the last component of the path)
    if let Some(file_name) = path_buf.file_name() {
        if let Some(file_name_str) = file_name.to_str() {
            let mut dir_to_create = path_buf.clone();
            // Last component will be pulled out
            dir_to_create.pop();
            match mkdirp::mkdirp(&dir_to_create) {
                Ok(p) => p,
                Err(err) => bail!(err),
            };

            let actual_path = dir_to_create.join(file_name_str);
            let mut file = match File::create(actual_path) {
                Ok(f) => f,
                Err(err) => bail!(err),
            };

            match file.write_all(content) {
                Ok(()) => Ok(true),
                Err(err) => bail!(err),
            }
        } else {
            bail!("Invalid file name");
        }
    } else {
        bail!("Invalid path");
    }
}

/*
 * Writes to path safely, creates director if required
 */
pub fn safe_open_file(path: &str) -> Result<File> {
    let path_buf = match construct_dir_path(path) {
        Ok(v) => v,
        Err(e) => bail!(e),
    };
    let file = match File::open(path_buf) {
        Ok(v) => v,
        Err(e) => {
            bail!("failed to open public key file - {}", e);
        }
    };
    Ok(file)
}

pub fn construct_dir_path(path: &str) -> Result<PathBuf> {
    // Convert the string path to a Path
    let mut path_buf = PathBuf::from(path);

    // Expand the tilde if it exists
    if path.starts_with("~") {
        if let Some(home_dir) = home_dir() {
            let path_to_join = match path_buf.strip_prefix("~") {
                Ok(v) => v,
                Err(e) => {
                    bail!("Failed to join path - {}", e);
                }
            };
            path_buf = home_dir.join(path_to_join);
        }
    }
    Ok(path_buf)
}
