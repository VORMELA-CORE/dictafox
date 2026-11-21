use serde::{Deserialize, Serialize};

/// A user-presentable error. `detail` is for local diagnostics and developer
/// mode only; the UI shows `message` and `hint`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
    pub detail: Option<String>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            hint: None,
            detail: None,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_detail(mut self, detail: impl std::fmt::Display) -> Self {
        self.detail = Some(detail.to_string());
        self
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::new("io_error", "A local file operation failed.")
            .hint("Check that your data directory is writable and has free space.")
            .with_detail(err)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::new("storage_error", "DictaFox could not read or write its local database.")
            .hint("Check free disk space and permissions for ~/.local/share/dictafox.")
            .with_detail(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::new("invalid_data", "DictaFox read data it could not understand.")
            .hint("Reset the affected setting or delete the local database to recover.")
            .with_detail(err)
    }
}

impl From<hound::Error> for AppError {
    fn from(err: hound::Error) -> Self {
        AppError::new("audio_error", "The recorded audio could not be read.")
            .hint("Try recording again. If this repeats, check your microphone in Audio settings.")
            .with_detail(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_serialisable_for_the_frontend() {
        let err = AppError::new("no_microphone", "No microphone was found.")
            .hint("Connect a microphone and refresh.");
        let value = serde_json::to_value(&err).unwrap();
        assert_eq!(value["code"], "no_microphone");
        assert_eq!(value["hint"], "Connect a microphone and refresh.");
        assert!(value.get("detail").unwrap().is_null());
    }

    #[test]
    fn io_errors_do_not_leak_stack_traces_in_the_message() {
        let err = AppError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "x"));
        assert!(!err.message.contains("NotFound"));
        assert!(err.detail.is_some());
    }
}
