use anyhow::{bail, Result};
use flate2::read::GzDecoder;
use nix::unistd::Uid;
use sha256::try_digest;
use std::process::{Child, Command};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use tar::Archive;
use tracing::{debug, error, info, trace};
use zip::ZipArchive;
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub fn sha256_file(path: &str) -> Result<String> {
    trace!(
        func = "sha256_file",
        package = PACKAGE_NAME,
        "file path - {}",
        path
    );
    let input = Path::new(path);
    debug!(
        func = "sha256_file",
        package = PACKAGE_NAME,
        "file path - {:?}",
        input
    );
    let value = match try_digest(input) {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "sha256_file",
                package = PACKAGE_NAME,
                "error while getting sha256 of file - {}",
                e
            );
            bail!(e)
        }
    };
    Ok(value)
}

pub async fn extract_zip_file(compressed_file_path: &str, temp_path: &PathBuf) -> Result<bool> {
    let file = std::fs::File::open(compressed_file_path)?;
    let mut archive = ZipArchive::new(file)?;
    archive.extract(temp_path)?;
    Ok(true)
}

pub async fn extract_tar_file(compressed_file_path: &str, temp_path: &PathBuf) -> Result<bool> {
    let file = File::open(compressed_file_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(&temp_path)?;
    Ok(true)
}

pub fn is_sudo() -> bool {
    return Uid::effective().is_root();
}

pub fn run_command(cmd: &str) -> Result<bool> {
    let mut parts = cmd.split_whitespace();
    let program = parts.next().unwrap();
    let args = parts.collect::<Vec<_>>();

    let mut binding = Command::new(program);
    let output_result = binding.args(&args).output();

    let output = match output_result {
        Ok(v) => v,
        Err(e) => bail!("error while getting output result {}", e),
    };

    let res = match output.status.success() {
        true => true,
        false => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!(
                func = "run_command",
                package = PACKAGE_NAME,
                "error while running command, error - {}",
                stderr
            );
            bail!("error while running command, error - {}", stderr)
        }
    };
    info!(
        func = "run_command",
        package = PACKAGE_NAME,
        "result of executed command - {}",
        res
    );
    Ok(res)
}

pub fn spawn_command(cmd: &str) -> Result<Child> {
    let mut parts = cmd.split_whitespace();
    let program = parts.next().unwrap();
    let args = parts.collect::<Vec<_>>();

    let mut binding = Command::new(program);
    let spawn_result = binding.args(&args).spawn();

    let spawn_child = match spawn_result {
        Ok(v) => v,
        Err(e) => {
            error!(
                func = "spawn_command",
                package = PACKAGE_NAME,
                "failed to spawn command {}, error - {}",
                cmd,
                e
            );
            bail!("failed to spawn command {}, error - {}", cmd, e);
        }
    };

    Ok(spawn_child)
}
