use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

use dictafox_core::paths;
use tauri::{AppHandle, Emitter};

use crate::actions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliAction {
    Toggle,
    Start,
    Stop,
    Cancel,
    Show,
    Settings,
}

impl CliAction {
    fn as_wire(self) -> &'static str {
        match self {
            CliAction::Toggle => "toggle",
            CliAction::Start => "start",
            CliAction::Stop => "stop",
            CliAction::Cancel => "cancel",
            CliAction::Show => "show",
            CliAction::Settings => "settings",
        }
    }

    fn from_wire(value: &str) -> Option<Self> {
        match value.trim() {
            "toggle" => Some(CliAction::Toggle),
            "start" => Some(CliAction::Start),
            "stop" => Some(CliAction::Stop),
            "cancel" => Some(CliAction::Cancel),
            "show" => Some(CliAction::Show),
            "settings" => Some(CliAction::Settings),
            _ => None,
        }
    }
}

pub enum Entrypoint {
    Run(Option<CliAction>),
    Exit,
}

pub fn entrypoint() -> Entrypoint {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut action = None;
    for arg in &args {
        match arg.as_str() {
            "--toggle" => action = Some(CliAction::Toggle),
            "--start" => action = Some(CliAction::Start),
            "--stop" => action = Some(CliAction::Stop),
            "--cancel" => action = Some(CliAction::Cancel),
            "--show" => action = Some(CliAction::Show),
            "--settings" => action = Some(CliAction::Settings),
            "--version" | "-V" => {
                println!("DictaFox {}", env!("CARGO_PKG_VERSION"));
                return Entrypoint::Exit;
            }
            "--help" | "-h" => {
                print_help();
                return Entrypoint::Exit;
            }
            other => {
                eprintln!("dictafox: unknown argument {other}\n");
                print_help();
                std::process::exit(2);
            }
        }
    }

    // A running instance owns the microphone and settings, so commands are sent
    // to it over a private local channel rather than starting a second copy.
    // Launching DictaFox again with no option just brings the window back.
    match send(action.unwrap_or(CliAction::Show)) {
        Ok(()) => Entrypoint::Exit,
        Err(_) => Entrypoint::Run(action),
    }
}

fn print_help() {
    println!(
        "DictaFox {} — Speak Naturally. Type Everywhere.\n\n\
         Usage: dictafox [OPTION]\n\n\
         Options:\n\
           --toggle      Start dictation, or stop and transcribe if already recording\n\
           --start       Start recording\n\
           --stop        Stop recording and transcribe\n\
           --cancel      Discard the current recording\n\
           --show        Open the DictaFox dashboard\n\
           --settings    Open the dashboard directly in Settings\n\
           --version     Print the version\n\
           --help        Show this help\n\n\
         With no option, DictaFox opens its dashboard (or shows the running one).\n\
         On Linux Wayland, bind one of the first five options to a custom desktop\n\
         shortcut to get a global hotkey.",
        env!("CARGO_PKG_VERSION")
    );
}

fn handle_line(app: &AppHandle, line: &str) {
    if let Some(action) = CliAction::from_wire(line) {
        dispatch(app, action);
    }
}

#[cfg(unix)]
mod transport {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;

    fn socket_path() -> PathBuf {
        paths::data_dir().join("daemon.sock")
    }

    pub fn send(action: CliAction) -> std::io::Result<()> {
        let mut stream = UnixStream::connect(socket_path())?;
        stream.set_write_timeout(Some(Duration::from_millis(800)))?;
        stream.write_all(action.as_wire().as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()
    }

    pub fn serve(app: AppHandle) {
        let path = socket_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if path.exists() {
            // Remove a stale socket left by a crash; only if nobody is listening.
            if UnixStream::connect(&path).is_err() {
                let _ = std::fs::remove_file(&path);
            } else {
                dictafox_core::logging::warn("another DictaFox instance is already running");
                return;
            }
        }
        let listener = match UnixListener::bind(&path) {
            Ok(listener) => listener,
            Err(err) => {
                dictafox_core::logging::warn(format!("could not bind CLI socket: {err}"));
                return;
            }
        };
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let app = app.clone();
                std::thread::spawn(move || {
                    let mut line = String::new();
                    if BufReader::new(stream).read_line(&mut line).is_ok() {
                        handle_line(&app, &line);
                    }
                });
            }
        });
    }

    pub fn cleanup() {
        let _ = std::fs::remove_file(socket_path());
    }
}

