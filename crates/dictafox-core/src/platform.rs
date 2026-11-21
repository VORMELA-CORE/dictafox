use serde::Serialize;

use crate::audio::{self, AudioDevice};
use crate::inject::{self, choose_paste_method, PasteMethod, SessionInfo};
use crate::models;
use crate::paths;
use crate::settings::Settings;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub required: bool,
    pub purpose: String,
    pub install_hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limitation {
    pub code: String,
    pub title: String,
    pub description: String,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub version: String,
    /// `linux`, `windows` or `macos`.
    pub os: String,
    /// Human label for the OS, e.g. "Linux", "Windows", "macOS".
    pub os_label: String,
    pub session_type: String,
    pub desktop: String,
    pub display_server: String,
    pub audio_backend: Option<String>,
    pub microphone_count: usize,
    pub default_microphone: Option<String>,
    pub paste_method: String,
    pub global_shortcut_supported: bool,
    /// GNOME Wayland: DictaFox can register its shortcut in GNOME settings.
    pub desktop_shortcut_available: bool,
    pub desktop_shortcut_installed: bool,
    pub engine: String,
    pub data_dir: String,
    pub models_dir: String,
    pub log_file: String,
    pub history_count: i64,
    pub active_model: String,
    pub model_installed: bool,
    pub disk_free_gb: Option<f64>,
    pub dependencies: Vec<DependencyStatus>,
    pub limitations: Vec<Limitation>,
}

pub struct CapabilityInput<'a> {
    pub settings: &'a Settings,
    pub session: &'a SessionInfo,
    pub devices: &'a [AudioDevice],
    pub history_count: i64,
}

#[cfg(target_os = "linux")]
fn linux_dependencies(session: &SessionInfo) -> Vec<DependencyStatus> {
    vec![
        DependencyStatus {
            name: "pactl".into(),
            installed: session.tools.pactl,
            required: true,
            purpose: "List microphones from PipeWire/PulseAudio".into(),
            install_hint: "sudo apt install pulseaudio-utils".into(),
        },
        DependencyStatus {
            name: "parecord".into(),
            installed: session.tools.parecord,
            required: true,
            purpose: "Capture microphone audio".into(),
            install_hint: "sudo apt install pulseaudio-utils pipewire-bin".into(),
        },
        DependencyStatus {
            name: if session.wayland { "wl-copy".into() } else { "xclip".into() },
            installed: if session.wayland {
                session.tools.wl_copy
            } else {
                session.tools.xclip
            },
            required: true,
            purpose: "Put the transcript on the clipboard".into(),
            install_hint: if session.wayland {
                "sudo apt install wl-clipboard".into()
            } else {
                "sudo apt install xclip".into()
            },
        },
        DependencyStatus {
            name: "wtype".into(),
            installed: session.tools.wtype,
            required: false,
            purpose: "Automatic paste on wlroots Wayland desktops".into(),
            install_hint: "sudo apt install wtype".into(),
        },
        DependencyStatus {
            name: "ydotool".into(),
            installed: session.tools.ydotool,
            required: false,
            purpose: "Automatic paste through uinput on Wayland".into(),
            install_hint: "sudo apt install ydotool, then allow your user to write /dev/uinput (ydotool 1.x also needs ydotoold running)".into(),
        },
        DependencyStatus {
            name: "libXtst".into(),
            installed: session.tools.x_test,
            required: false,
            purpose: "Automatic paste on X11".into(),
            install_hint: "sudo apt install libxtst6".into(),
        },
    ]
}

/// Windows and macOS use built-in OS APIs for audio, clipboard and paste, so
/// there is nothing extra to install.
#[cfg(not(target_os = "linux"))]
fn linux_dependencies(_session: &SessionInfo) -> Vec<DependencyStatus> {
    Vec::new()
}

pub fn os_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(windows) {
        "Windows"
    } else {
        "Linux"
    }
}

