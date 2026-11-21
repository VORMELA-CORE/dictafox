use dictafox_core::AppError;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};


#[derive(Debug, Clone)]
pub struct Binding {
    pub shortcut: Shortcut,
    /// X11 keysym name used to poll the physical key for push-to-talk release.
    pub keysym: String,
    pub display: String,
    /// GTK accelerator syntax (`<Control>space`) for desktop keybindings.
    pub accelerator: String,
}

pub fn parse(input: &str) -> Result<Binding, AppError> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(AppError::new(
            "shortcut_invalid",
            "The shortcut is empty.",
        )
        .hint("Choose a shortcut such as Ctrl+Space."));
    }
    let mut modifiers = Modifiers::empty();
    let mut key: Option<(Code, String, String)> = None;
    for token in raw.split('+').map(str::trim).filter(|t| !t.is_empty()) {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "super" | "meta" | "cmd" | "command" | "win" | "windows" => {
                modifiers |= Modifiers::SUPER
            }
            _ => {
                if key.is_some() {
                    return Err(AppError::new(
                        "shortcut_invalid",
                        "The shortcut has more than one key.",
                    )
                    .hint("Use modifiers plus one key, for example Ctrl+Shift+Space."));
                }
                key = parse_key(token);
                if key.is_none() {
                    return Err(AppError::new(
                        "shortcut_invalid",
                        format!("Unsupported key \"{token}\"."),
                    )
                    .hint("Use letters, digits, F1–F24, Space, Enter, Tab, Escape or the arrow keys."));
                }
            }
        }
    }
    let (code, keysym, label) = key.ok_or_else(|| {
        AppError::new("shortcut_invalid", "The shortcut has no key.").hint(
            "Use modifiers plus one key, for example Ctrl+Space.",
        )
    })?;
    if modifiers.is_empty() && !label.starts_with('F') {
        return Err(AppError::new(
            "shortcut_invalid",
            "A global shortcut needs at least one modifier.",
        )
        .hint("Use something like Ctrl+Space, Alt+Shift+D or Ctrl+F9."));
    }
    let display = format_display(modifiers, &label);
    let accelerator = format_accelerator(modifiers, &keysym);
    Ok(Binding {
        shortcut: Shortcut::new(Some(modifiers), code),
        keysym,
        display,
        accelerator,
    })
}

fn parse_key(token: &str) -> Option<(Code, String, String)> {
    let lower = token.to_ascii_lowercase();
    if lower.len() == 1 {
        let ch = lower.chars().next()?;
        if ch.is_ascii_alphabetic() {
            let code = match ch {
                'a' => Code::KeyA,
                'b' => Code::KeyB,
                'c' => Code::KeyC,
                'd' => Code::KeyD,
                'e' => Code::KeyE,
                'f' => Code::KeyF,
                'g' => Code::KeyG,
                'h' => Code::KeyH,
                'i' => Code::KeyI,
                'j' => Code::KeyJ,
                'k' => Code::KeyK,
                'l' => Code::KeyL,
                'm' => Code::KeyM,
                'n' => Code::KeyN,
                'o' => Code::KeyO,
                'p' => Code::KeyP,
                'q' => Code::KeyQ,
                'r' => Code::KeyR,
                's' => Code::KeyS,
                't' => Code::KeyT,
                'u' => Code::KeyU,
                'v' => Code::KeyV,
                'w' => Code::KeyW,
                'x' => Code::KeyX,
                'y' => Code::KeyY,
                'z' => Code::KeyZ,
                _ => return None,
            };
            return Some((code, lower.clone(), token.to_uppercase()));
        }
        if ch.is_ascii_digit() {
            let code = match ch {
                '0' => Code::Digit0,
                '1' => Code::Digit1,
                '2' => Code::Digit2,
                '3' => Code::Digit3,
                '4' => Code::Digit4,
                '5' => Code::Digit5,
                '6' => Code::Digit6,
                '7' => Code::Digit7,
                '8' => Code::Digit8,
                '9' => Code::Digit9,
                _ => return None,
            };
            return Some((code, lower.clone(), token.to_string()));
        }
    }
    match lower.as_str() {
        "space" | "spacebar" => Some((Code::Space, "space".into(), "Space".into())),
        "enter" | "return" => Some((Code::Enter, "Return".into(), "Enter".into())),
        "tab" => Some((Code::Tab, "Tab".into(), "Tab".into())),
        "escape" | "esc" => Some((Code::Escape, "Escape".into(), "Escape".into())),
        "backspace" => Some((Code::Backspace, "BackSpace".into(), "Backspace".into())),
        "delete" | "del" => Some((Code::Delete, "Delete".into(), "Delete".into())),
        "insert" | "ins" => Some((Code::Insert, "Insert".into(), "Insert".into())),
        "home" => Some((Code::Home, "Home".into(), "Home".into())),
        "end" => Some((Code::End, "End".into(), "End".into())),
        "pageup" | "pgup" => Some((Code::PageUp, "Prior".into(), "PageUp".into())),
        "pagedown" | "pgdn" => Some((Code::PageDown, "Next".into(), "PageDown".into())),
        "arrowup" | "up" => Some((Code::ArrowUp, "Up".into(), "Up".into())),
        "arrowdown" | "down" => Some((Code::ArrowDown, "Down".into(), "Down".into())),
        "arrowleft" | "left" => Some((Code::ArrowLeft, "Left".into(), "Left".into())),
        "arrowright" | "right" => Some((Code::ArrowRight, "Right".into(), "Right".into())),
        other => {
            if let Some(number) = other.strip_prefix('f') {
                if let Ok(index) = number.parse::<u8>() {
                    let code = match index {
                        1 => Some(Code::F1),
                        2 => Some(Code::F2),
                        3 => Some(Code::F3),
                        4 => Some(Code::F4),
                        5 => Some(Code::F5),
                        6 => Some(Code::F6),
                        7 => Some(Code::F7),
                        8 => Some(Code::F8),
                        9 => Some(Code::F9),
                        10 => Some(Code::F10),
                        11 => Some(Code::F11),
                        12 => Some(Code::F12),
                        13 => Some(Code::F13),
                        14 => Some(Code::F14),
                        15 => Some(Code::F15),
                        16 => Some(Code::F16),
                        17 => Some(Code::F17),
                        18 => Some(Code::F18),
                        19 => Some(Code::F19),
                        20 => Some(Code::F20),
                        21 => Some(Code::F21),
                        22 => Some(Code::F22),
                        23 => Some(Code::F23),
                        24 => Some(Code::F24),
                        _ => None,
                    };
                    if let Some(code) = code {
                        // X11 keysym names are case-sensitive: "F9", not "f9".
                        return Some((code, other.to_uppercase(), other.to_uppercase()));
                    }
                }
            }
            None
        }
    }
}

