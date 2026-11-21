#[cfg(target_os = "linux")]
use std::ffi::{c_char, c_void, CString};
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
use crate::audio::command_exists;
use crate::error::{AppError, AppResult};
use crate::settings::InsertionMode;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolAvailability {
    pub wl_copy: bool,
    pub wl_paste: bool,
    pub xclip: bool,
    pub wtype: bool,
    pub ydotool: bool,
    pub x_test: bool,
    pub pactl: bool,
    pub parecord: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_type: String,
    pub desktop: String,
    pub wayland: bool,
    pub display: Option<String>,
    pub tools: ToolAvailability,
}

#[cfg(target_os = "linux")]
pub fn detect_session() -> SessionInfo {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    let wayland = session_type.eq_ignore_ascii_case("wayland")
        || std::env::var_os("WAYLAND_DISPLAY").is_some();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("XDG_SESSION_DESKTOP"))
        .unwrap_or_else(|_| "unknown".into());
    let display = std::env::var("DISPLAY").ok().filter(|d| !d.trim().is_empty());
    SessionInfo {
        session_type,
        desktop,
        wayland,
        display,
        tools: ToolAvailability {
            wl_copy: command_exists("wl-copy"),
            wl_paste: command_exists("wl-paste"),
            xclip: command_exists("xclip"),
            wtype: command_exists("wtype"),
            ydotool: command_exists("ydotool"),
            x_test: x_test_available(),
            pactl: command_exists("pactl"),
            parecord: command_exists("parecord") || command_exists("pw-record"),
        },
    }
}

#[cfg(not(target_os = "linux"))]
pub fn detect_session() -> SessionInfo {
    let (session_type, desktop) = if cfg!(target_os = "macos") {
        ("macos", "macOS")
    } else {
        ("windows", "Windows")
    };
    SessionInfo {
        session_type: session_type.into(),
        desktop: desktop.into(),
        wayland: false,
        display: None,
        tools: ToolAvailability::default(),
    }
}

/// The keys that paste in the focused app, for user-facing messages.
pub const PASTE_KEYS: &str = if cfg!(target_os = "macos") { "Cmd+V" } else { "Ctrl+V" };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PasteMethod {
    Wtype,
    Ydotool,
    XTest,
    /// Windows SendInput or macOS Quartz events.
    Native,
    None,
}

impl PasteMethod {
    pub fn label(self) -> &'static str {
        match self {
            PasteMethod::Wtype => "wtype virtual keyboard",
            PasteMethod::Ydotool => "ydotool (uinput)",
            PasteMethod::XTest => "X11 XTest",
            PasteMethod::Native => {
                if cfg!(target_os = "macos") {
                    "macOS Quartz events (Cmd+V)"
                } else {
                    "Windows SendInput (Ctrl+V)"
                }
            }
            PasteMethod::None => "clipboard only",
        }
    }
}