#[cfg(target_os = "linux")]
fn desktop_shortcut_state(session: &SessionInfo) -> (bool, bool) {
    if gnome_shortcut::available(session) {
        (true, gnome_shortcut::installed())
    } else {
        (false, false)
    }
}

#[cfg(not(target_os = "linux"))]
fn desktop_shortcut_state(_session: &SessionInfo) -> (bool, bool) {
    (false, false)
}

pub fn capabilities(input: CapabilityInput<'_>) -> Capabilities {
    let CapabilityInput {
        settings,
        session,
        devices,
        history_count,
    } = input;
    let paste_method = choose_paste_method(session, settings.insertion.mode);
    let audio_backend = audio::backend_name().map(str::to_string);
    let (model_path, _) = models::resolve_path(
        &paths::models_dir(),
        &settings.model,
        settings.custom_model_path.as_deref(),
    )
    .unwrap_or_else(|_| (paths::models_dir().join("missing"), None));
    let model_installed = model_path.is_file();
    let disk = models::disk_usage();
    let mut dependencies = linux_dependencies(session);
    dependencies.sort_by(|a, b| b.required.cmp(&a.required));

    let (desktop_shortcut_available, desktop_shortcut_installed) = desktop_shortcut_state(session);
    let mut limitations = Vec::new();
    if session.wayland && !session.tools.wtype && !session.tools.ydotool {
        limitations.push(Limitation {
            code: "wayland_paste".into(),
            title: "Automatic paste is limited on Wayland".into(),
            description: format!(
                "{} on Wayland does not let DictaFox press Ctrl+V in native apps. DictaFox copies the transcript and asks you to paste it.",
                if session.desktop.is_empty() { "This desktop" } else { &session.desktop }
            ),
            hint: "Install wtype (wlroots desktops) or ydotool (uinput, needs its daemon) to enable automatic insertion. The clipboard fallback always works.".into(),
        });
    }
    if session.wayland && !desktop_shortcut_installed {
        limitations.push(Limitation {
            code: "wayland_shortcut".into(),
            title: "Global shortcuts need a desktop binding on Wayland".into(),
            description: "Wayland compositors do not allow applications to grab global keys. DictaFox registers the shortcut, but it only fires while an XWayland window is focused on most desktops.".into(),
            hint: if desktop_shortcut_available {
                "Select \"Set up GNOME shortcut\" in Settings → Shortcuts, or bind a custom shortcut in GNOME Settings → Keyboard that runs: dictafox --toggle. The tray menu and dashboard always work.".into()
            } else {
                "Bind a custom shortcut in your desktop settings that runs: dictafox --toggle (or dictafox --start / --stop). The tray menu and dashboard always work.".into()
            },
        });
    }
    if devices.iter().all(|device| device.is_monitor) || devices.is_empty() {
        limitations.push(Limitation {
            code: "no_microphone".into(),
            title: "No microphone detected".into(),
            description: "Only monitor (output) sources are available, so there is nothing to record from.".into(),
            hint: "Connect a microphone, check the input device in your sound settings, then refresh DictaFox.".into(),
        });
    }
    if !model_installed {
        limitations.push(Limitation {
            code: "no_model".into(),
            title: "No speech model installed".into(),
            description: "DictaFox transcribes locally and needs a whisper.cpp model file before the first dictation.".into(),
            hint: "Open Settings → Models and download a model. Tiny is 78 MB and fast; Base is more accurate.".into(),
        });
    }
    if cfg!(target_os = "macos")
        && paste_method != PasteMethod::None
        && !inject::accessibility_trusted()
    {
        limitations.push(Limitation {
            code: "macos_accessibility".into(),
            title: "Accessibility permission needed for automatic paste".into(),
            description: "macOS only lets DictaFox press Cmd+V for you after you allow it. Until then the transcript is copied and you paste it yourself.".into(),
            hint: "Open System Settings → Privacy & Security → Accessibility, enable DictaFox, then restart DictaFox.".into(),
        });
    }
    if cfg!(target_os = "linux") && paste_method == PasteMethod::None && !session.wayland {
        limitations.push(Limitation {
            code: "x11_insertion".into(),
            title: "Clipboard-only insertion".into(),
            description: "DictaFox could not load the X11 key-simulation library.".into(),
            hint: "Install libxtst6 to enable automatic paste on X11, or paste manually with Ctrl+V.".into(),
        });
    }

    Capabilities {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        os_label: os_label().to_string(),
        session_type: session.session_type.clone(),
        desktop: session.desktop.clone(),
        display_server: if cfg!(target_os = "macos") {
            "Quartz".into()
        } else if cfg!(windows) {
            "Desktop Window Manager".into()
        } else if session.wayland {
            "Wayland".into()
        } else {
            "X11".into()
        },
        audio_backend,
        microphone_count: devices.iter().filter(|d| !d.is_monitor).count(),
        default_microphone: devices
            .iter()
            .find(|d| d.is_default && !d.is_monitor)
            .or_else(|| devices.iter().find(|d| !d.is_monitor))
            .map(|d| d.id.clone()),
        paste_method: paste_method.label().to_string(),
        global_shortcut_supported: !session.wayland || desktop_shortcut_installed,
        desktop_shortcut_available,
        desktop_shortcut_installed,
        engine: "whisper.cpp (bundled, local)".into(),
        data_dir: paths::data_dir().display().to_string(),
        models_dir: paths::models_dir().display().to_string(),
        log_file: paths::logs_dir().join("dictafox.log").display().to_string(),
        history_count,
        active_model: settings.model.clone(),
        model_installed,
        disk_free_gb: disk.map(|(_, free)| ((free as f64 / 1_073_741_824.0) * 10.0).round() / 10.0),
        dependencies,
        limitations,
    }
}

