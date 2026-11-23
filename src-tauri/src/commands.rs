use std::sync::Arc;

use dictafox_core::audio::{AudioDevice, Recorder};
use dictafox_core::dictionary::{DictionaryEntry, DictionaryInput};
use dictafox_core::history::HistoryEntry;
use dictafox_core::models::{DownloadProgress, InstalledModel, Verification};
use dictafox_core::service::{
    DictationResult, EngineStatus, RecordingOrigin, RecordingSnapshot,
};
use dictafox_core::settings::{Profile, Settings};
use dictafox_core::{platform, AppError, AppResult, Dictafox};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::shortcuts;
use crate::AppState;

fn service(app: &AppHandle) -> Arc<Dictafox> {
    app.state::<AppState>().service.clone()
}

async fn blocking<T, F>(work: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| {
            AppError::new("internal_error", "A background task did not finish.")
                .hint("Try again. If it keeps happening, restart DictaFox.")
                .with_detail(err)
        })?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub settings: Settings,
    pub capabilities: dictafox_core::platform::Capabilities,
    pub devices: Vec<AudioDevice>,
    pub audio_error: Option<AppError>,
    pub models: Vec<InstalledModel>,
    pub dictionary: Vec<DictionaryEntry>,
    pub history: Vec<HistoryEntry>,
    pub last_result: Option<DictationResult>,
    pub engine: EngineStatus,
    pub recording: RecordingSnapshot,
}

