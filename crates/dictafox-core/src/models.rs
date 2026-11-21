use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::paths;

pub const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSpec {
    pub id: String,
    pub label: String,
    pub file: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub multilingual: bool,
    pub note: String,
    pub recommended_ram_gb: f32,
}

impl ModelSpec {
    pub fn url(&self) -> String {
        format!("{HF_BASE}/{}", self.file)
    }
}

fn spec(
    id: &str,
    label: &str,
    file: &str,
    size_bytes: u64,
    sha256: &str,
    multilingual: bool,
    note: &str,
    ram: f32,
) -> ModelSpec {
    ModelSpec {
        id: id.into(),
        label: label.into(),
        file: file.into(),
        size_bytes,
        sha256: sha256.into(),
        multilingual,
        note: note.into(),
        recommended_ram_gb: ram,
    }
}

/// Published SHA-256 digests from the whisper.cpp model repository. They are
/// checked after every download so a truncated or tampered file is rejected.
pub fn catalog() -> Vec<ModelSpec> {
    vec![
        spec("tiny.en", "Tiny (English)", "ggml-tiny.en.bin", 77_704_715, "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f", false, "Fastest. Ideal for quick notes on any CPU.", 1.0),
        spec("tiny", "Tiny (Multilingual)", "ggml-tiny.bin", 77_691_713, "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21", true, "Fastest multilingual option, 99 languages.", 1.0),
        spec("base.en", "Base (English)", "ggml-base.en.bin", 147_964_211, "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002", false, "Recommended default. Good accuracy on CPU.", 2.0),
        spec("base", "Base (Multilingual)", "ggml-base.bin", 147_951_465, "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe", true, "Recommended multilingual default, including Urdu.", 2.0),
        spec("small.en", "Small (English)", "ggml-small.en.bin", 487_614_201, "c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d", false, "Better accuracy with punctuation, still CPU friendly.", 4.0),
        spec("small", "Small (Multilingual)", "ggml-small.bin", 487_601_967, "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b", true, "Better multilingual accuracy for code-switching.", 4.0),
        spec("medium.en", "Medium (English)", "ggml-medium.en.bin", 1_533_774_781, "cc37e93478338ec7700281a7ac30a10128929eb8f427dda2e865faa8f6da4356", false, "High accuracy; needs a fast multi-core CPU.", 8.0),
        spec("medium", "Medium (Multilingual)", "ggml-medium.bin", 1_533_763_059, "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208", true, "High accuracy multilingual; slower on CPU.", 8.0),
        spec("large-v3-turbo", "Large v3 Turbo", "ggml-large-v3-turbo.bin", 1_624_555_275, "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69", true, "Best accuracy per second, needs plenty of RAM.", 12.0),
    ]
}

pub fn find_spec(id: &str) -> Option<ModelSpec> {
    catalog().into_iter().find(|model| model.id == id)
}

