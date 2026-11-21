//! DictaFox core (Vormela Core): a local-first dictation engine for Linux,
//! Windows and macOS.
//!
//! The crate deliberately contains no UI and no Tauri types. It owns audio
//! capture, whisper.cpp inference, text refinement, text delivery,
//! settings, history and the personal dictionary so every piece can be tested
//! without a graphical session.

pub mod audio;
pub mod dictionary;
pub mod error;
pub mod history;
pub mod inject;
pub mod logging;
pub mod models;
pub mod paths;
pub mod platform;
pub mod refine;
pub mod service;
pub mod settings;
pub mod storage;
pub mod whisper;

pub use error::{AppError, AppResult};
pub use service::Dictafox;
