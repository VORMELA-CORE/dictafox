use std::fmt::Display;
use std::io::Write;

use crate::paths;

const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Append a line to the local diagnostic log. Logs never contain audio,
/// transcripts or API keys.
pub fn log(level: &str, message: impl Display) {
    let line = format!(
        "{} [{level}] {message}",
        timestamp()
    );
    eprintln!("[dictafox] {line}");
    let path = paths::logs_dir().join("dictafox.log");
    if let Ok(metadata) = std::fs::metadata(&path) {
        if metadata.len() > MAX_LOG_BYTES {
            let rotated = path.with_extension("log.1");
            let _ = std::fs::rename(&path, rotated);
        }
    }
    if let Err(err) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| writeln!(file, "{line}"))
    {
        eprintln!("[dictafox] could not write log file: {err}");
    }
}

pub fn info(message: impl Display) {
    log("info", message);
}

pub fn warn(message: impl Display) {
    log("warn", message);
}

pub fn error(message: impl Display) {
    log("error", message);
}

fn timestamp() -> String {
    // Avoid a chrono dependency for one timestamp: format seconds since epoch
    // as a stable local diagnostic marker.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