pub fn model_path(dir: &Path, spec: &ModelSpec) -> PathBuf {
    dir.join(&spec.file)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModel {
    pub id: String,
    pub label: String,
    pub path: String,
    pub size_bytes: u64,
    pub multilingual: bool,
    pub note: String,
    pub recommended_ram_gb: f32,
    pub installed: bool,
    pub active: bool,
    pub file_present: bool,
}

/// Catalog with installation state. Sizes come from the files on disk so the UI
/// never invents values.
pub fn list_models(dir: &Path, active: &str, custom_path: Option<&str>) -> Vec<InstalledModel> {
    catalog()
        .into_iter()
        .map(|spec| {
            let path = model_path(dir, &spec);
            let metadata = std::fs::metadata(&path).ok();
            let installed = metadata
                .as_ref()
                .map(|m| m.len() == spec.size_bytes)
                .unwrap_or(false);
            InstalledModel {
                id: spec.id.clone(),
                label: spec.label.clone(),
                path: path.display().to_string(),
                size_bytes: metadata.map(|m| m.len()).unwrap_or(spec.size_bytes),
                multilingual: spec.multilingual,
                note: spec.note.clone(),
                recommended_ram_gb: spec.recommended_ram_gb,
                file_present: path.is_file(),
                installed,
                active: custom_path.is_none() && spec.id == active,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: u64,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub valid: bool,
    pub size_ok: bool,
    pub hash_ok: bool,
    pub header_ok: bool,
    pub message: String,
}

pub fn verify_file(path: &Path, expected_size: u64, expected_sha: &str) -> AppResult<Verification> {
    let metadata = std::fs::metadata(path).map_err(|_| {
        AppError::new("model_missing", "The model file is missing.")
            .hint("Download the model again from Settings → Models.")
    })?;
    let size_ok = metadata.len() == expected_size;
    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 4];
    let header_ok = file.read_exact(&mut header).is_ok() && has_ggml_magic(&header);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut hash_ok = true;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = format!("{:x}", hasher.finalize());
    if !expected_sha.is_empty() {
        hash_ok = digest.eq_ignore_ascii_case(expected_sha);
    }
    let valid = size_ok && header_ok && hash_ok;
    let message = if valid {
        "Model verified".to_string()
    } else if !header_ok {
        "This file is not a whisper.cpp ggml model.".to_string()
    } else if !size_ok {
        format!(
            "Size mismatch: expected {} bytes, found {}.",
            expected_size,
            metadata.len()
        )
    } else {
        "Checksum mismatch: the file is corrupt or was modified.".to_string()
    };
    Ok(Verification {
        valid,
        size_ok,
        hash_ok,
        header_ok,
        message,
    })
}

/// Download to a temporary file, verify it, then atomically move it into place.
/// The download can be cancelled cooperatively through `cancel`.
pub fn download(
    spec: &ModelSpec,
    dir: &Path,
    cancel: Arc<AtomicBool>,
    mut progress: impl FnMut(DownloadProgress),
) -> AppResult<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let destination = model_path(dir, spec);
    let partial = destination.with_extension("bin.part");

    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("DictaFox/", env!("CARGO_PKG_VERSION"), " (+https://github.com/VORMELA-CORE/dictafox)"))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|err| {
            AppError::new("network_error", "DictaFox could not prepare the model download.")
                .hint("Check your network connection and try again.")
                .with_detail(err)
        })?;

    let mut response = client.get(spec.url()).send().map_err(|err| {
        AppError::new("network_error", "The model download could not start.")
            .hint("Check your internet connection, then retry the download.")
            .with_detail(err)
    })?;
    if !response.status().is_success() {
        return Err(AppError::new(
            "download_failed",
            format!("The model server returned HTTP {}.", response.status()),
        )
        .hint("Try again in a few minutes, or download the ggml file manually and set a custom model path.")
        .detail(response.status().to_string()));
    }
    let total = response.content_length().unwrap_or(spec.size_bytes);

    let mut file = std::fs::File::create(&partial)?;
    let mut downloaded: u64 = 0;
    let mut buffer = vec![0u8; 256 * 1024];
    progress(DownloadProgress {
        model_id: spec.id.clone(),
        downloaded,
        total,
        phase: "downloading".into(),
    });
    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&partial);
            return Err(AppError::new(
                "download_cancelled",
                "The model download was cancelled.",
            ));
        }
        let read = response.read(&mut buffer).map_err(|err| {
            AppError::new("network_error", "The model download was interrupted.")
                .hint("Check your connection and start the download again.")
                .with_detail(err)
        })?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        downloaded += read as u64;
        progress(DownloadProgress {
            model_id: spec.id.clone(),
            downloaded,
            total,
            phase: "downloading".into(),
        });
    }
    file.flush()?;
    drop(file);

    progress(DownloadProgress {
        model_id: spec.id.clone(),
        downloaded,
        total,
        phase: "verifying".into(),
    });
    let verification = verify_file(&partial, spec.size_bytes, &spec.sha256)?;
    if !verification.valid {
        let _ = std::fs::remove_file(&partial);
        return Err(AppError::new(
            "model_corrupt",
            "The downloaded model failed verification.",
        )
        .hint("Delete the partial download and try again. If it keeps failing, download the model manually from the whisper.cpp repository.")
        .detail(verification.message));
    }
    std::fs::rename(&partial, &destination)?;
    progress(DownloadProgress {
        model_id: spec.id.clone(),
        downloaded,
        total,
        phase: "done".into(),
    });
    Ok(destination)
}

pub fn delete_model(dir: &Path, spec: &ModelSpec) -> AppResult<()> {
    let path = model_path(dir, spec);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Resolve the configured model file: an explicit custom path wins, otherwise
/// the catalog file inside the DictaFox data directory.
pub fn resolve_path(
    dir: &Path,
    model_id: &str,
    custom_path: Option<&str>,
) -> AppResult<(PathBuf, Option<ModelSpec>)> {
    if let Some(custom) = custom_path.filter(|path| !path.trim().is_empty()) {
        let path = PathBuf::from(custom);
        if !path.is_file() {
            return Err(AppError::new(
                "model_missing",
                "The custom model file could not be found.",
            )
            .hint("Check the path in Settings → Models, or download a managed model instead."));
        }
        inspect_ggml(&path)?;
        return Ok((path, None));
    }
    let spec = find_spec(model_id).ok_or_else(|| {
        AppError::new("model_unknown", format!("Unknown speech model \"{model_id}\"."))
            .hint("Choose a model in Settings → Models.")
    })?;
    let path = model_path(dir, &spec);
    if !path.is_file() {
        return Err(AppError::new(
            "model_missing",
            format!("The {} speech model is not installed.", spec.label),
        )
        .hint("Open Settings → Models and download it. DictaFox never downloads models without your consent."));
    }
    Ok((path, Some(spec)))
}

/// whisper.cpp writes the magic 0x67676d6c ("ggml") as a little-endian u32,
/// so model files start with the bytes `lmgg`.
pub const GGML_MAGIC: [u8; 4] = 0x6767_6d6c_u32.to_le_bytes();

fn has_ggml_magic(header: &[u8; 4]) -> bool {
    *header == GGML_MAGIC
}

pub fn inspect_ggml(path: &Path) -> AppResult<u64> {
    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 4];
    file.read_exact(&mut header).map_err(|_| {
        AppError::new(
            "model_invalid",
            "The selected file is not a whisper.cpp model.",
        )
        .hint("Choose a ggml-*.bin file produced by whisper.cpp.")
    })?;
    if !has_ggml_magic(&header) {
        return Err(AppError::new(
            "model_invalid",
            "The selected file is not a whisper.cpp ggml model.",
        )
        .hint("Choose a ggml-*.bin file, for example ggml-base.en.bin."));
    }
    Ok(file.metadata()?.len())
}

