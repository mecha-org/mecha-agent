use anyhow::{bail, Result};
use flate2::read::GzDecoder;
use nix::unistd::Uid;
use sha256::try_digest;
use std::process::{Command, Stdio};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use tar::Archive;
use zip::ZipArchive;
/*
 * Writes to path safely, creates director if required
 */

pub fn sha256_file(path: &str) -> Result<String> {
    let input = Path::new(path);
    let value = match try_digest(input) {
        Ok(v) => v,
        Err(e) => bail!(e),
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
            bail!("error while running command, error - {}", stderr)
        }
    };

    Ok(res)
}
