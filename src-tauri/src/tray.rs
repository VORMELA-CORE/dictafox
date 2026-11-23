use dictafox_core::service::RecordingOrigin;
use dictafox_core::settings::Profile;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::{actions, AppState};

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open DictaFox", true, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "Start dictation", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Stop & transcribe", true, None::<&str>)?;
    let cancel = MenuItem::with_id(app, "cancel", "Cancel recording", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit DictaFox", true, None::<&str>)?;

    let profile_items = Profile::all()
        .iter()
        .map(|profile| {
            let id = format!(
                "profile:{}",
                serde_json::to_value(profile)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_string))
                    .unwrap_or_else(|| "general".into())
            );
            MenuItem::with_id(app, id, profile.label(), true, None::<&str>)
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let profile_refs = profile_items
        .iter()
        .map(|item| item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect::<Vec<_>>();
    let profile_menu = Submenu::with_items(app, "Profile", true, &profile_refs)?;

    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &start,
            &stop,
            &cancel,
            &separator,
            &profile_menu,
            &settings,
            &quit,
        ],
    )?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("DictaFox by Vormela Core")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "open" => actions::show_main(app),
                "start" => actions::start(app, RecordingOrigin::Tray),
                "stop" => actions::stop(app),
                "cancel" => actions::cancel(app),
                "settings" => {
                    actions::show_main(app);
                    let _ = app.emit("dictafox://navigate", "settings");
                }
                "quit" => app.exit(0),
                other if other.starts_with("profile:") => {
                    if let Ok(profile) =
                        serde_json::from_value::<Profile>(serde_json::Value::String(
                            other.trim_start_matches("profile:").to_string(),
                        ))
                    {
                        let state = app.state::<AppState>();
                        match state.service.set_profile(profile) {
                            Ok(settings) => {
                                let _ = app.emit("dictafox://settings", settings);
                                let _ = app.emit(
                                    "dictafox://toast",
                                    serde_json::json!({
                                        "kind": "info",
                                        "message": format!("Profile: {}", profile.label())
                                    }),
                                );
                            }
                            Err(err) => {
                                let _ = app.emit("dictafox://error", err);
                            }
                        }
                    }
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                actions::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