fn format_display(modifiers: Modifiers, key: &str) -> String {
    let mut parts = Vec::new();
    if modifiers.contains(Modifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if modifiers.contains(Modifiers::ALT) {
        parts.push(if cfg!(target_os = "macos") { "Option" } else { "Alt" }.to_string());
    }
    if modifiers.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_string());
    }
    if modifiers.contains(Modifiers::SUPER) {
        let label = if cfg!(target_os = "macos") {
            "Cmd"
        } else if cfg!(windows) {
            "Win"
        } else {
            "Super"
        };
        parts.push(label.to_string());
    }
    parts.push(key.to_string());
    parts.join("+")
}

fn format_accelerator(modifiers: Modifiers, keysym: &str) -> String {
    let mut accelerator = String::new();
    for (modifier, name) in [
        (Modifiers::CONTROL, "<Control>"),
        (Modifiers::ALT, "<Alt>"),
        (Modifiers::SHIFT, "<Shift>"),
        (Modifiers::SUPER, "<Super>"),
    ] {
        if modifiers.contains(modifier) {
            accelerator.push_str(name);
        }
    }
    accelerator.push_str(keysym);
    accelerator
}

/// The command a desktop keybinding runs. An AppImage is mounted at a new
/// path on every launch, so its stable outer path (`$APPIMAGE`) is used.
#[cfg(target_os = "linux")]
fn toggle_command() -> Result<String, AppError> {
    let exe = std::env::var_os("APPIMAGE")
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .ok_or_else(|| {
            AppError::new("desktop_shortcut_failed", "DictaFox could not find its own program file.")
                .hint("Bind a custom shortcut that runs: dictafox --toggle")
        })?;
    let exe = exe.display().to_string();
    let safe = exe
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-+".contains(c));
    let exe = if safe {
        exe
    } else {
        format!("'{}'", exe.replace('\'', "'\\''"))
    };
    Ok(format!("{exe} --toggle"))
}

