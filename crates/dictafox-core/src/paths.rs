use std::path::PathBuf;

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Per-OS location for persistent data:
/// Linux `$XDG_DATA_HOME/dictafox`, macOS `~/Library/Application Support/DictaFox`,
/// Windows `%APPDATA%\DictaFox`.
fn platform_data_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        home().join("Library/Application Support/DictaFox")
    } else if cfg!(windows) {
        env_dir("APPDATA")
            .unwrap_or_else(|| home().join("AppData").join("Roaming"))
            .join("DictaFox")
    } else {
        env_dir("XDG_DATA_HOME")
            .unwrap_or_else(|| home().join(".local/share"))
            .join("dictafox")
    }
}

fn platform_cache_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        home().join("Library/Caches/DictaFox")
    } else if cfg!(windows) {
        env_dir("LOCALAPPDATA")
            .unwrap_or_else(|| home().join("AppData").join("Local"))
            .join("DictaFox")
            .join("cache")
    } else {
        env_dir("XDG_CACHE_HOME")
            .unwrap_or_else(|| home().join(".cache"))
            .join("dictafox")
    }
}

/// All persistent application data. `DICTAFOX_DATA_DIR` overrides for tests,
/// portable installs and development builds.
pub fn data_dir() -> PathBuf {
    env_dir("DICTAFOX_DATA_DIR").unwrap_or_else(platform_data_dir)
}

pub fn cache_dir() -> PathBuf {
    env_dir("DICTAFOX_CACHE_DIR").unwrap_or_else(platform_cache_dir)
}

pub fn models_dir() -> PathBuf {
    data_dir().join("models")
}

pub fn recordings_dir() -> PathBuf {
    cache_dir().join("recordings")
}

pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

pub fn database_path() -> PathBuf {
    data_dir().join("dictafox.sqlite3")
}

pub fn create_dirs() -> std::io::Result<()> {
    for dir in [data_dir(), models_dir(), recordings_dir(), logs_dir()] {
        std::fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // User-private data directory: local transcripts live here.
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Environment variables are process-wide; serialise these tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn data_dir_honours_env_override() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DICTAFOX_DATA_DIR", "/tmp/dictafox-test-data");
        assert_eq!(data_dir(), PathBuf::from("/tmp/dictafox-test-data"));
        std::env::remove_var("DICTAFOX_DATA_DIR");
    }

    #[test]
    fn derived_paths_live_under_data_dir() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DICTAFOX_DATA_DIR", "/tmp/dictafox-test-derived");
        assert_eq!(models_dir(), PathBuf::from("/tmp/dictafox-test-derived/models"));
        assert_eq!(
            database_path(),
            PathBuf::from("/tmp/dictafox-test-derived/dictafox.sqlite3")
        );
        std::env::remove_var("DICTAFOX_DATA_DIR");
    }
}