/// Total and free bytes on the volume holding the data directory.
#[cfg(unix)]
pub fn disk_usage() -> Option<(u64, u64)> {
    // POSIX `df -P -k` output is identical on Linux and macOS; it avoids a
    // libc dependency for a single readout.
    let dir = paths::data_dir();
    let target = if dir.exists() { dir } else { std::env::temp_dir() };
    let output = std::process::Command::new("df")
        .args(["-P", "-k"])
        .arg(target)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }
    let total: u64 = fields[1].parse().ok()?;
    let free: u64 = fields[3].parse().ok()?;
    Some((total * 1024, free * 1024))
}

#[cfg(windows)]
pub fn disk_usage() -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let dir = paths::data_dir();
    let target = if dir.exists() { dir } else { std::env::temp_dir() };
    let wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let (mut free, mut total, mut total_free) = (0u64, 0u64, 0u64);
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free) };
    (ok != 0).then_some((total, free))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent_and_has_no_duplicates() {
        let models = catalog();
        assert!(models.len() >= 8);
        let mut ids = std::collections::HashSet::new();
        for model in &models {
            assert!(ids.insert(model.id.clone()), "duplicate {}", model.id);
            assert!(model.size_bytes > 1_000_000);
            assert_eq!(model.sha256.len(), 64);
            assert!(model.url().starts_with("https://huggingface.co/"));
            assert!(model.file.starts_with("ggml-") && model.file.ends_with(".bin"));
        }
    }

    #[test]
    fn unknown_model_returns_actionable_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_path(dir.path(), "nope", None).unwrap_err();
        assert_eq!(err.code, "model_unknown");
        assert!(err.hint.is_some());
    }

    #[test]
    fn missing_model_never_silently_downloads() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_path(dir.path(), "base.en", None).unwrap_err();
        assert_eq!(err.code, "model_missing");
        assert!(err.hint.unwrap().contains("download"));
    }

    #[test]
    fn rejects_files_without_the_ggml_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.bin");
        std::fs::write(&path, b"not-a-model").unwrap();
        let err = inspect_ggml(&path).unwrap_err();
        assert_eq!(err.code, "model_invalid");
        // The ASCII spelling is not what whisper.cpp writes to disk.
        std::fs::write(&path, b"ggml-wrong-byte-order").unwrap();
        assert_eq!(inspect_ggml(&path).unwrap_err().code, "model_invalid");
    }

    #[test]
    fn real_model_header_bytes_are_accepted() {
        assert_eq!(&GGML_MAGIC, b"lmgg");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("real-header.bin");
        std::fs::write(&path, [0x6c, 0x6d, 0x67, 0x67, 0x01, 0x00]).unwrap();
        assert!(inspect_ggml(&path).is_ok());
        assert!(verify_file(&path, 6, "").unwrap().header_ok);
    }

    #[test]
    fn verification_detects_size_and_hash_mismatches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ggml-tiny.bin");
        std::fs::write(&path, b"lmgg-fake-content").unwrap();
        let result = verify_file(&path, 999, "00").unwrap();
        assert!(!result.valid);
        assert!(result.header_ok);
        assert!(!result.size_ok);
        assert!(!result.hash_ok);
        assert!(result.message.contains("Size mismatch"));
    }

    #[test]
    fn installed_state_is_reported_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let models = list_models(dir.path(), "base.en", None);
        assert!(models.iter().all(|m| !m.installed));
        assert!(models.iter().any(|m| m.active && m.id == "base.en"));
        std::fs::write(
            model_path(dir.path(), &find_spec("tiny.en").unwrap()),
            GGML_MAGIC,
        )
        .unwrap();
        let models = list_models(dir.path(), "tiny.en", None);
        let tiny = models.iter().find(|m| m.id == "tiny.en").unwrap();
        assert!(tiny.file_present);
        assert!(!tiny.installed, "a short file must not count as installed");
    }

    #[test]
    fn custom_model_path_must_be_a_ggml_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom.bin");
        std::fs::write(&path, b"lmgg-custom").unwrap();
        let (resolved, spec) =
            resolve_path(dir.path(), "base.en", Some(path.to_str().unwrap())).unwrap();
        assert_eq!(resolved, path);
        assert!(spec.is_none());
    }

    #[test]
    fn download_urls_match_catalog_ids() {
        let base = find_spec("base.en").unwrap();
        assert_eq!(base.size_bytes, 147_964_211);
        assert!(base.url().ends_with("ggml-base.en.bin"));
    }
}
