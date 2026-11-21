use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;

use crate::audio::{self, AudioDevice, Recorder};
use crate::dictionary::{self, DictionaryEntry, DictionaryInput};
use crate::error::{AppError, AppResult};
use crate::history::{self, HistoryEntry, NewHistoryEntry};
use crate::inject::{self, InsertionOutcome, SessionInfo};
use crate::logging;
use crate::models::{self, DownloadProgress, InstalledModel};
use crate::paths;
use crate::platform::{self, Capabilities, CapabilityInput};
use crate::refine::{self, RefineOutcome};
use crate::settings::{self, Profile, RefinementMode, Settings};
use crate::storage::Store;
use crate::whisper::{TranscribeOptions, WhisperEngine};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordingOrigin {
    Shortcut,
    Ui,
    Tray,
    Cli,
    Test,
}

impl std::fmt::Display for RecordingOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl RecordingOrigin {
    /// Only shortcut/tray/CLI dictation targets the previously focused window.
    /// Dictation started inside the dashboard must never paste into DictaFox.
    pub fn auto_insert(self) -> bool {
        matches!(
            self,
            RecordingOrigin::Shortcut | RecordingOrigin::Tray | RecordingOrigin::Cli
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RecordingOrigin::Shortcut => "shortcut",
            RecordingOrigin::Ui => "ui",
            RecordingOrigin::Tray => "tray",
            RecordingOrigin::Cli => "cli",
            RecordingOrigin::Test => "test",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSnapshot {
    pub state: String,
    pub level: f32,
    pub duration_ms: u64,
    pub max_seconds: u32,
    pub origin: Option<String>,
    pub microphone: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationResult {
    pub text: String,
    pub raw_text: String,
    pub mode: RefinementMode,
    pub profile: Profile,
    pub model: String,
    pub language: String,
    pub duration_ms: u64,
    pub audio_ms: u64,
    pub transcription_ms: u64,
    pub inserted: bool,
    pub insertion: Option<InsertionOutcome>,
    pub warning: Option<String>,
    pub history_id: Option<i64>,
    pub created_at: i64,
    pub saved_audio: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub engine: String,
    pub loaded_model: Option<String>,
    pub keep_model_loaded: bool,
    pub threads: u32,
}

struct RecordingSession {
    recorder: Recorder,
    settings: Settings,
    origin: RecordingOrigin,
}

pub struct Dictafox {
    store: Arc<Store>,
    settings: RwLock<Settings>,
    engine: WhisperEngine,
    session: SessionInfo,
    recording: Mutex<Option<RecordingSession>>,
    /// Held while a recorder is being opened, so two near-simultaneous starts
    /// (key autorepeat, tray + shortcut) cannot both open the microphone.
    starting: AtomicBool,
    transcribing: AtomicBool,
    last_result: Mutex<Option<DictationResult>>,
    downloads: Mutex<HashMap<String, Arc<AtomicBool>>>,
    models_dir: PathBuf,
    recordings_dir: PathBuf,
    take_counter: AtomicU64,
}

impl Dictafox {
    pub fn open() -> AppResult<Self> {
        paths::create_dirs()?;
        let store = Store::open(&paths::database_path())?;
        let settings = settings::load_settings(&store)?;
        logging::info(format!(
            "starting DictaFox {} ({} {}, {} devices)",
            env!("CARGO_PKG_VERSION"),
            settings.model,
            settings.profile.label(),
            "local"
        ));
        Ok(Self::with_parts(
            store,
            settings,
            paths::models_dir(),
            paths::recordings_dir(),
            platform::detect(),
        ))
    }

    pub fn with_parts(
        store: Store,
        settings: Settings,
        models_dir: PathBuf,
        recordings_dir: PathBuf,
        session: SessionInfo,
    ) -> Self {
        Self {
            store: Arc::new(store),
            settings: RwLock::new(settings),
            engine: WhisperEngine::new(),
            session,
            recording: Mutex::new(None),
            starting: AtomicBool::new(false),
            transcribing: AtomicBool::new(false),
            last_result: Mutex::new(None),
            downloads: Mutex::new(HashMap::new()),
            models_dir,
            recordings_dir,
            take_counter: AtomicU64::new(0),
        }
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn session(&self) -> &SessionInfo {
        &self.session
    }

    pub fn settings(&self) -> Settings {
        self.settings
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn update_settings(&self, mut new_settings: Settings) -> AppResult<Settings> {
        let previous = self.settings();
        new_settings.validate()?;
        let saved = settings::save_settings(&self.store, new_settings)?;
        if previous.model != saved.model
            || previous.custom_model_path != saved.custom_model_path
            || previous.threads != saved.threads
        {
            self.engine.unload();
        }
        if let Ok(mut guard) = self.settings.write() {
            *guard = saved.clone();
        }
        logging::info(format!(
            "settings saved: model={}, profile={}, shortcut={}, history={}",
            saved.model,
            saved.profile.label(),
            saved.shortcut,
            saved.persist_history
        ));
        Ok(saved)
    }

    pub fn set_profile(&self, profile: Profile) -> AppResult<Settings> {
        let mut settings = self.settings();
        settings.profile = profile;
        self.update_settings(settings)
    }

    pub fn list_devices(&self) -> AppResult<Vec<AudioDevice>> {
        audio::list_devices()
    }

    pub fn capabilities(&self) -> Capabilities {
        let devices = audio::list_devices().unwrap_or_default();
        let history_count = history::count(&self.store).unwrap_or(0);
        let settings = self.settings();
        platform::capabilities(CapabilityInput {
            settings: &settings,
            session: &self.session,
            devices: &devices,
            history_count,
        })
    }

    pub fn active_model_path(&self, settings: &Settings) -> AppResult<PathBuf> {
        let (path, _) = models::resolve_path(
            &self.models_dir,
            &settings.model,
            settings.custom_model_path.as_deref(),
        )?;
        Ok(path)
    }

    pub fn start_recording(&self, origin: RecordingOrigin) -> AppResult<RecordingSnapshot> {
        if self
            .starting
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(AppError::new(
                "already_recording",
                "DictaFox is already starting a recording.",
            ));
        }
        let result = self.start_recording_inner(origin);
        self.starting.store(false, Ordering::SeqCst);
        result
    }

    fn start_recording_inner(&self, origin: RecordingOrigin) -> AppResult<RecordingSnapshot> {
        {
            let guard = self.recording.lock().map_err(|_| {
                AppError::new("recording_busy", "The recorder is unavailable.")
                    .hint("Restart DictaFox and try again.")
            })?;
            if guard.is_some() {
                return Err(AppError::new(
                    "already_recording",
                    "DictaFox is already recording.",
                )
                .hint("Release the shortcut or press it again to stop, then start a new dictation."));
            }
        }
        if self.transcribing.load(Ordering::SeqCst) {
            return Err(AppError::new(
                "busy",
                "The previous recording is still being transcribed.",
            )
            .hint("Wait for the current transcript to finish, then try again."));
        }

        let settings = self.settings();
        // Fail before opening the microphone: downloading a model is a user
        // decision, never an automatic background transfer.
        self.active_model_path(&settings)?;

        if let Some(device) = settings.microphone.as_deref() {
            if !device.trim().is_empty() {
                let available = audio::list_devices().unwrap_or_default();
                if !available.iter().any(|d| d.id == device) {
                    return Err(AppError::new(
                        "microphone_unavailable",
                        "The selected microphone is no longer connected.",
                    )
                    .hint("Choose another input device in Settings → Audio, or set it back to System default."));
                }
            }
        }

        let recorder = Recorder::start(
            settings.microphone.as_deref(),
            settings.max_recording_seconds,
        )?;
        let microphone = settings
            .microphone
            .clone()
            .or_else(|| self.capabilities().default_microphone);
        logging::info(format!(
            "recording started ({origin}, device={})",
            microphone.as_deref().unwrap_or("system default")
        ));
        if let Ok(mut guard) = self.recording.lock() {
            *guard = Some(RecordingSession {
                recorder,
                settings: settings.clone(),
                origin,
            });
        }
        Ok(RecordingSnapshot {
            state: "recording".into(),
            level: 0.0,
            duration_ms: 0,
            max_seconds: settings.max_recording_seconds,
            origin: Some(origin.as_str().into()),
            microphone,
        })
    }

    pub fn snapshot(&self) -> RecordingSnapshot {
        if let Ok(guard) = self.recording.lock() {
            if let Some(session) = guard.as_ref() {
                return RecordingSnapshot {
                    state: "recording".into(),
                    level: session.recorder.level(),
                    duration_ms: session.recorder.duration_ms(),
                    max_seconds: session.settings.max_recording_seconds,
                    origin: Some(session.origin.as_str().into()),
                    microphone: session.settings.microphone.clone(),
                };
            }
        }
        let settings = self.settings();
        RecordingSnapshot {
            state: if self.transcribing.load(Ordering::SeqCst) {
                "transcribing".into()
            } else {
                "idle".into()
            },
            level: 0.0,
            duration_ms: 0,
            max_seconds: settings.max_recording_seconds,
            origin: None,
            microphone: settings.microphone.clone(),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    pub fn stop_recording(&self) -> AppResult<DictationResult> {
        let session = {
            let mut guard = self.recording.lock().map_err(|_| {
                AppError::new("recording_busy", "The recorder is unavailable.")
                    .hint("Restart DictaFox and try again.")
            })?;
            // Set while still holding the lock so no snapshot or start can
            // observe "idle" between taking the session and transcribing.
            let session = guard.take();
            if session.is_some() {
                self.transcribing.store(true, Ordering::SeqCst);
            }
            session
        }
        .ok_or_else(|| {
            AppError::new("not_recording", "DictaFox is not recording.")
                .hint("Start a dictation with the global shortcut or the Record button.")
        })?;

        let result = self.transcribe(session);
        self.transcribing.store(false, Ordering::SeqCst);
        result
    }

    pub fn cancel_recording(&self) -> AppResult<bool> {
        let session = self
            .recording
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        if let Some(session) = session {
            session.recorder.cancel();
            logging::info("recording cancelled");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn transcribe(&self, session: RecordingSession) -> AppResult<DictationResult> {
        let settings = session.settings.clone();
        let origin = session.origin;
        let recorded = session.recorder.stop()?;
        let duration_ms = recorded.duration_ms();
        if duration_ms < 200 {
            return Err(AppError::new(
                "recording_too_short",
                "The recording was too short to transcribe.",
            )
            .hint("Hold the shortcut a little longer, then speak."));
        }

        let take = self.take_counter.fetch_add(1, Ordering::Relaxed);
        let wav_path = self.recordings_dir.join(format!(
            "take-{}-{take}.wav",
            history::now_millis()
        ));
        audio::write_wav(&wav_path, &recorded.samples, recorded.sample_rate)?;

        let outcome = self.transcribe_audio(&settings, &recorded, duration_ms, &wav_path, origin);
        if !settings.save_audio {
            let _ = std::fs::remove_file(&wav_path);
        }
        if !settings.keep_model_loaded {
            self.engine.unload();
        }
        match &outcome {
            Ok(result) => logging::info(format!(
                "dictation complete: audio={}ms transcription={}ms mode={} inserted={} chars={}",
                result.audio_ms,
                result.transcription_ms,
                serde_json::to_value(result.mode)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default(),
                result.inserted,
                result.text.chars().count()
            )),
            Err(err) => logging::error(format!("transcription failed: {} ({})", err.code, err.message)),
        }
        outcome
    }

    fn transcribe_audio(
        &self,
        settings: &Settings,
        recorded: &audio::RecordedAudio,
        duration_ms: u64,
        wav_path: &Path,
        origin: RecordingOrigin,
    ) -> AppResult<DictationResult> {
        let (model_path, spec) = models::resolve_path(
            &self.models_dir,
            &settings.model,
            settings.custom_model_path.as_deref(),
        )?;
        let mut samples = audio::prepare_samples(&recorded.samples, settings.input_gain);
        if settings.vad_enabled {
            let trimmed = audio::trim_silence(&recorded.samples, recorded.sample_rate);
            if trimmed.is_empty() {
                return Err(AppError::new(
                    "no_speech",
                    "No speech was detected in the recording.",
                )
                .hint("Check the selected microphone and its input level in Settings → Audio, then try again."));
            }
            samples = audio::prepare_samples(&trimmed, settings.input_gain);
        }

        let options = TranscribeOptions {
            language: effective_language(&settings.language, spec.as_ref(), &model_path),
            threads: settings.threads,
            translate: false,
        };
        let transcript = self
            .engine
            .transcribe_samples(&samples, &model_path, &options)?;
        if transcript.text.trim().is_empty() {
            return Err(AppError::new(
                "empty_transcription",
                "The speech engine did not recognise any words.",
            )
            .hint("Speak a little louder, check the microphone level, or try a larger model in Settings → Models."));
        }

        let entries = if settings.dictionary_enabled {
            dictionary::list(&self.store).unwrap_or_default()
        } else {
            Vec::new()
        };
        let refined = refine::refine(settings, &entries, &transcript.text);
        let mut warnings = Vec::new();
        if let Some(warning) = refined.warning.clone() {
            warnings.push(warning);
        }

        let mut insertion = None;
        if origin.auto_insert() {
            let live_session = platform::detect();
            match inject::deliver(&refined.text, &settings.insertion, &live_session) {
                Ok(outcome) => {
                    if let Some(warning) = outcome.warning.clone() {
                        warnings.push(warning);
                    }
                    insertion = Some(outcome);
                }
                Err(err) => {
                    warnings.push(format!(
                        "Automatic insertion failed: {}{}",
                        err.message,
                        err.hint
                            .as_ref()
                            .map(|hint| format!(" {hint}"))
                            .unwrap_or_default()
                    ));
                }
            }
        }

        let history_id = if settings.persist_history {
            history::insert(
                &self.store,
                &NewHistoryEntry {
                    text: refined.text.clone(),
                    raw_text: Some(transcript.text.clone()),
                    profile: settings.profile,
                    refinement: refined.mode,
                    model: settings.model.clone(),
                    language: transcript.language.clone(),
                    duration_ms: duration_ms as i64,
                },
            )
            .ok()
        } else {
            None
        };

        let result = DictationResult {
            text: refined.text,
            raw_text: transcript.text,
            mode: refined.mode,
            profile: settings.profile,
            model: settings.model.clone(),
            language: transcript.language,
            duration_ms,
            audio_ms: transcript.audio_ms,
            transcription_ms: transcript.elapsed_ms,
            inserted: insertion.as_ref().map(|i| i.pasted).unwrap_or(false),
            insertion,
            warning: if warnings.is_empty() {
                None
            } else {
                Some(warnings.join(" "))
            },
            history_id,
            created_at: history::now_millis(),
            saved_audio: if settings.save_audio {
                Some(wav_path.display().to_string())
            } else {
                None
            },
        };
        if let Ok(mut guard) = self.last_result.lock() {
            *guard = Some(result.clone());
        }
        Ok(result)
    }

    pub fn last_result(&self) -> Option<DictationResult> {
        self.last_result
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    pub fn insert_text(&self, text: &str) -> AppResult<InsertionOutcome> {
        let settings = self.settings();
        inject::deliver(text, &settings.insertion, &platform::detect())
    }

    pub fn copy_text(&self, text: &str) -> AppResult<String> {
        let session = platform::detect();
        Ok(inject::copy_to_clipboard(text, &session)?.backend)
    }

    pub fn refine_preview(&self, text: &str) -> RefineOutcome {
        let settings = self.settings();
        let entries = dictionary::list(&self.store).unwrap_or_default();
        refine::refine(&settings, &entries, text)
    }

    pub fn history_list(&self, search: Option<String>, limit: u32) -> AppResult<Vec<HistoryEntry>> {
        history::list(&self.store, search.as_deref(), limit)
    }

    pub fn history_delete(&self, id: i64) -> AppResult<bool> {
        history::delete(&self.store, id)
    }

    pub fn history_clear(&self) -> AppResult<usize> {
        let cleared = history::clear(&self.store)?;
        logging::info(format!("history cleared ({cleared} entries)"));
        Ok(cleared)
    }

    pub fn dictionary_list(&self) -> AppResult<Vec<DictionaryEntry>> {
        dictionary::list(&self.store)
    }

    pub fn dictionary_upsert(&self, input: DictionaryInput) -> AppResult<DictionaryEntry> {
        dictionary::upsert(&self.store, &input)
    }

    pub fn dictionary_delete(&self, id: i64) -> AppResult<bool> {
        dictionary::delete(&self.store, id)
    }

    pub fn models_list(&self) -> Vec<InstalledModel> {
        let settings = self.settings();
        models::list_models(
            &self.models_dir,
            &settings.model,
            settings.custom_model_path.as_deref(),
        )
    }

    pub fn download_model(
        &self,
        model_id: &str,
        progress: impl FnMut(DownloadProgress),
    ) -> AppResult<PathBuf> {
        let spec = models::find_spec(model_id).ok_or_else(|| {
            AppError::new("model_unknown", format!("Unknown model \"{model_id}\"."))
                .hint("Choose a model from the list in Settings → Models.")
        })?;
        let cancel = Arc::new(AtomicBool::new(false));
        if let Ok(mut downloads) = self.downloads.lock() {
            downloads.insert(spec.id.clone(), Arc::clone(&cancel));
        }
        let result = models::download(&spec, &self.models_dir, cancel, progress);
        if let Ok(mut downloads) = self.downloads.lock() {
            downloads.remove(&spec.id);
        }
        if let Ok(path) = &result {
            logging::info(format!("model installed: {}", path.display()));
        }
        result
    }

    pub fn cancel_model_download(&self, model_id: &str) -> bool {
        self.downloads
            .lock()
            .ok()
            .and_then(|downloads| {
                downloads.get(model_id).map(|cancel| {
                    cancel.store(true, Ordering::Relaxed);
                    true
                })
            })
            .unwrap_or(false)
    }

    pub fn delete_model(&self, model_id: &str) -> AppResult<()> {
        let settings = self.settings();
        let spec = models::find_spec(model_id).ok_or_else(|| {
            AppError::new("model_unknown", format!("Unknown model \"{model_id}\"."))
        })?;
        if settings.custom_model_path.is_none() && settings.model == spec.id {
            return Err(AppError::new(
                "model_active",
                "This model is currently in use.",
            )
            .hint("Select another model in Settings → Models before deleting this one."));
        }
        models::delete_model(&self.models_dir, &spec)?;
        logging::info(format!("model deleted: {}", spec.id));
        Ok(())
    }

    pub fn verify_model(&self, model_id: &str) -> AppResult<models::Verification> {
        let spec = models::find_spec(model_id).ok_or_else(|| {
            AppError::new("model_unknown", format!("Unknown model \"{model_id}\"."))
        })?;
        models::verify_file(
            &models::model_path(&self.models_dir, &spec),
            spec.size_bytes,
            &spec.sha256,
        )
    }

    pub fn select_model(&self, model_id: &str) -> AppResult<Settings> {
        let spec = models::find_spec(model_id).ok_or_else(|| {
            AppError::new("model_unknown", format!("Unknown model \"{model_id}\"."))
                .hint("Choose a model from the list in Settings → Models.")
        })?;
        let path = models::model_path(&self.models_dir, &spec);
        if !path.is_file() {
            return Err(AppError::new(
                "model_missing",
                format!("{} is not installed.", spec.label),
            )
            .hint("Download it first, then select it."));
        }
        let mut settings = self.settings();
        settings.model = spec.id;
        settings.custom_model_path = None;
        self.update_settings(settings)
    }

    pub fn engine_status(&self) -> EngineStatus {
        let settings = self.settings();
        EngineStatus {
            engine: "whisper.cpp".into(),
            loaded_model: self
                .engine
                .loaded_path()
                .map(|path| path.display().to_string()),
            keep_model_loaded: settings.keep_model_loaded,
            threads: settings.threads,
        }
    }

    pub fn preview_model_path(&self, settings: &Settings) -> AppResult<PathBuf> {
        self.active_model_path(settings)
    }

    pub fn diagnostics(&self) -> String {
        let settings = self.settings();
        let devices = audio::list_devices().unwrap_or_default();
        let history_count = history::count(&self.store).unwrap_or(0);
        let extra = vec![
            ("Profile", format!("{:?}", settings.profile)),
            ("Refinement", format!("{:?}", settings.refinement)),
            ("Shortcut", format!("{} ({:?})", settings.shortcut, settings.shortcut_mode)),
            ("Microphone", settings.microphone.clone().unwrap_or_else(|| "system default".into())),
            ("Language", settings.language.clone()),
            ("Model", settings.model.clone()),
            ("Custom model path", settings.custom_model_path.clone().unwrap_or_else(|| "none".into())),
            ("Cloud refinement", if settings.provider.enabled {
                format!("enabled ({} {})", settings.provider.base_url, settings.provider.model)
            } else {
                "disabled".into()
            }),
            ("History persistence", settings.persist_history.to_string()),
            ("Save audio", settings.save_audio.to_string()),
            ("Keep model loaded", settings.keep_model_loaded.to_string()),
            ("Engine loaded", self.engine.loaded_path().map(|p| p.display().to_string()).unwrap_or_else(|| "no".into())),
            ("Database", self.store.path().map(|p| p.display().to_string()).unwrap_or_else(|| "in-memory".into())),
        ];
        platform::diagnostics(
            CapabilityInput {
                settings: &settings,
                session: &self.session,
                devices: &devices,
                history_count,
            },
            &extra,
        )
    }

    pub fn shutdown(&self) {
        let _ = self.cancel_recording();
        self.engine.unload();
    }
}

/// English-only models (`*.en`) cannot transcribe other languages, and asking
/// whisper.cpp to auto-detect costs an extra encoder pass per dictation.
fn effective_language(configured: &str, spec: Option<&models::ModelSpec>, path: &Path) -> String {
    let english_only = match spec {
        Some(spec) => !spec.multilingual,
        None => path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_ascii_lowercase().contains(".en."))
            .unwrap_or(false),
    };
    if english_only {
        "en".to_string()
    } else {
        configured.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inject::ToolAvailability;

    #[test]
    fn english_only_models_skip_language_detection() {
        let tiny_en = models::find_spec("tiny.en").unwrap();
        let tiny = models::find_spec("tiny").unwrap();
        let path = Path::new("/models/x.bin");
        assert_eq!(effective_language("auto", Some(&tiny_en), path), "en");
        assert_eq!(effective_language("ur", Some(&tiny_en), path), "en");
        assert_eq!(effective_language("auto", Some(&tiny), path), "auto");
        assert_eq!(effective_language("ur", Some(&tiny), path), "ur");
        assert_eq!(
            effective_language("auto", None, Path::new("/m/ggml-small.en.bin")),
            "en"
        );
        assert_eq!(effective_language("auto", None, Path::new("/m/custom.bin")), "auto");
    }

    fn test_service() -> (Dictafox, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::in_memory().unwrap();
        let mut settings = Settings::default();
        settings.model = "base.en".into();
        let session = SessionInfo {
            session_type: "x11".into(),
            desktop: "test".into(),
            wayland: false,
            display: Some(":0".into()),
            tools: ToolAvailability::default(),
        };
        let service = Dictafox::with_parts(
            store,
            settings,
            dir.path().join("models"),
            dir.path().join("recordings"),
            session,
        );
        (service, dir)
    }

    #[test]
    fn idle_snapshot_is_stable() {
        let (service, _dir) = test_service();
        let snapshot = service.snapshot();
        assert_eq!(snapshot.state, "idle");
        assert_eq!(snapshot.level, 0.0);
        assert!(!service.is_recording());
        assert!(!service.cancel_recording().unwrap());
    }

    #[test]
    fn recording_requires_an_installed_model_before_opening_the_mic() {
        let (service, _dir) = test_service();
        let err = service.start_recording(RecordingOrigin::Shortcut).unwrap_err();
        assert_eq!(err.code, "model_missing");
        assert!(!service.is_recording());
    }

    #[test]
    fn settings_roundtrip_through_the_service() {
        let (service, _dir) = test_service();
        let mut settings = service.settings();
        settings.profile = Profile::Email;
        settings.language = "en".into();
        let saved = service.update_settings(settings).unwrap();
        assert_eq!(saved.profile, Profile::Email);
        assert_eq!(service.settings().language, "en");
        assert_eq!(
            service.settings().effective_refinement(),
            RefinementMode::Email
        );
    }

    #[test]
    fn profile_switch_is_persisted() {
        let (service, _dir) = test_service();
        service.set_profile(Profile::Coding).unwrap();
        assert_eq!(service.settings().profile, Profile::Coding);
        assert_eq!(
            service.settings().effective_refinement(),
            RefinementMode::Developer
        );
    }

    #[test]
    fn dictionary_entries_flow_into_refinement_preview() {
        let (service, _dir) = test_service();
        service
            .dictionary_upsert(DictionaryInput {
                id: None,
                spoken: "doc type".into(),
                written: "DocType".into(),
                category: Some("Frappe".into()),
                case_sensitive: false,
                enabled: true,
            })
            .unwrap();
        let preview = service.refine_preview("um update the doc type");
        assert_eq!(preview.text, "Update the DocType.");
    }

    #[test]
    fn empty_text_cannot_be_inserted() {
        let (service, _dir) = test_service();
        let err = service.insert_text("   ").unwrap_err();
        assert_eq!(err.code, "empty_transcript");
    }

    #[test]
    fn active_model_cannot_be_deleted() {
        let (service, dir) = test_service();
        let spec = models::find_spec("base.en").unwrap();
        std::fs::create_dir_all(dir.path().join("models")).unwrap();
        std::fs::write(
            models::model_path(&dir.path().join("models"), &spec),
            models::GGML_MAGIC,
        )
        .unwrap();
        let err = service.delete_model("base.en").unwrap_err();
        assert_eq!(err.code, "model_active");
    }

    #[test]
    fn unknown_models_are_rejected_everywhere() {
        let (service, _dir) = test_service();
        assert_eq!(
            service.delete_model("does-not-exist").unwrap_err().code,
            "model_unknown"
        );
        assert_eq!(
            service.select_model("does-not-exist").unwrap_err().code,
            "model_unknown"
        );
        assert_eq!(
            service.verify_model("does-not-exist").unwrap_err().code,
            "model_unknown"
        );
    }

    #[test]
    fn history_helpers_delegate_to_storage() {
        let (service, _dir) = test_service();
        assert_eq!(service.history_clear().unwrap(), 0);
        assert!(service.history_list(None, 10).unwrap().is_empty());
        assert!(!service.history_delete(42).unwrap());
    }

    #[test]
    fn capabilities_describe_this_machine_without_inventing_audio() {
        let (service, _dir) = test_service();
        let caps = service.capabilities();
        assert_eq!(caps.version, env!("CARGO_PKG_VERSION"));
        assert!(caps.engine.contains("whisper.cpp"));
        assert!(caps.dependencies.iter().any(|d| d.name == "pactl"));
    }

    #[test]
    fn diagnostics_never_include_the_provider_key() {
        let (service, _dir) = test_service();
        let mut settings = service.settings();
        settings.provider.api_key = Some("super-secret-key".into());
        settings.provider.enabled = false;
        service.update_settings(settings).unwrap();
        let report = service.diagnostics();
        assert!(!report.contains("super-secret-key"));
    }

    #[test]
    fn engine_starts_cold() {
        let (service, _dir) = test_service();
        assert!(service.engine_status().loaded_model.is_none());
        service.shutdown();
    }
}