pub fn choose_paste_method(session: &SessionInfo, mode: InsertionMode) -> PasteMethod {
    if mode == InsertionMode::ClipboardOnly {
        return PasteMethod::None;
    }
    if !cfg!(target_os = "linux") {
        return PasteMethod::Native;
    }
    if session.wayland {
        if session.tools.wtype {
            return PasteMethod::Wtype;
        }
        if session.tools.ydotool {
            return PasteMethod::Ydotool;
        }
        // XTest reaches XWayland clients only; never claim it is universal.
        return PasteMethod::None;
    }
    if session.tools.x_test && session.display.is_some() {
        PasteMethod::XTest
    } else {
        PasteMethod::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertionOutcome {
    pub method: String,
    pub clipboard: bool,
    pub pasted: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardWrite {
    pub backend: String,
}

#[cfg(target_os = "linux")]
pub fn copy_to_clipboard(text: &str, session: &SessionInfo) -> AppResult<ClipboardWrite> {
    if session.wayland && session.tools.wl_copy {
        write_stdin("wl-copy", &[], text)?;
        return Ok(ClipboardWrite {
            backend: "wl-clipboard".into(),
        });
    }
    if session.tools.xclip {
        write_stdin("xclip", &["-selection", "clipboard", "-in"], text)?;
        return Ok(ClipboardWrite {
            backend: "xclip".into(),
        });
    }
    if session.tools.wl_copy {
        write_stdin("wl-copy", &[], text)?;
        return Ok(ClipboardWrite {
            backend: "wl-clipboard".into(),
        });
    }
    Err(AppError::new(
        "clipboard_unavailable",
        "DictaFox could not access the clipboard.",
    )
    .hint(
        "Install wl-clipboard (Wayland) or xclip (X11): sudo apt install wl-clipboard xclip. Your transcript is still shown in DictaFox and can be selected and copied manually.",
    ))
}

#[cfg(target_os = "linux")]
pub fn read_clipboard(session: &SessionInfo) -> Option<String> {
    if session.wayland && session.tools.wl_paste {
        if let Ok(out) = Command::new("wl-paste")
            .args(["--no-newline"])
            .stderr(Stdio::null())
            .output()
        {
            if out.status.success() {
                return Some(String::from_utf8_lossy(&out.stdout).to_string());
            }
        }
    }
    if session.tools.xclip {
        if let Ok(out) = Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .stderr(Stdio::null())
            .output()
        {
            if out.status.success() {
                return Some(String::from_utf8_lossy(&out.stdout).to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn write_stdin(program: &str, args: &[&str], text: &str) -> AppResult<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            AppError::new(
                "clipboard_unavailable",
                format!("DictaFox could not start {program}."),
            )
            .hint("Install the clipboard utility for your session and try again.")
            .with_detail(err)
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|err| {
            AppError::new(
                "clipboard_unavailable",
                "DictaFox could not write the transcript to the clipboard.",
            )
            .hint("Try copying from the transcript card. Your text has not been lost.")
            .with_detail(err)
        })?;
    }
    let status = child.wait().map_err(|err| {
        AppError::new(
            "clipboard_unavailable",
            "The clipboard utility did not finish.",
        )
        .with_detail(err)
    })?;
    if !status.success() {
        return Err(AppError::new(
            "clipboard_unavailable",
            format!("{program} rejected the clipboard write."),
        )
        .hint("Check that your Wayland/X11 session allows clipboard access."));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn copy_to_clipboard(text: &str, _session: &SessionInfo) -> AppResult<ClipboardWrite> {
    let unavailable = |err: arboard::Error| {
        AppError::new(
            "clipboard_unavailable",
            "DictaFox could not access the clipboard.",
        )
        .hint("Another app may be holding the clipboard. Try again; your transcript is still shown in DictaFox.")
        .with_detail(err)
    };
    let mut clipboard = arboard::Clipboard::new().map_err(unavailable)?;
    clipboard.set_text(text.to_string()).map_err(unavailable)?;
    Ok(ClipboardWrite {
        backend: "system clipboard".into(),
    })
}

#[cfg(not(target_os = "linux"))]
pub fn read_clipboard(_session: &SessionInfo) -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Copy the transcript and, when the desktop permits it, paste it into the
/// previously focused window. The transcript is never discarded: if paste
/// simulation is unavailable it stays on the clipboard with a clear warning.
pub fn deliver(
    text: &str,
    settings: &crate::settings::InsertionSettings,
    session: &SessionInfo,
) -> AppResult<InsertionOutcome> {
    if text.trim().is_empty() {
        return Err(AppError::new(
            "empty_transcript",
            "There is no text to insert.",
        ));
    }
    let method = choose_paste_method(session, settings.mode);
    let previous = if settings.restore_clipboard && method != PasteMethod::None {
        read_clipboard(session)
    } else {
        None
    };
    copy_to_clipboard(text, session)?;

    if method == PasteMethod::None {
        let warning = if !cfg!(target_os = "linux") {
            (settings.mode != InsertionMode::ClipboardOnly).then(|| {
                format!("The transcript is on your clipboard: press {PASTE_KEYS} in the target app.")
            })
        } else if session.wayland {
            Some(
                "Wayland prevents DictaFox from simulating a paste without wtype or ydotool. The transcript is on your clipboard: press Ctrl+V in the target app. Install wtype (wlroots desktops) or ydotool (uinput) to enable automatic insertion."
                    .to_string(),
            )
        } else {
            Some(
                "Automatic paste is unavailable because XTest could not be used. The transcript is on your clipboard: press Ctrl+V in the target app."
                    .to_string(),
            )
        };
        return Ok(InsertionOutcome {
            method: method.label().to_string(),
            clipboard: true,
            pasted: false,
            warning,
        });
    }

    std::thread::sleep(Duration::from_millis(settings.paste_delay_ms.min(2000) as u64));
    let warning = match simulate_paste(method) {
        Ok(()) => None,
        Err(err) => Some(format!(
            "DictaFox could not simulate a paste ({err}). The transcript is on your clipboard: press {PASTE_KEYS} in the target app."
        )),
    };
    let pasted = warning.is_none();

    if pasted {
        if let Some(previous) = previous {
            if previous != text {
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(900));
                    let session = detect_session();
                    let _ = copy_to_clipboard(&previous, &session);
                });
            }
        }
    }

    Ok(InsertionOutcome {
        method: method.label().to_string(),
        clipboard: true,
        pasted,
        warning,
    })
}

#[cfg(target_os = "linux")]
fn simulate_paste(method: PasteMethod) -> Result<(), String> {
    let status = match method {
        PasteMethod::Wtype => Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
            .status(),
        PasteMethod::Ydotool => return ydotool_ctrl_v(),
        PasteMethod::XTest => {
            return unsafe {
                let x11 = X11::load()?;
                x11.press_ctrl_v()
            };
        }
        PasteMethod::Native | PasteMethod::None => return Err("no paste method".into()),
    };
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("paste command exited with {status}")),
        Err(err) => Err(err.to_string()),
    }
}

/// ydotool 1.x takes raw keycodes (`29:1` = Ctrl down, `47` = V) while the
/// 0.1.x releases still packaged by Debian/Ubuntu only accept `key ctrl+v`.
#[cfg(target_os = "linux")]
fn ydotool_ctrl_v() -> Result<(), String> {
    let run = |args: &[&str]| {
        Command::new("ydotool")
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
    };
    let modern = run(&["key", "29:1", "47:1", "47:0", "29:0"]).map_err(|err| err.to_string())?;
    if modern.status.success() {
        return Ok(());
    }
    let legacy = run(&["key", "ctrl+v"]).map_err(|err| err.to_string())?;
    if legacy.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&legacy.stderr);
    let stderr = stderr.trim();
    Err(if stderr.is_empty() {
        format!("ydotool exited with {}", legacy.status)
    } else {
        format!("ydotool: {stderr} (it needs write access to /dev/uinput)")
    })
}

#[cfg(windows)]
fn simulate_paste(_method: PasteMethod) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
    };
    const VK_V: u16 = 0x56;
    let key = |vk: u16, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err("Windows blocked the simulated keystroke (the target app may run as administrator)".into())
    }
}