/// Windows has no Unix sockets in std, so the running instance listens on a
/// loopback port and writes `port token` to a user-private file. Clients must
/// echo the random token, so other local users or web pages cannot drive it.
#[cfg(windows)]
mod transport {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::path::PathBuf;

    fn endpoint_path() -> PathBuf {
        paths::data_dir().join("daemon.endpoint")
    }

    fn read_endpoint() -> std::io::Result<(u16, String)> {
        let text = std::fs::read_to_string(endpoint_path())?;
        let mut parts = text.split_whitespace();
        let invalid = || std::io::Error::new(std::io::ErrorKind::InvalidData, "bad endpoint file");
        let port = parts.next().and_then(|p| p.parse().ok()).ok_or_else(invalid)?;
        let token = parts.next().ok_or_else(invalid)?.to_string();
        Ok((port, token))
    }

    fn new_token() -> String {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        (0..4)
            .map(|_| {
                let mut hasher = RandomState::new().build_hasher();
                hasher.write_u128(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or_default(),
                );
                format!("{:016x}", hasher.finish())
            })
            .collect()
    }

    pub fn send(action: CliAction) -> std::io::Result<()> {
        let (port, token) = read_endpoint()?;
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(500))?;
        stream.set_write_timeout(Some(Duration::from_millis(800)))?;
        stream.write_all(format!("{token} {}\n", action.as_wire()).as_bytes())?;
        stream.flush()
    }

    pub fn serve(app: AppHandle) {
        if let Ok((port, _)) = read_endpoint() {
            let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
            if TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok() {
                dictafox_core::logging::warn("another DictaFox instance is already running");
                return;
            }
        }
        let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
            Ok(listener) => listener,
            Err(err) => {
                dictafox_core::logging::warn(format!("could not open CLI endpoint: {err}"));
                return;
            }
        };
        let Ok(port) = listener.local_addr().map(|a| a.port()) else {
            return;
        };
        let token = new_token();
        if let Some(parent) = endpoint_path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(err) = std::fs::write(endpoint_path(), format!("{port} {token}")) {
            dictafox_core::logging::warn(format!("could not write CLI endpoint: {err}"));
            return;
        }
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let app = app.clone();
                let token = token.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut line = String::new();
                    if BufReader::new(stream).read_line(&mut line).is_err() {
                        return;
                    }
                    if let Some((given, action)) = line.trim().split_once(' ') {
                        if given == token {
                            handle_line(&app, action);
                        }
                    }
                });
            }
        });
    }

    pub fn cleanup() {
        let _ = std::fs::remove_file(endpoint_path());
    }
}

fn send(action: CliAction) -> std::io::Result<()> {
    transport::send(action)
}

/// Listen for CLI commands while the app runs.
pub fn serve(app: AppHandle) {
    transport::serve(app)
}

pub fn cleanup_socket() {
    transport::cleanup()
}

pub fn dispatch(app: &AppHandle, action: CliAction) {
    match action {
        CliAction::Toggle => actions::toggle(app),
        CliAction::Start => actions::start(app, dictafox_core::service::RecordingOrigin::Cli),
        CliAction::Stop => actions::stop(app),
        CliAction::Cancel => actions::cancel(app),
        CliAction::Show => actions::show_main(app),
        CliAction::Settings => {
            actions::show_main(app);
            let _ = app.emit("dictafox://navigate", "settings");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_commands_roundtrip() {
        for action in [
            CliAction::Toggle,
            CliAction::Start,
            CliAction::Stop,
            CliAction::Cancel,
            CliAction::Show,
            CliAction::Settings,
        ] {
            assert_eq!(CliAction::from_wire(action.as_wire()), Some(action));
        }
        assert_eq!(CliAction::from_wire("bogus"), None);
    }
}
