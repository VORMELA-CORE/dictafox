use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::error::{AppError, AppResult};
use crate::storage::Store;

const SETTINGS_KEY: &str = "settings";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutMode {
    /// Hold the shortcut to record, release to transcribe.
    PushToTalk,
    /// Press once to start, press again to stop.
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InsertionMode {
    /// Simulate a paste when the desktop allows it, otherwise copy and warn.
    Auto,
    /// Never simulate keys; leave the text on the clipboard.
    ClipboardOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Profile {
    General,
    Coding,
    Email,
    Messaging,
    Academic,
    Professional,
}

impl Profile {
    pub fn all() -> [Profile; 6] {
        [
            Profile::General,
            Profile::Coding,
            Profile::Email,
            Profile::Messaging,
            Profile::Academic,
            Profile::Professional,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Profile::General => "General",
            Profile::Coding => "Coding",
            Profile::Email => "Email",
            Profile::Messaging => "Messaging",
            Profile::Academic => "Academic",
            Profile::Professional => "Professional",
        }
    }

    /// The refinement applied when the user chooses "Follow profile".
    pub fn default_refinement(self) -> RefinementMode {
        match self {
            Profile::General => RefinementMode::Clean,
            Profile::Coding => RefinementMode::Developer,
            Profile::Email => RefinementMode::Email,
            Profile::Messaging => RefinementMode::Casual,
            Profile::Academic => RefinementMode::Clean,
            Profile::Professional => RefinementMode::Professional,
        }
    }
}

impl FromStr for Profile {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "general" => Ok(Profile::General),
            "coding" => Ok(Profile::Coding),
            "email" => Ok(Profile::Email),
            "messaging" => Ok(Profile::Messaging),
            "academic" => Ok(Profile::Academic),
            "professional" => Ok(Profile::Professional),
            other => Err(AppError::new(
                "invalid_profile",
                format!("Unknown profile \"{other}\"."),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RefinementMode {
    /// Follow the active context profile.
    Profile,
    Raw,
    Clean,
    Professional,
    Casual,
    Email,
    Technical,
    Developer,
}

impl RefinementMode {
    pub fn all() -> [RefinementMode; 8] {
        [
            RefinementMode::Profile,
            RefinementMode::Raw,
            RefinementMode::Clean,
            RefinementMode::Professional,
            RefinementMode::Casual,
            RefinementMode::Email,
            RefinementMode::Technical,
            RefinementMode::Developer,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            RefinementMode::Profile => "Follow profile",
            RefinementMode::Raw => "Raw transcript",
            RefinementMode::Clean => "Clean",
            RefinementMode::Professional => "Professional",
            RefinementMode::Casual => "Casual",
            RefinementMode::Email => "Email",
            RefinementMode::Technical => "Technical",
            RefinementMode::Developer => "Developer",
        }
    }

    pub fn resolve(self, profile: Profile) -> RefinementMode {
        match self {
            RefinementMode::Profile => profile.default_refinement(),
            other => other,
        }
    }
}

impl FromStr for RefinementMode {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "profile" => Ok(RefinementMode::Profile),
            "raw" => Ok(RefinementMode::Raw),
            "clean" => Ok(RefinementMode::Clean),
            "professional" => Ok(RefinementMode::Professional),
            "casual" => Ok(RefinementMode::Casual),
            "email" => Ok(RefinementMode::Email),
            "technical" => Ok(RefinementMode::Technical),
            "developer" => Ok(RefinementMode::Developer),
            other => Err(AppError::new(
                "invalid_refinement",
                format!("Unknown refinement mode \"{other}\"."),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderSettings {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    /// Prefer the `DICTAFOX_OPENAI_API_KEY` environment variable in shared or
    /// multi-user machines. Stored locally with 0600 permissions otherwise.
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: None,
            timeout_seconds: 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InsertionSettings {
    pub mode: InsertionMode,
    pub restore_clipboard: bool,
    pub paste_delay_ms: u32,
}

impl Default for InsertionSettings {
    fn default() -> Self {
        Self {
            mode: InsertionMode::Auto,
            restore_clipboard: true,
            paste_delay_ms: 120,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub onboarding_complete: bool,
    pub shortcut: String,
    pub shortcut_mode: ShortcutMode,
    pub microphone: Option<String>,
    pub max_recording_seconds: u32,
    pub vad_enabled: bool,
    pub input_gain: f32,
    pub model: String,
    pub custom_model_path: Option<String>,
    pub language: String,
    /// 0 means "let whisper.cpp choose based on available cores".
    pub threads: u32,
    pub keep_model_loaded: bool,
    pub profile: Profile,
    pub refinement: RefinementMode,
    pub dictionary_enabled: bool,
    pub provider: ProviderSettings,
    pub persist_history: bool,
    pub save_audio: bool,
    pub theme: Theme,
    pub reduce_motion: bool,
    pub insertion: InsertionSettings,
    pub developer_mode: bool,
}

/// Ctrl+Space switches input sources on macOS, so the Mac default uses
/// Option+Space instead.
pub fn default_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "Alt+Space"
    } else {
        "Ctrl+Space"
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            shortcut: default_shortcut().to_string(),
            shortcut_mode: ShortcutMode::PushToTalk,
            microphone: None,
            max_recording_seconds: 120,
            vad_enabled: true,
            input_gain: 1.0,
            model: "base.en".to_string(),
            custom_model_path: None,
            language: "auto".to_string(),
            threads: 0,
            keep_model_loaded: true,
            profile: Profile::General,
            refinement: RefinementMode::Profile,
            dictionary_enabled: true,
            provider: ProviderSettings::default(),
            persist_history: true,
            save_audio: false,
            theme: Theme::Dark,
            reduce_motion: false,
            insertion: InsertionSettings::default(),
            developer_mode: false,
        }
    }
}

impl Settings {
    /// Clamp unsafe values and reject combinations that cannot work, so a
    /// malformed settings file can never break dictation at runtime.
    pub fn validate(&mut self) -> AppResult<()> {
        self.shortcut = self.shortcut.trim().to_string();
        if self.shortcut.is_empty() {
            self.shortcut = default_shortcut().to_string();
        }
        self.max_recording_seconds = self.max_recording_seconds.clamp(5, 600);
        self.input_gain = self.input_gain.clamp(0.5, 3.0);
        self.threads = self.threads.min(64);
        self.insertion.paste_delay_ms = self.insertion.paste_delay_ms.min(2000);
        self.language = self.language.trim().to_string();
        if self.language.is_empty() {
            self.language = "auto".to_string();
        }
        self.model = self.model.trim().to_string();
        if self.model.is_empty() {
            self.model = "base.en".to_string();
        }
        self.custom_model_path = self
            .custom_model_path
            .take()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty());
        if let Some(path) = &self.custom_model_path {
            if !std::path::Path::new(path).is_file() {
                return Err(AppError::new(
                    "model_not_found",
                    "The custom model file could not be found.",
                )
                .hint("Choose an existing ggml whisper.cpp model file, or clear the custom path."));
            }
        }
        self.provider.base_url = self.provider.base_url.trim().trim_end_matches('/').to_string();
        if self.provider.base_url.is_empty() {
            return Err(AppError::new(
                "provider_invalid",
                "The text refinement provider has no endpoint URL.",
            )
            .hint("Enter an OpenAI-compatible base URL such as https://api.openai.com/v1."));
        }
        self.provider.model = self.provider.model.trim().to_string();
        self.provider.timeout_seconds = self.provider.timeout_seconds.clamp(3, 120);
        if self.provider.enabled && !self.provider_has_key() {
            return Err(AppError::new(
                "provider_missing_key",
                "Cloud text refinement is enabled but no API key is configured.",
            )
            .hint("Add an API key, set DICTAFOX_OPENAI_API_KEY, or turn cloud refinement off. Local transcription is unaffected."));
        }
        Ok(())
    }

    pub fn provider_has_key(&self) -> bool {
        provider_api_key(&self.provider).is_some()
    }

    pub fn effective_refinement(&self) -> RefinementMode {
        self.refinement.resolve(self.profile)
    }
}

pub fn provider_api_key(provider: &ProviderSettings) -> Option<String> {
    if let Ok(key) = std::env::var("DICTAFOX_OPENAI_API_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Some(key);
        }
    }
    provider
        .api_key
        .as_ref()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
}

pub fn load_settings(store: &Store) -> AppResult<Settings> {
    let raw: Option<String> = store.with_conn(|conn| {
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [SETTINGS_KEY],
            |row| row.get(0),
        )
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .map(|value: Option<String>| value)
    })?;

    let Some(raw) = raw else {
        return Ok(Settings::default());
    };
    match serde_json::from_str::<Settings>(&raw) {
        Ok(mut settings) => {
            // A stale or hand-edited file must not brick the app, but one bad
            // field (a moved custom model, a removed API key) must not wipe the
            // user's whole configuration and send them back to onboarding.
            if let Err(err) = settings.validate() {
                crate::logging::warn(format!("stored settings need repair: {err}"));
                repair(&mut settings);
                if let Err(err) = settings.validate() {
                    crate::logging::warn(format!("stored settings rejected: {err}"));
                    return Ok(Settings {
                        onboarding_complete: settings.onboarding_complete,
                        ..Settings::default()
                    });
                }
            }
            Ok(settings)
        }
        Err(err) => {
            crate::logging::warn(format!(
                "stored settings were unreadable, using defaults: {err}"
            ));
            Ok(Settings::default())
        }
    }
}

/// Reset only the fields that `validate` can reject.
fn repair(settings: &mut Settings) {
    let custom_missing = settings
        .custom_model_path
        .as_deref()
        .map(|path| !std::path::Path::new(path.trim()).is_file())
        .unwrap_or(false);
    if custom_missing {
        settings.custom_model_path = None;
    }
    if settings.provider.base_url.trim().is_empty() {
        settings.provider.base_url = ProviderSettings::default().base_url;
    }
    if settings.provider.enabled && !settings.provider_has_key() {
        settings.provider.enabled = false;
    }
}

pub fn save_settings(store: &Store, mut settings: Settings) -> AppResult<Settings> {
    settings.validate()?;
    let json = serde_json::to_string(&settings)?;
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![SETTINGS_KEY, json],
        )
    })?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::in_memory().unwrap()
    }

    #[test]
    fn one_invalid_field_does_not_wipe_stored_settings() {
        let store = store();
        let mut settings = Settings::default();
        settings.onboarding_complete = true;
        settings.shortcut = "Ctrl+Shift+R".into();
        settings.custom_model_path = None;
        save_settings(&store, settings).unwrap();
        // Simulate a custom model that was moved after it was saved.
        store
            .with_conn(|conn| {
                conn.execute(
                    "UPDATE settings SET value = json_set(value, '$.customModelPath', '/gone/model.bin')",
                    [],
                )
            })
            .unwrap();
        let loaded = load_settings(&store).unwrap();
        assert!(loaded.onboarding_complete);
        assert_eq!(loaded.shortcut, "Ctrl+Shift+R");
        assert!(loaded.custom_model_path.is_none());
    }

    #[test]
    fn defaults_are_sane_and_valid() {
        let mut settings = Settings::default();
        assert!(settings.validate().is_ok());
        assert_eq!(settings.shortcut, default_shortcut());
        assert_eq!(settings.effective_refinement(), RefinementMode::Clean);
        assert!(!settings.provider.enabled);
    }

    #[test]
    fn settings_roundtrip_through_sqlite() {
        let store = store();
        let mut settings = Settings::default();
        settings.model = "small.en".into();
        settings.profile = Profile::Coding;
        settings.max_recording_seconds = 45;
        settings.persist_history = false;
        let saved = save_settings(&store, settings.clone()).unwrap();
        assert_eq!(saved.model, "small.en");
        let loaded = load_settings(&store).unwrap();
        assert_eq!(loaded, saved);
        assert_eq!(loaded.effective_refinement(), RefinementMode::Developer);
    }

    #[test]
    fn validation_clamps_values() {
        let mut settings = Settings::default();
        settings.max_recording_seconds = 99_999;
        settings.input_gain = 99.0;
        settings.threads = 400;
        settings.insertion.paste_delay_ms = 9_999;
        settings.shortcut = "  ".into();
        settings.language = "  ".into();
        settings.validate().unwrap();
        assert_eq!(settings.max_recording_seconds, 600);
        assert_eq!(settings.input_gain, 3.0);
        assert_eq!(settings.threads, 64);
        assert_eq!(settings.insertion.paste_delay_ms, 2000);
        assert_eq!(settings.shortcut, default_shortcut());
        assert_eq!(settings.language, "auto");
    }

    #[test]
    fn corrupt_settings_fall_back_to_defaults() {
        let store = store();
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO settings (key, value) VALUES ('settings', '{not json')",
                    [],
                )
            })
            .unwrap();
        let loaded = load_settings(&store).unwrap();
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn cloud_refinement_requires_a_key() {
        let mut settings = Settings::default();
        settings.provider.enabled = true;
        settings.provider.api_key = None;
        // Make the test independent of any developer environment variable.
        std::env::remove_var("DICTAFOX_OPENAI_API_KEY");
        let err = settings.validate().unwrap_err();
        assert_eq!(err.code, "provider_missing_key");
    }

    #[test]
    fn missing_custom_model_path_is_rejected() {
        let mut settings = Settings::default();
        settings.custom_model_path = Some("/nonexistent/model.bin".into());
        let err = settings.validate().unwrap_err();
        assert_eq!(err.code, "model_not_found");
    }

    #[test]
    fn enums_serialise_as_camel_case_for_typescript() {
        assert_eq!(
            serde_json::to_string(&RefinementMode::Developer).unwrap(),
            "\"developer\""
        );
        assert_eq!(
            serde_json::to_string(&ShortcutMode::PushToTalk).unwrap(),
            "\"pushToTalk\""
        );
        assert_eq!(
            serde_json::to_string(&InsertionMode::ClipboardOnly).unwrap(),
            "\"clipboardOnly\""
        );
    }
}
