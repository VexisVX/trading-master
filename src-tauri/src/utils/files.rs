//! Copy user-selected files into the app's attachments folder.

use std::fs;
use std::path::{Path, PathBuf};

pub fn copy_attachment(
    source: &Path,
    app_data_dir: &Path,
    trade_id: i64,
    kind: &str,
) -> std::io::Result<PathBuf> {
    if !source.exists() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "source not found"));
    }

    let attach_dir = app_data_dir.join("attachments").join(trade_id.to_string());
    fs::create_dir_all(&attach_dir)?;

    let filename = source.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string();

    let dest = attach_dir.join(format!("{}_{}", kind, filename));
    fs::copy(source, &dest)?;
    Ok(dest)
}
