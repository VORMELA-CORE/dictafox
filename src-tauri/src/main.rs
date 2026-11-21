#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    attach_parent_console();
    match dictafox_lib::cli::entrypoint() {
        dictafox_lib::cli::Entrypoint::Run(initial) => dictafox_lib::run(initial),
        dictafox_lib::cli::Entrypoint::Exit => {}
    }
}

/// Release builds on Windows are GUI-subsystem executables with no console.
/// When started from a terminal with options (`--help`, `--toggle`), attach
/// to that terminal so output is visible.
#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    if std::env::args().len() > 1 {
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}