#[cfg(target_os = "macos")]
fn simulate_paste(_method: PasteMethod) -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    if !accessibility_trusted() {
        return Err("Accessibility permission is required".into());
    }
    const KEY_V: u16 = 9;
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "could not create a keyboard event source".to_string())?;
    for down in [true, false] {
        let event = CGEvent::new_keyboard_event(source.clone(), KEY_V, down)
            .map_err(|_| "could not create a keyboard event".to_string())?;
        event.set_flags(CGEventFlags::CGEventFlagCommand);
        event.post(CGEventTapLocation::HID);
    }
    Ok(())
}

/// macOS only lets apps synthesise keystrokes once the user grants
/// Accessibility access. Passing `prompt` shows the system dialog.
#[cfg(target_os = "macos")]
pub fn accessibility_trusted() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_trusted() -> bool {
    true
}

#[cfg(target_os = "linux")]
pub fn x_test_available() -> bool {
    unsafe { X11::load().is_ok() }
}

#[cfg(not(target_os = "linux"))]
pub fn x_test_available() -> bool {
    false
}

/// X11 keyboard-state reader used for push-to-talk release detection, where the
/// global shortcut plugin does not deliver key-release events.
#[cfg(target_os = "linux")]
pub struct X11Keyboard {
    _libx11: libloading::Library,
    display: *mut c_void,
    x_close_display: unsafe extern "C" fn(*mut c_void) -> i32,
    x_string_to_keysym: unsafe extern "C" fn(*const c_char) -> u64,
    x_keysym_to_keycode: unsafe extern "C" fn(*mut c_void, u64) -> u8,
    x_query_keymap: unsafe extern "C" fn(*mut c_void, *mut [c_char; 32]) -> i32,
}