#[tauri::command]
pub async fn get_bootstrap(app: AppHandle) -> AppResult<Bootstrap> {
    let service = service(&app);
    blocking(move || {
        let devices = match service.list_devices() {
            Ok(devices) => (devices, None),
            Err(err) => (Vec::new(), Some(err)),
        };
        let history = service.history_list(None, 100).unwrap_or_default();
        Ok(Bootstrap {
            settings: service.settings(),
            capabilities: service.capabilities(),
            devices: devices.0,
            audio_error: devices.1,
            models: service.models_list(),
            dictionary: service.dictionary_list().unwrap_or_default(),
            history,
            last_result: service.last_result(),
            engine: service.engine_status(),
            recording: service.snapshot(),
        })
    })
    .await
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> AppResult<Settings> {
    let service = service(&app);
    blocking(move || Ok(service.settings())).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsOutcome {
    pub settings: Settings,
    pub shortcut_warning: Option<String>,
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> AppResult<SaveSettingsOutcome> {
    let service = service(&app);
    let previous_shortcut = service.settings().shortcut;
    let binding_present = app
        .state::<AppState>()
        .binding
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    let service_for_save = Arc::clone(&service);
    let saved = blocking(move || service_for_save.update_settings(settings)).await?;

    let mut shortcut_warning = None;
    if saved.shortcut != previous_shortcut || !binding_present {
        match shortcuts::install(&app, &saved.shortcut) {
            Ok(binding) => {
                shortcuts::sync_desktop_shortcut(&app, &binding);
                if let Ok(mut guard) = app.state::<AppState>().binding.lock() {
                    *guard = Some(binding);
                }
            }
            Err(err) => {
                // Keep the previous working shortcut registered rather than
                // leaving the user without any global dictation key.
                let restored = shortcuts::install(&app, &previous_shortcut);
                match restored {
                    Ok(binding) => {
                        if let Ok(mut guard) = app.state::<AppState>().binding.lock() {
                            *guard = Some(binding);
                        }
                        shortcut_warning = Some(format!(
                            "{}. The previous shortcut ({}) is still active.",
                            err.message, previous_shortcut
                        ));
                    }
                    Err(_) => {
                        shortcut_warning = Some(format!(
                            "{}. No global shortcut is active; use the tray or dashboard to dictate.",
                            err.message
                        ));
                    }
                }
                let _ = app.emit("dictafox://shortcut-warning", &shortcut_warning);
            }
        }
    }

    let _ = app.emit("dictafox://settings", &saved);
    Ok(SaveSettingsOutcome {
        settings: saved,
        shortcut_warning,
    })
}

#[tauri::command]
pub async fn set_profile(app: AppHandle, profile: Profile) -> AppResult<Settings> {
    let service = service(&app);
    let saved = blocking(move || service.set_profile(profile)).await?;
    let _ = app.emit("dictafox://settings", &saved);
    Ok(saved)
}

#[tauri::command]
pub async fn get_capabilities(
    app: AppHandle,
) -> AppResult<dictafox_core::platform::Capabilities> {
    let service = service(&app);
    blocking(move || Ok(service.capabilities())).await
}

#[tauri::command]
pub async fn get_recording_state(app: AppHandle) -> AppResult<RecordingSnapshot> {
    let service = service(&app);
    blocking(move || Ok(service.snapshot())).await
}

#[tauri::command]
pub async fn get_engine_status(app: AppHandle) -> AppResult<EngineStatus> {
    let service = service(&app);
    blocking(move || Ok(service.engine_status())).await
}

#[tauri::command]
pub async fn get_last_result(app: AppHandle) -> AppResult<Option<DictationResult>> {
    let service = service(&app);
    blocking(move || Ok(service.last_result())).await
}

fn parse_origin(origin: Option<String>) -> RecordingOrigin {
    match origin.as_deref() {
        Some("shortcut") => RecordingOrigin::Shortcut,
        Some("tray") => RecordingOrigin::Tray,
        Some("cli") => RecordingOrigin::Cli,
        Some("test") => RecordingOrigin::Test,
        _ => RecordingOrigin::Ui,
    }
}

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    origin: Option<String>,
) -> AppResult<RecordingSnapshot> {
    let service = service(&app);
    let origin = parse_origin(origin);
    let snapshot = blocking(move || service.start_recording(origin)).await?;
    let _ = app.emit("dictafox://state", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn stop_recording(app: AppHandle) -> AppResult<DictationResult> {
    let service = service(&app);
    let _ = app.emit(
        "dictafox://state",
        serde_json::json!({ "state": "transcribing" }),
    );
    let service_for_stop = Arc::clone(&service);
    let result = blocking(move || service_for_stop.stop_recording()).await?;
    let _ = app.emit("dictafox://result", &result);
    let _ = app.emit("dictafox://state", service.snapshot());
    Ok(result)
}

#[tauri::command]
pub async fn cancel_recording(app: AppHandle) -> AppResult<bool> {
    let service = service(&app);
    let service_for_cancel = Arc::clone(&service);
    let cancelled = blocking(move || service_for_cancel.cancel_recording()).await?;
    let _ = app.emit("dictafox://state", service.snapshot());
    Ok(cancelled)
}

#[tauri::command]
pub async fn insert_text(
    app: AppHandle,
    text: String,
) -> AppResult<dictafox_core::inject::InsertionOutcome> {
    let service = service(&app);
    let outcome = blocking(move || service.insert_text(&text)).await?;
    if let Some(warning) = &outcome.warning {
        let _ = app.emit(
            "dictafox://toast",
            serde_json::json!({ "kind": "warning", "message": warning }),
        );
    }
    Ok(outcome)
}

#[tauri::command]
pub async fn copy_text(app: AppHandle, text: String) -> AppResult<String> {
    let service = service(&app);
    blocking(move || service.copy_text(&text)).await
}

#[tauri::command]
pub async fn refine_preview(
    app: AppHandle,
    text: String,
) -> AppResult<dictafox_core::refine::RefineOutcome> {
    let service = service(&app);
    blocking(move || Ok(service.refine_preview(&text))).await
}

#[tauri::command]
pub async fn list_history(
    app: AppHandle,
    search: Option<String>,
    limit: Option<u32>,
) -> AppResult<Vec<HistoryEntry>> {
    let service = service(&app);
    blocking(move || service.history_list(search, limit.unwrap_or(100))).await
}

#[tauri::command]
pub async fn delete_history(app: AppHandle, id: i64) -> AppResult<bool> {
    let service = service(&app);
    blocking(move || service.history_delete(id)).await
}

#[tauri::command]
pub async fn clear_history(app: AppHandle) -> AppResult<usize> {
    let service = service(&app);
    blocking(move || service.history_clear()).await
}

#[tauri::command]
pub async fn list_dictionary(app: AppHandle) -> AppResult<Vec<DictionaryEntry>> {
    let service = service(&app);
    blocking(move || service.dictionary_list()).await
}

#[tauri::command]
pub async fn upsert_dictionary(
    app: AppHandle,
    entry: DictionaryInput,
) -> AppResult<DictionaryEntry> {
    let service = service(&app);
    blocking(move || service.dictionary_upsert(entry)).await
}

#[tauri::command]
pub async fn delete_dictionary(app: AppHandle, id: i64) -> AppResult<bool> {
    let service = service(&app);
    blocking(move || service.dictionary_delete(id)).await
}

#[tauri::command]
pub async fn list_devices(app: AppHandle) -> AppResult<Vec<AudioDevice>> {
    let service = service(&app);
    blocking(move || service.list_devices()).await
}

#[tauri::command]
pub async fn list_models(app: AppHandle) -> AppResult<Vec<InstalledModel>> {
    let service = service(&app);
    blocking(move || Ok(service.models_list())).await
}

#[tauri::command]
pub async fn download_model(app: AppHandle, model_id: String) -> AppResult<()> {
    let service = service(&app);
    let event_app = app.clone();
    blocking(move || {
        service
            .download_model(&model_id, move |progress: DownloadProgress| {
                let _ = event_app.emit("dictafox://model-progress", &progress);
            })
            .map(|_| ())
    })
    .await?;
    let _ = app.emit("dictafox://models-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn cancel_model_download(app: AppHandle, model_id: String) -> AppResult<bool> {
    let service = service(&app);
    blocking(move || Ok(service.cancel_model_download(&model_id))).await
}

#[tauri::command]
pub async fn delete_model(app: AppHandle, model_id: String) -> AppResult<()> {
    let service = service(&app);
    blocking(move || service.delete_model(&model_id)).await?;
    let _ = app.emit("dictafox://models-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn verify_model(app: AppHandle, model_id: String) -> AppResult<Verification> {
    let service = service(&app);
    blocking(move || service.verify_model(&model_id)).await
}

#[tauri::command]
pub async fn select_model(app: AppHandle, model_id: String) -> AppResult<Settings> {
    let service = service(&app);
    let saved = blocking(move || service.select_model(&model_id)).await?;
    let _ = app.emit("dictafox://settings", &saved);
    let _ = app.emit("dictafox://models-updated", ());
    Ok(saved)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicTestState {
    pub active: bool,
    pub level: f32,
    pub peak: f32,
    pub duration_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicTestResult {
    pub success: bool,
    pub duration_ms: u64,
    pub peak: f32,
    pub message: String,
}

#[tauri::command]
pub async fn start_mic_test(app: AppHandle, device: Option<String>) -> AppResult<MicTestState> {
    // A test abandoned by a page that unmounted must not hold the microphone
    // or block every later test.
    let leftover = app
        .state::<AppState>()
        .mic_test
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    if let Some(previous) = leftover {
        previous.cancel();
    }
    let recorder = blocking(move || Recorder::start(device.as_deref(), 20)).await?;
    // Scope the guard so it is not held across the await below; a held
    // MutexGuard makes the command future non-Send.
    {
        let state = app.state::<AppState>();
        let mut guard = state.mic_test.lock().map_err(|_| {
            AppError::new("microphone_unavailable", "The microphone test is unavailable.")
                .hint("Restart DictaFox and try again.")
        })?;
        if guard.is_some() {
            return Err(AppError::new(
                "already_recording",
                "A microphone test is already running.",
            ));
        }
        *guard = Some(recorder);
    }
    mic_test_state(app).await
}

#[tauri::command]
pub async fn mic_test_state(app: AppHandle) -> AppResult<MicTestState> {
    let state = app.state::<AppState>();
    let guard = state.mic_test.lock().map_err(|_| {
        AppError::new("microphone_unavailable", "The microphone test is unavailable.")
    })?;
    Ok(match guard.as_ref() {
        Some(recorder) => MicTestState {
            active: true,
            level: recorder.level(),
            peak: recorder.peak(),
            duration_ms: recorder.duration_ms(),
        },
        None => MicTestState {
            active: false,
            level: 0.0,
            peak: 0.0,
            duration_ms: 0,
        },
    })
}

#[tauri::command]
pub async fn stop_mic_test(app: AppHandle) -> AppResult<MicTestResult> {
    let state = app.state::<AppState>();
    let recorder = {
        let mut guard = state.mic_test.lock().map_err(|_| {
            AppError::new("microphone_unavailable", "The microphone test is unavailable.")
        })?;
        guard.take()
    };
    let Some(recorder) = recorder else {
        return Ok(MicTestResult {
            success: false,
            duration_ms: 0,
            peak: 0.0,
            message: "No microphone test is running.".into(),
        });
    };
    let duration = recorder.duration_ms();
    let peak = recorder.peak();
    match recorder.stop() {
        Ok(audio) => Ok(MicTestResult {
            success: true,
            duration_ms: audio.duration_ms(),
            peak: peak.max(audio.peak),
            message: "Microphone is capturing audio.".into(),
        }),
        Err(err) => Ok(MicTestResult {
            success: false,
            duration_ms: duration,
            peak,
            message: err.message,
        }),
    }
}

/// GNOME Wayland: register `dictafox --toggle` as a GNOME keyboard shortcut.
/// `shortcut` defaults to the saved one (onboarding passes its unsaved choice).
#[tauri::command]
pub async fn setup_desktop_shortcut(
    app: AppHandle,
    shortcut: Option<String>,
) -> AppResult<dictafox_core::platform::Capabilities> {
    let service = service(&app);
    let text = shortcut.unwrap_or_else(|| service.settings().shortcut);
    let binding = shortcuts::parse(&text)?;
    blocking(move || {
        shortcuts::install_desktop_shortcut(&binding)?;
        Ok(service.capabilities())
    })
    .await
}

#[tauri::command]
pub fn validate_shortcut(shortcut: String) -> AppResult<String> {
    Ok(shortcuts::parse(&shortcut)?.display)
}

#[tauri::command]
pub async fn get_diagnostics(app: AppHandle) -> AppResult<String> {
    let service = service(&app);
    blocking(move || Ok(service.diagnostics())).await
}

#[tauri::command]
pub async fn open_data_dir() -> AppResult<()> {
    blocking(move || {
        platform::open_path_in_file_manager(&dictafox_core::paths::data_dir()).map_err(|err| {
            AppError::new("open_failed", "DictaFox could not open the data directory.")
                .hint(err)
        })
    })
    .await
}

#[tauri::command]
pub async fn open_models_dir() -> AppResult<()> {
    blocking(move || {
        platform::open_path_in_file_manager(&dictafox_core::paths::models_dir()).map_err(|err| {
            AppError::new("open_failed", "DictaFox could not open the models directory.")
                .hint(err)
        })
    })
    .await
}

/// Opens one of DictaFox's own project links. The webview names the link;
/// it never supplies a URL, so the page cannot open arbitrary addresses.
#[tauri::command]
pub async fn open_link(link: String) -> AppResult<()> {
    let url = match link.as_str() {
        "website" => "https://vormelacore.com",
        "repository" => "https://github.com/VORMELA-CORE/dictafox",
        _ => {
            return Err(AppError::new("unknown_link", "DictaFox does not know that link."));
        }
    };
    blocking(move || {
        platform::open_url(url).map_err(|err| {
            AppError::new("open_failed", "DictaFox could not open the link.").hint(err)
        })
    })
    .await
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
