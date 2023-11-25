use anyhow::{bail, Result};
use flate2::read::GzDecoder;
use sha256::try_digest;
use std::process::{Command, Stdio};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use tar::Archive;
use zip::ZipArchive;
/*
 * Writes to path safely, creates director if required
 */
pub fn safe_write_to_path(path: &str, content: &[u8]) -> Result<bool> {
    // Convert the string path to a Path
    let path = Path::new(&path);

    // Extract the file name (the last component of the path)
    if let Some(file_name) = path.file_name() {
        if let Some(file_name_str) = file_name.to_str() {
            let mut dir_to_crate = PathBuf::from(path);
            //Last component will be pooled out
            dir_to_crate.pop();
            match mkdirp::mkdirp(&dir_to_crate) {
                Ok(p) => p,
                Err(err) => bail!(err),
            };

            let actual_path = dir_to_crate.join(file_name_str);
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
    let sudo_check = Command::new("sudo")
        .arg("-n")
        .arg("true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Ok(exit_status) = sudo_check {
        return exit_status.success();
    }

    false
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