/// Register (or update) the GNOME keybinding that runs `dictafox --toggle`.
#[cfg(target_os = "linux")]
pub fn install_desktop_shortcut(binding: &Binding) -> Result<(), AppError> {
    let command = toggle_command()?;
    dictafox_core::platform::gnome_shortcut::install(&binding.accelerator, &command).map_err(|err| {
        AppError::new(
            "desktop_shortcut_failed",
            "DictaFox could not add its shortcut to GNOME settings.",
        )
        .hint("Open GNOME Settings → Keyboard → Custom Shortcuts and add one that runs: dictafox --toggle")
        .with_detail(err)
    })?;
    dictafox_core::logging::info(format!(
        "GNOME shortcut set: {} runs {command}",
        binding.accelerator
    ));
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn install_desktop_shortcut(_binding: &Binding) -> Result<(), AppError> {
    Err(AppError::new(
        "desktop_shortcut_unsupported",
        "Desktop shortcuts are only needed on Linux Wayland.",
    ))
}

/// Keep an existing GNOME keybinding in step with a changed shortcut.
pub fn sync_desktop_shortcut(app: &AppHandle, binding: &Binding) {
    #[cfg(target_os = "linux")]
    {
        use tauri::Manager;
        let state = app.state::<crate::AppState>();
        let session = state.service.session();
        if dictafox_core::platform::gnome_shortcut::available(session)
            && dictafox_core::platform::gnome_shortcut::installed()
        {
            if let Err(err) = install_desktop_shortcut(binding) {
                dictafox_core::logging::warn(format!("GNOME shortcut not updated: {}", err.message));
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (app, binding);
}

pub fn install(app: &AppHandle, shortcut_text: &str) -> Result<Binding, AppError> {
    let binding = parse(shortcut_text)?;
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();
    manager.register(binding.shortcut).map_err(|err| {
        AppError::new(
            "shortcut_conflict",
            format!("The shortcut {} is already used by another application.", binding.display),
        )
        .hint("Choose a different shortcut in Settings → Shortcuts.")
        .with_detail(err)
    })?;
    Ok(binding)
}

/// Poll the physical key on X11 so push-to-talk stops on release even when the
/// compositor/plugin does not deliver a release event. On Wayland, keyboard
/// state cannot be read globally, so the shortcut acts as press-to-toggle and
/// the watchdog bounds recording length. Windows and macOS deliver real
/// release events through the global-shortcut plugin, so nothing is polled.
#[cfg(target_os = "linux")]
pub fn watch_release(app: AppHandle) {
    use dictafox_core::inject::X11Keyboard;
    use dictafox_core::settings::ShortcutMode;
    use std::time::{Duration, Instant};
    use tauri::Manager;

    use crate::{actions, AppState};

    let state = app.state::<AppState>();
    let service = state.service.clone();
    if service.session().wayland {
        return;
    }
    let settings = service.settings();
    if settings.shortcut_mode != ShortcutMode::PushToTalk {
        return;
    }
    let keysym = match state.binding.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(binding) => binding.keysym.clone(),
            None => return,
        },
        Err(_) => return,
    };

    std::thread::spawn(move || {
        let keyboard = match X11Keyboard::new() {
            Ok(keyboard) => keyboard,
            Err(err) => {
                dictafox_core::logging::warn(format!(
                    "push-to-talk release monitor unavailable: {err}"
                ));
                return;
            }
        };
        let started = Instant::now();
        let max = Duration::from_secs(settings.max_recording_seconds as u64 + 2);
        // A short grace period so the key is still held when polling begins.
        std::thread::sleep(Duration::from_millis(60));
        while service.is_recording() && started.elapsed() < max {
            match keyboard.is_key_down(&keysym) {
                Ok(true) => std::thread::sleep(Duration::from_millis(35)),
                Ok(false) => {
                    actions::stop(&app);
                    return;
                }
                Err(err) => {
                    dictafox_core::logging::warn(format!("key polling failed: {err}"));
                    return;
                }
            }
        }
    });
}

#[cfg(not(target_os = "linux"))]
pub fn watch_release(_app: AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_shortcuts() {
        let binding = parse("Ctrl+Space").unwrap();
        assert_eq!(binding.display, "Ctrl+Space");
        assert_eq!(binding.keysym, "space");
        assert_eq!(
            parse("ctrl + shift + d").unwrap().display,
            "Ctrl+Shift+D"
        );
        assert_eq!(parse("F9").unwrap().display, "F9");
        let alt_up = if cfg!(target_os = "macos") { "Option+Up" } else { "Alt+Up" };
        assert_eq!(parse("Alt+ArrowUp").unwrap().display, alt_up);
    }

    #[test]
    fn keysyms_and_accelerators_use_x11_names() {
        let f9 = parse("Ctrl+F9").unwrap();
        assert_eq!(f9.keysym, "F9");
        assert_eq!(f9.accelerator, "<Control>F9");
        assert_eq!(parse("Ctrl+Space").unwrap().accelerator, "<Control>space");
        assert_eq!(
            parse("Super+Alt+Shift+D").unwrap().accelerator,
            "<Alt><Shift><Super>d"
        );
    }

    #[test]
    fn rejects_bad_shortcuts_with_actionable_errors() {
        assert_eq!(parse("").unwrap_err().code, "shortcut_invalid");
        assert_eq!(parse("Space").unwrap_err().code, "shortcut_invalid");
        assert_eq!(parse("Ctrl+").unwrap_err().code, "shortcut_invalid");
        assert_eq!(parse("Ctrl+A+B").unwrap_err().code, "shortcut_invalid");
        assert_eq!(parse("Ctrl+Banana").unwrap_err().code, "shortcut_invalid");
    }
}
