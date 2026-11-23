pub mod actions;
pub mod cli;
pub mod commands;
pub mod shortcuts;
pub mod tray;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use dictafox_core::audio::Recorder;
use dictafox_core::settings::ShortcutMode;
use dictafox_core::Dictafox;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

use crate::shortcuts::Binding;

pub struct AppState {
    pub service: Arc<Dictafox>,
    pub binding: Mutex<Option<Binding>>,
    pub mic_test: Mutex<Option<Recorder>>,
}

pub fn run(initial: Option<cli::CliAction>) {
    let shortcut_plugin = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            let state = app.state::<AppState>();
            let settings = state.service.settings();
            let recording = state.service.is_recording();
            match event.state() {
                ShortcutState::Pressed => {
                    if !actions::accept_trigger() {
                        return;
                    }
                    if recording {
                        // Failsafe for compositors that do not deliver key
                        // releases, and for a missed release.
                        actions::stop(app);
                    } else {
                        actions::start(app, dictafox_core::service::RecordingOrigin::Shortcut);
                    }
                }
                ShortcutState::Released => {
                    if settings.shortcut_mode == ShortcutMode::PushToTalk
                        && recording
                        && !state.service.session().wayland
                    {
                        actions::stop(app);
                    }
                }
            }
        })
        .build();

    let builder = tauri::Builder::default()
        .plugin(shortcut_plugin)
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::get_settings,
            commands::save_settings,
            commands::set_profile,
            commands::get_capabilities,
            commands::get_recording_state,
            commands::get_engine_status,
            commands::get_last_result,
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::insert_text,
            commands::copy_text,
            commands::refine_preview,
            commands::list_history,
            commands::delete_history,
            commands::clear_history,
            commands::list_dictionary,
            commands::upsert_dictionary,
            commands::delete_dictionary,
            commands::list_devices,
            commands::list_models,
            commands::download_model,
            commands::cancel_model_download,
            commands::delete_model,
            commands::verify_model,
            commands::select_model,
            commands::start_mic_test,
            commands::mic_test_state,
            commands::stop_mic_test,
            commands::validate_shortcut,
            commands::setup_desktop_shortcut,
            commands::get_diagnostics,
            commands::open_data_dir,
            commands::open_models_dir,
            commands::open_link,
            commands::quit_app,
        ])
        .setup(move |app| {
            let service = Arc::new(Dictafox::open().map_err(|err| {
                std::io::Error::other(err.message)
            })?);
            app.manage(AppState {
                service: Arc::clone(&service),
                binding: Mutex::new(None),
                mic_test: Mutex::new(None),
            });

            tray::build(app.handle())?;

            let settings = service.settings();
            match shortcuts::install(app.handle(), &settings.shortcut) {
                Ok(binding) => {
                    if let Ok(mut guard) = app.state::<AppState>().binding.lock() {
                        *guard = Some(binding);
                    }
                }
                Err(err) => {
                    dictafox_core::logging::warn(format!(
                        "global shortcut could not be registered: {}",
                        err.message
                    ));
                    let _ = app
                        .handle()
                        .emit("dictafox://shortcut-warning", err.message.clone());
                }
            }

            spawn_recording_watchdog(app.handle().clone());
            cli::serve(app.handle().clone());

            if let Some(action) = initial {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    // Give the webview a moment to attach listeners before a
                    // CLI-triggered dictation starts.
                    std::thread::sleep(Duration::from_millis(700));
                    cli::dispatch(&handle, action);
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Closing the dashboard keeps dictation available from the
                // tray. Quit is an explicit tray action.
                api.prevent_close();
                let _ = window.hide();
            }
        });

    let app = builder
        .build(tauri::generate_context!())
        .expect("DictaFox could not start its window");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::Exit => {
            let state = app_handle.state::<AppState>();
            state.service.shutdown();
            cli::cleanup_socket();
        }
        // Clicking the Dock icon while the window is hidden brings it back.
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => actions::show_main(app_handle),
        _ => {}
    });
}

/// Hard safety net: a missed shortcut release must never leave the microphone
/// recording indefinitely.
fn spawn_recording_watchdog(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last_state = String::new();
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let Some(state) = app.try_state::<AppState>() else {
                continue;
            };
            let snapshot = state.service.snapshot();
            if snapshot.state == "recording" {
                let limit = snapshot.max_seconds as u64 * 1000 + 1500;
                if snapshot.duration_ms >= limit {
                    dictafox_core::logging::warn("maximum recording duration reached; stopping");
                    actions::stop(&app);
                }
                if let Some(tray) = app.tray_by_id("main") {
                    let _ = tray.set_tooltip(Some("DictaFox — recording"));
                }
            } else if last_state == "recording" {
                if let Some(tray) = app.tray_by_id("main") {
                    let _ = tray.set_tooltip(Some("DictaFox"));
                }
            }
            last_state = snapshot.state;
        }
    });
}
