use anyhow::{bail, Result};
use dirs::home_dir;
use std::{fs::File, io::Write, path::PathBuf};
use tracing::{error, trace};

use crate::errors::{FsError, FsErrorCodes};
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub mod errors;
/*
 * Writes to path safely, creates director if required
 */
pub fn safe_write_to_path(path: &str, content: &[u8]) -> Result<bool> {
    trace!(
        func = "safe_write_to_path",
        package = PACKAGE_NAME,
        "writing to path - {}",
        path
    );
    let path_buf = match construct_dir_path(path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "safe_write_to_path",
                package = PACKAGE_NAME,
                "failed to construct path - {}, error - {}",
                path,
                e
            );
            bail!(e)
        }
    };
    // Extract the file name (the last component of the path)
    if let Some(file_name) = path_buf.file_name() {
        if let Some(file_name_str) = file_name.to_str() {
            let mut dir_to_create = path_buf.clone();
            // Last component will be pulled out
            dir_to_create.pop();
            match mkdirp::mkdirp(&dir_to_create) {
                Ok(p) => p,
                Err(err) => {
                    error!(
                        func = "safe_write_to_path",
                        package = PACKAGE_NAME,
                        "failed to create directory - {}, error - {}",
                        dir_to_create.to_str().unwrap_or(""),
                        err
                    );
                    bail!(err)
                }
            };

            let actual_path = dir_to_create.join(file_name_str);
            let mut file = match File::create(&actual_path) {
                Ok(f) => f,
                Err(err) => {
                    error!(
                        func = "safe_write_to_path",
                        package = PACKAGE_NAME,
                        "failed to create file - {}, error - {}",
                        file_name_str,
                        err
                    );
                    bail!(FsError::new(
                        FsErrorCodes::FileCreateError,
                        format!(
                            "failed to create file path - {:?}, error - {}",
                            &actual_path, err
                        ),
                    ));
                }
            };

            match file.write_all(content) {
                Ok(()) => Ok(true),
                Err(err) => {
                    error!(
                        func = "safe_write_to_path",
                        package = PACKAGE_NAME,
                        "failed to write to file - {}, error - {}",
                        file_name_str,
                        err
                    );
                    bail!(FsError::new(
                        FsErrorCodes::FileWriteError,
                        format!("failed to write to file - {}", file_name_str)
                    ));
                }
            }
        } else {
            error!(
                func = "safe_write_to_path",
                package = PACKAGE_NAME,
                "invalid file name - {}",
                file_name.to_str().unwrap_or("")
            );
            bail!(FsError::new(
                FsErrorCodes::InvalidFileNameError,
                "invalid file name".to_string(),
            ));
        }
    } else {
        error!(
            func = "safe_write_to_path",
            package = PACKAGE_NAME,
            "invalid file path - {}",
            path
        );
        bail!(FsError::new(
            FsErrorCodes::InvalidFilePathError,
            "invalid file path".to_string(),
        ));
    }
}

pub fn remove_files(paths: Vec<&str>) -> Result<()> {
    for path in paths {
        let path_buf = match construct_dir_path(path) {
            Ok(v) => v,
            Err(e) => {
                error!(
                    func = "remove_files",
                    package = PACKAGE_NAME,
                    "failed to construct path - {}, error - {}",
                    path,
                    e
                );
                bail!(e)
            }
        };
        match std::fs::remove_file(path_buf) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    func = "remove_files",
                    package = PACKAGE_NAME,
                    "failed to remove file - {}, error - {}",
                    path,
                    e
                );
                bail!(FsError::new(
                    FsErrorCodes::FileRemoveError,
                    format!("failed to remove file - {}", path),

                ));
            }
        };
    }
    Ok(())
}
/*
 * Writes to path safely, creates director if required
 */
pub fn safe_open_file(path: &str) -> Result<File> {
    trace!(
        func = "safe_open_file",
        package = PACKAGE_NAME,
        "opening file path - {}",
        path
    );
    let path_buf = match construct_dir_path(path) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "safe_open_file",
                package = PACKAGE_NAME,
                "failed to construct path - {}, error - {}",
                path,
                e
            );
            bail!(e)
        }
    };
    let file = match File::open(path_buf) {
        Ok(v) => v,
        Err(err) => {
            error!(
                func = "safe_open_file",
                package = PACKAGE_NAME,
                "failed to open file - {}, error - {}",
                path,
                err
            );
            bail!(FsError::new(
                FsErrorCodes::FileOpenError,
                format!("failed to open file - {}, error - {}", path, err),
            ));
        }
    };
    Ok(file)
}

pub fn construct_dir_path(path: &str) -> Result<PathBuf> {
    trace!(
        func = "construct_dir_path",
        package = PACKAGE_NAME,
        "constructing path - {}",
        path
    );
    // Convert the string path to a Path
    let mut path_buf = PathBuf::from(path);

    // Expand the tilde if it exists
    if path.starts_with("~") {
        if let Some(home_dir) = home_dir() {
            let path_to_join = match path_buf.strip_prefix("~") {
                Ok(v) => v,
                Err(e) => {
                    error!(
                        func = "construct_dir_path",
                        package = PACKAGE_NAME,
                        "failed to strip prefix - {}, error - {}",
                        path,
                        e
                    );
                    bail!(FsError::new(
                        FsErrorCodes::JoinPathError,
                        format!("failed to strip prefix - {}, error - {}", path, e),
    
                    ));
                }
            };
            path_buf = home_dir.join(path_to_join);
        }
    }
    Ok(path_buf)
}