/// Text report for the developer panel and bug reports. Contains no
/// transcripts, audio or API keys.
pub fn diagnostics(
    input: CapabilityInput<'_>,
    extra: &[(&str, String)],
) -> String {
    let caps = capabilities(input);
    let mut lines = vec![
        format!("DictaFox {} ({})", caps.version, caps.os_label),
        format!("Session: {} ({})", caps.session_type, caps.desktop),
        format!("Display server: {}", caps.display_server),
        format!("Audio backend: {}", caps.audio_backend.as_deref().unwrap_or("missing")),
        format!("Microphones: {}", caps.microphone_count),
        format!("Paste method: {}", caps.paste_method),
        format!("Global shortcut supported: {}", caps.global_shortcut_supported),
        format!("Engine: {}", caps.engine),
        format!("Data directory: {}", caps.data_dir),
        format!("Models directory: {}", caps.models_dir),
        format!("Log file: {}", caps.log_file),
        format!("History entries: {}", caps.history_count),
        format!("Model installed: {}", caps.model_installed),
        format!(
            "Disk free: {}",
            caps.disk_free_gb
                .map(|gb| format!("{gb:.1} GB"))
                .unwrap_or_else(|| "unknown".into())
        ),
        "Dependencies:".to_string(),
    ];
    for dep in &caps.dependencies {
        lines.push(format!(
            "  {}: {}{}",
            dep.name,
            if dep.installed { "ok" } else { "missing" },
            if dep.required { " (required)" } else { " (optional)" }
        ));
    }
    if !caps.limitations.is_empty() {
        lines.push("Limitations:".to_string());
        for limitation in &caps.limitations {
            lines.push(format!("  {}: {}", limitation.code, limitation.title));
        }
    }
    for (key, value) in extra {
        lines.push(format!("{key}: {value}"));
    }
    lines.join("\n")
}

pub fn open_path_in_file_manager(path: &std::path::Path) -> Result<(), String> {
    let target = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("could not open file manager: {err}"))
}

