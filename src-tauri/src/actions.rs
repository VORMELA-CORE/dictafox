use std::sync::Arc;

use dictafox_core::service::{DictationResult, RecordingOrigin};
use dictafox_core::{platform, Dictafox};
use tauri::{AppHandle, Emitter, Manager};

use crate::shortcuts;
use crate::AppState;

pub fn service(app: &AppHandle) -> Arc<Dictafox> {
    app.state::<AppState>().service.clone()
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn emit_error(app: &AppHandle, err: &dictafox_core::AppError) {
    dictafox_core::logging::error(format!("{}: {}", err.code, err.message));
    let _ = app.emit("dictafox://error", err);
    let _ = app.emit("dictafox://state", service(app).snapshot());
    if matches!(
        err.code.as_str(),
        "model_missing" | "model_load_failed" | "audio_backend_missing" | "microphone_unavailable"
    ) {
        show_main(app);
        let _ = app.emit("dictafox://navigate", "settings");
    }
}

pub fn start(app: &AppHandle, origin: RecordingOrigin) {
    let app = app.clone();
    std::thread::spawn(move || {
        let service = service(&app);
        if origin != RecordingOrigin::Ui && origin != RecordingOrigin::Test {
            // If the dashboard currently has focus, hide it so the transcript
            // is pasted into the window the user came from, never into DictaFox.
            if let Some(window) = app.get_webview_window("main") {
                if window.is_focused().unwrap_or(false) {
                    let _ = window.hide();
                }
            }
        }
        match service.start_recording(origin) {
            Ok(snapshot) => {
                let _ = app.emit("dictafox://state", &snapshot);
                if let Some(tray) = app.tray_by_id("main") {
                    let _ = tray.set_tooltip(Some("DictaFox — recording"));
                }
                if origin == RecordingOrigin::Shortcut
                    && !app
                        .get_webview_window("main")
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false)
                {
                    platform::notify(
                        "DictaFox is recording",
                        "Release the shortcut to transcribe, or press it again to stop.",
                    );
                }
                if origin == RecordingOrigin::Shortcut {
                    shortcuts::watch_release(app.clone());
                }
            }
            // A duplicate press (key autorepeat, double trigger) is harmless.
            Err(err) if err.code == "already_recording" => {}
            Err(err) => emit_error(&app, &err),
        }
    });
}

pub fn stop(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let service = service(&app);
        if !service.is_recording() {
            return;
        }
        let _ = app.emit(
            "dictafox://state",
            serde_json::json!({ "state": "transcribing" }),
        );
        match service.stop_recording() {
            Ok(result) => {
                announce_result(&app, &result);
            }
            Err(err) => emit_error(&app, &err),
        }
    });
}

pub fn announce_result(app: &AppHandle, result: &DictationResult) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some("DictaFox"));
    }
    let _ = app.emit("dictafox://result", result);
    let _ = app.emit("dictafox://state", service(app).snapshot());
    if let Some(warning) = &result.warning {
        let _ = app.emit(
            "dictafox://toast",
            serde_json::json!({ "kind": "warning", "message": warning }),
        );
        if !result.inserted {
            platform::notify("DictaFox", warning);
        }
    }
}

pub fn cancel(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let service = service(&app);
        match service.cancel_recording() {
            Ok(true) => {
                let _ = app.emit("dictafox://state", service.snapshot());
                let _ = app.emit(
                    "dictafox://toast",
                    serde_json::json!({ "kind": "info", "message": "Recording discarded." }),
                );
            }
            Ok(false) => {}
            Err(err) => emit_error(&app, &err),
        }
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_tooltip(Some("DictaFox"));
        }
    });
}

/// On GNOME Wayland one key press can arrive twice: through the app's own
/// grab (while an XWayland window is focused) and through the GNOME binding
/// that runs `dictafox --toggle`. Presses closer together than a human can
/// type are treated as one.
pub fn accept_trigger() -> bool {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now = dictafox_core::history::now_millis().max(0) as u64;
    let last = LAST.swap(now, Ordering::SeqCst);
    now.saturating_sub(last) >= 300
}

pub fn toggle(app: &AppHandle) {
    if !accept_trigger() {
        return;
    }
    if service(app).is_recording() {
        stop(app);
    } else {
        start(app, RecordingOrigin::Cli);
    }
}
