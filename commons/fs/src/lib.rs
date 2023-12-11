use anyhow::{bail, Result};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

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