/// Best-effort desktop notification; failures are silent because every
/// result is also shown in the DictaFox window and tray.
pub fn notify(summary: &str, body: &str) {
    #[cfg(target_os = "linux")]
    if audio::command_exists("notify-send") {
        let _ = std::process::Command::new("notify-send")
            .args(["--app-name=DictaFox", "--icon=dictafox", summary, body])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let quote = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "display notification \"{}\" with title \"DictaFox\" subtitle \"{}\"",
            quote(body),
            quote(summary)
        );
        let _ = std::process::Command::new("osascript").args(["-e", &script]).spawn();
    }
    #[cfg(windows)]
    {
        let _ = (summary, body);
    }
}

pub fn detect() -> SessionInfo {
    inject::detect_session()
}

/// GNOME on Wayland ignores application key grabs, but it runs commands bound
/// in its own keyboard settings. DictaFox can register `dictafox --toggle`
/// there for the user, which is the only dependable global shortcut on GNOME.
#[cfg(target_os = "linux")]
pub mod gnome_shortcut {
    use std::process::Command;

    use crate::audio::command_exists;
    use crate::inject::SessionInfo;

    const LIST_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
    const ENTRY_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
    pub const ENTRY_PATH: &str =
        "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/dictafox/";

    pub fn available(session: &SessionInfo) -> bool {
        session.wayland
            && session.desktop.to_ascii_lowercase().contains("gnome")
            && command_exists("gsettings")
    }

    fn gsettings(args: &[&str]) -> Result<String, String> {
        let out = Command::new("gsettings")
            .args(args)
            .output()
            .map_err(|err| format!("could not run gsettings: {err}"))?;
        if !out.status.success() {
            return Err(format!(
                "gsettings {} failed: {}",
                args.first().copied().unwrap_or(""),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Parse a GVariant string array such as `['/a/', '/b/']` or `@as []`.
    pub fn parse_path_list(raw: &str) -> Vec<String> {
        raw.split('\'')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .filter(|path| !path.is_empty())
            .collect()
    }

    pub fn format_path_list(paths: &[String]) -> String {
        let items = paths
            .iter()
            .map(|path| format!("'{path}'"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{items}]")
    }

    /// GVariant string literal.
    pub fn quote(value: &str) -> String {
        format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
    }

    fn entry_schema() -> String {
        format!("{ENTRY_SCHEMA}:{ENTRY_PATH}")
    }

    pub fn installed() -> bool {
        gsettings(&["get", LIST_SCHEMA, "custom-keybindings"])
            .map(|raw| parse_path_list(&raw).iter().any(|path| path == ENTRY_PATH))
            .unwrap_or(false)
    }

    /// Create or update the DictaFox entry. `accelerator` uses GTK syntax,
    /// e.g. `<Control>space`; `command` is the full command line to run.
    pub fn install(accelerator: &str, command: &str) -> Result<(), String> {
        let schema = entry_schema();
        gsettings(&["set", &schema, "name", &quote("DictaFox dictation")])?;
        gsettings(&["set", &schema, "command", &quote(command)])?;
        gsettings(&["set", &schema, "binding", &quote(accelerator)])?;
        let mut paths = parse_path_list(&gsettings(&["get", LIST_SCHEMA, "custom-keybindings"])?);
        if !paths.iter().any(|path| path == ENTRY_PATH) {
            paths.push(ENTRY_PATH.to_string());
            gsettings(&["set", LIST_SCHEMA, "custom-keybindings", &format_path_list(&paths)])?;
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn gvariant_lists_roundtrip() {
            assert!(parse_path_list("@as []").is_empty());
            let paths = parse_path_list("['/a/custom0/', '/b/']");
            assert_eq!(paths, vec!["/a/custom0/".to_string(), "/b/".to_string()]);
            assert_eq!(format_path_list(&paths), "['/a/custom0/', '/b/']");
            assert_eq!(format_path_list(&[]), "[]");
        }

        #[test]
        fn quoting_escapes_quotes_and_backslashes() {
            assert_eq!(quote("it's"), "'it\\'s'");
            assert_eq!(quote("a\\b"), "'a\\\\b'");
        }
    }
}