#[cfg(target_os = "linux")]
impl X11Keyboard {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let libx11 = libloading::Library::new("libX11.so.6")
                .map_err(|err| format!("libX11.so.6: {err}"))?;
            let x_open_display: unsafe extern "C" fn(*const c_char) -> *mut c_void =
                *libx11
                    .get(b"XOpenDisplay\0")
                    .map_err(|err| format!("XOpenDisplay: {err}"))?;
            let x_close_display = *libx11
                .get(b"XCloseDisplay\0")
                .map_err(|err| format!("XCloseDisplay: {err}"))?;
            let x_string_to_keysym = *libx11
                .get(b"XStringToKeysym\0")
                .map_err(|err| format!("XStringToKeysym: {err}"))?;
            let x_keysym_to_keycode = *libx11
                .get(b"XKeysymToKeycode\0")
                .map_err(|err| format!("XKeysymToKeycode: {err}"))?;
            let x_query_keymap = *libx11
                .get(b"XQueryKeymap\0")
                .map_err(|err| format!("XQueryKeymap: {err}"))?;
            let display = x_open_display(std::ptr::null());
            if display.is_null() {
                return Err("X11 display is unavailable".into());
            }
            Ok(Self {
                _libx11: libx11,
                display,
                x_close_display,
                x_string_to_keysym,
                x_keysym_to_keycode,
                x_query_keymap,
            })
        }
    }

    pub fn is_key_down(&self, keysym_name: &str) -> Result<bool, String> {
        unsafe {
            let name = CString::new(keysym_name)
                .map_err(|_| "invalid keysym name".to_string())?;
            let keysym = (self.x_string_to_keysym)(name.as_ptr());
            if keysym == 0 {
                return Err(format!("unknown keysym {keysym_name}"));
            }
            let keycode = (self.x_keysym_to_keycode)(self.display, keysym);
            if keycode == 0 {
                return Err(format!("no keycode for {keysym_name}"));
            }
            let mut keys = [0 as c_char; 32];
            if (self.x_query_keymap)(self.display, &mut keys) == 0 {
                return Err("XQueryKeymap failed".into());
            }
            let index = (keycode / 8) as usize;
            let mask = 1u8 << (keycode % 8);
            Ok((keys[index] as u8 & mask) != 0)
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for X11Keyboard {
    fn drop(&mut self) {
        unsafe {
            (self.x_close_display)(self.display);
        }
    }
}

#[cfg(target_os = "linux")]
struct X11 {
    _libx11: libloading::Library,
    _libxtst: libloading::Library,
    display: *mut c_void,
    x_close_display: unsafe extern "C" fn(*mut c_void) -> i32,
    x_string_to_keysym: unsafe extern "C" fn(*const c_char) -> u64,
    x_keysym_to_keycode: unsafe extern "C" fn(*mut c_void, u64) -> u8,
    x_test_fake_key_event: unsafe extern "C" fn(*mut c_void, u32, i32, u64) -> i32,
    x_sync: unsafe extern "C" fn(*mut c_void, i32) -> i32,
}

#[cfg(target_os = "linux")]
impl X11 {
    unsafe fn load() -> Result<Self, String> {
        let libx11 = libloading::Library::new("libX11.so.6")
            .map_err(|err| format!("libX11.so.6: {err}"))?;
        let libxtst = libloading::Library::new("libXtst.so.6")
            .map_err(|err| format!("libXtst.so.6: {err}"))?;
        let x_open_display: unsafe extern "C" fn(*const c_char) -> *mut c_void =
            *libx11
                .get(b"XOpenDisplay\0")
                .map_err(|err| format!("XOpenDisplay: {err}"))?;
        let x_close_display = *libx11
            .get(b"XCloseDisplay\0")
            .map_err(|err| format!("XCloseDisplay: {err}"))?;
        let x_string_to_keysym = *libx11
            .get(b"XStringToKeysym\0")
            .map_err(|err| format!("XStringToKeysym: {err}"))?;
        let x_keysym_to_keycode = *libx11
            .get(b"XKeysymToKeycode\0")
            .map_err(|err| format!("XKeysymToKeycode: {err}"))?;
        let x_sync = *libx11
            .get(b"XSync\0")
            .map_err(|err| format!("XSync: {err}"))?;
        let x_test_fake_key_event = *libxtst
            .get(b"XTestFakeKeyEvent\0")
            .map_err(|err| format!("XTestFakeKeyEvent: {err}"))?;

        let display = x_open_display(std::ptr::null());
        if display.is_null() {
            return Err("X11 display is unavailable (DISPLAY unset or rejected)".into());
        }
        Ok(Self {
            _libx11: libx11,
            _libxtst: libxtst,
            display,
            x_close_display,
            x_string_to_keysym,
            x_keysym_to_keycode,
            x_test_fake_key_event,
            x_sync,
        })
    }

    unsafe fn keycode(&self, name: &str) -> Result<u8, String> {
        let c_name = CString::new(name).map_err(|_| "invalid keysym".to_string())?;
        let keysym = (self.x_string_to_keysym)(c_name.as_ptr());
        if keysym == 0 {
            return Err(format!("unknown keysym {name}"));
        }
        let code = (self.x_keysym_to_keycode)(self.display, keysym);
        if code == 0 {
            return Err(format!("no keycode for {name}"));
        }
        Ok(code)
    }

    unsafe fn press_ctrl_v(&self) -> Result<(), String> {
        let ctrl = self.keycode("Control_L")?;
        let v = self.keycode("v")?;
        (self.x_test_fake_key_event)(self.display, ctrl as u32, 1, 0);
        (self.x_test_fake_key_event)(self.display, v as u32, 1, 0);
        (self.x_test_fake_key_event)(self.display, v as u32, 0, 0);
        (self.x_test_fake_key_event)(self.display, ctrl as u32, 0, 0);
        (self.x_sync)(self.display, 0);
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for X11 {
    fn drop(&mut self) {
        unsafe {
            (self.x_close_display)(self.display);
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    fn session(wayland: bool, wtype: bool, ydotool: bool, x_test: bool) -> SessionInfo {
        SessionInfo {
            session_type: if wayland { "wayland".into() } else { "x11".into() },
            desktop: "GNOME".into(),
            wayland,
            display: Some(":0".into()),
            tools: ToolAvailability {
                wl_copy: true,
                wl_paste: true,
                xclip: true,
                wtype,
                ydotool,
                x_test,
                pactl: true,
                parecord: true,
            },
        }
    }

    #[test]
    fn wayland_prefers_native_virtual_keyboards() {
        assert_eq!(
            choose_paste_method(&session(true, true, true, true), InsertionMode::Auto),
            PasteMethod::Wtype
        );
        assert_eq!(
            choose_paste_method(&session(true, false, true, true), InsertionMode::Auto),
            PasteMethod::Ydotool
        );
        assert_eq!(
            choose_paste_method(&session(true, false, false, true), InsertionMode::Auto),
            PasteMethod::None,
            "XTest must not be claimed as working globally on Wayland"
        );
    }

    #[test]
    fn x11_uses_xtest_when_available() {
        assert_eq!(
            choose_paste_method(&session(false, false, false, true), InsertionMode::Auto),
            PasteMethod::XTest
        );
        assert_eq!(
            choose_paste_method(&session(false, false, false, false), InsertionMode::Auto),
            PasteMethod::None
        );
    }

    #[test]
    fn clipboard_only_mode_never_simulates_keys() {
        assert_eq!(
            choose_paste_method(&session(false, true, true, true), InsertionMode::ClipboardOnly),
            PasteMethod::None
        );
    }

    #[test]
    fn session_detection_reports_a_real_environment() {
        let info = detect_session();
        assert!(!info.session_type.is_empty());
        assert!(!info.desktop.is_empty());
        eprintln!(
            "session={} desktop={} wayland={} tools={:?}",
            info.session_type, info.desktop, info.wayland, info.tools
        );
    }
}
