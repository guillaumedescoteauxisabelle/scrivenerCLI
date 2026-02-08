use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use chrono::Utc;

pub fn read_input(from_file: Option<&Path>, stdin: bool) -> Result<String> {
    if let Some(path) = from_file {
        return Ok(fs::read_to_string(path)?);
    }
    if stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        return Ok(buf);
    }
    Err(anyhow!("input required: pass --from-file or --stdin"))
}

pub fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    ensure_parent(path)?;
    let tmp = path.with_extension(format!(
        "tmp.{}",
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    fs::write(&tmp, content)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn backup_file(path: &Path, backup_root: &Path) -> Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    fs::create_dir_all(backup_root)?;
    let stamp = Utc::now().format("%Y%m%d%H%M%S");
    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let backup = backup_root.join(format!("{}-{}", stamp, filename));
    fs::copy(path, &backup)?;
    Ok(Some(backup))
}

pub fn normalize_binder_path(path: &str) -> String {
    let mut out = path.trim().trim_matches('/').replace("//", "/");
    while out.contains("//") {
        out = out.replace("//", "/");
    }
    out
}
