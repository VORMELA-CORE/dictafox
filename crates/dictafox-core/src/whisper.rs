use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::audio;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    /// `auto` lets whisper.cpp detect the language.
    pub language: String,
    /// 0 chooses a sensible value from the available cores.
    pub threads: u32,
    pub translate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub text: String,
    pub language: String,
    pub segments: usize,
    pub audio_ms: u64,
    pub elapsed_ms: u64,
}

struct LoadedModel {
    path: PathBuf,
    context: WhisperContext,
}

/// Keeps a whisper.cpp context resident between dictations. Loading a model is
/// the expensive part; reusing it is what makes push-to-talk feel instant.
pub struct WhisperEngine {
    loaded: Mutex<Option<LoadedModel>>,
}

impl Default for WhisperEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WhisperEngine {
    pub fn new() -> Self {
        Self {
            loaded: Mutex::new(None),
        }
    }

    pub fn loaded_path(&self) -> Option<PathBuf> {
        self.loaded
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|model| model.path.clone()))
    }

    pub fn unload(&self) {
        if let Ok(mut guard) = self.loaded.lock() {
            *guard = None;
        }
    }

    fn ensure_loaded(&self, path: &Path) -> AppResult<()> {
        let mut guard = self.loaded.lock().map_err(|_| {
            AppError::new(
                "transcription_unavailable",
                "The speech engine is in an unusable state.",
            )
            .hint("Restart DictaFox.")
        })?;
        if guard.as_ref().map(|model| model.path == path).unwrap_or(false) {
            return Ok(());
        }
        // Drop the previous model before loading a new one so two large model
        // allocations never coexist.
        *guard = None;
        let display = path.display().to_string();
        let context = WhisperContext::new_with_params(&display, WhisperContextParameters::default())
            .map_err(|err| {
                AppError::new(
                    "model_load_failed",
                    "The speech model could not be loaded.",
                )
                .hint("Re-download the model in Settings → Models. Corrupt or incomplete files are rejected.")
                .with_detail(format!("{err:?}"))
            })?;
        *guard = Some(LoadedModel {
            path: path.to_path_buf(),
            context,
        });
        Ok(())
    }

    pub fn transcribe_file(
        &self,
        wav: &Path,
        model: &Path,
        options: &TranscribeOptions,
    ) -> AppResult<Transcript> {
        let (mut samples, rate) = audio::read_wav(wav)?;
        if rate != audio::SAMPLE_RATE {
            samples = audio::resample(&samples, rate, audio::SAMPLE_RATE);
        }
        self.transcribe_samples(&samples, model, options)
    }

    pub fn transcribe_samples(
        &self,
        samples: &[f32],
        model: &Path,
        options: &TranscribeOptions,
    ) -> AppResult<Transcript> {
        if samples.len() < (audio::SAMPLE_RATE as usize / 10) {
            return Err(AppError::new(
                "recording_too_short",
                "There was not enough audio to transcribe.",
            )
            .hint("Hold the shortcut a little longer and speak clearly into the microphone."));
        }
        let started = Instant::now();
        self.ensure_loaded(model)?;

        let guard = self.loaded.lock().map_err(|_| {
            AppError::new("transcription_unavailable", "The speech engine is unavailable.")
                .hint("Restart DictaFox.")
        })?;
        let loaded = guard.as_ref().ok_or_else(|| {
            AppError::new("model_load_failed", "The speech model is not loaded.")
        })?;
        let mut state = loaded.context.create_state().map_err(|err| {
            AppError::new(
                "transcription_failed",
                "The speech engine could not start a transcription.",
            )
            .hint("Try again. If it repeats, choose a smaller model in Settings → Models.")
            .with_detail(err.to_string())
        })?;

        let thread_count = if options.threads == 0 {
            std::thread::available_parallelism()
                .map(|n| (n.get() as u32).saturating_sub(1).clamp(1, 8))
                .unwrap_or(4)
        } else {
            options.threads.clamp(1, 64)
        };

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let language = options.language.trim();
        let language = if language.is_empty() || language == "auto" {
            None
        } else {
            Some(language)
        };
        params.set_language(language);
        params.set_translate(options.translate);
        params.set_n_threads(thread_count as i32);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // Dictation is one self-contained utterance; avoiding cross-segment
        // context prevents repetitive hallucination loops on silence.
        params.set_no_context(true);
        params.set_suppress_blank(true);
        params.set_temperature(0.0);

        state.full(params, samples).map_err(|err| {
            AppError::new(
                "transcription_failed",
                "The speech engine could not transcribe this recording.",
            )
            .hint("Try again with a shorter recording, or switch to a smaller model in Settings → Models.")
            .detail(err.to_string())
        })?;

        let segments = state.full_n_segments().map_err(|err| {
            AppError::new(
                "transcription_failed",
                "The transcription could not be read from the speech engine.",
            )
            .with_detail(err.to_string())
        })?;
        let mut text = String::new();
        for index in 0..segments {
            if let Ok(segment) = state.full_get_segment_text(index) {
                text.push_str(segment.trim());
                text.push(' ');
            }
        }
        drop(state);
        drop(guard);

        let cleaned = clean_output(&text);
        let elapsed_ms = started.elapsed().as_millis() as u64;
        Ok(Transcript {
            text: cleaned,
            language: options.language.clone(),
            segments: segments.max(0) as usize,
            audio_ms: (samples.len() as u64 * 1000) / audio::SAMPLE_RATE as u64,
            elapsed_ms,
        })
    }
}

/// whisper.cpp prefixes segments with spaces and occasionally emits bracketed
/// sound tags such as `[BLANK_AUDIO]`; neither belongs in dictated text.
pub fn clean_output(raw: &str) -> String {
    let mut text = raw.to_string();
    for tag in [
        "[BLANK_AUDIO]",
        "[MUSIC]",
        "[APPLAUSE]",
        "(silence)",
        "[silence]",
        "[NO SPEECH DETECTED]",
    ] {
        text = text.replace(tag, " ");
    }
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    joined
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" ?", "?")
        .replace(" !", "!")
        .replace(" ;", ";")
        .replace(" :", ":")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_cleanup_removes_whisper_artifacts() {
        assert_eq!(
            clean_output("  Hello   world . [BLANK_AUDIO] "),
            "Hello world."
        );
        assert_eq!(clean_output("[MUSIC]"), "");
        assert_eq!(clean_output(" a , b ? "), "a, b?");
    }

    #[test]
    fn too_short_audio_is_rejected_before_touching_the_model() {
        let engine = WhisperEngine::new();
        let err = engine
            .transcribe_samples(
                &vec![0.0f32; 10],
                Path::new("/nonexistent/model.bin"),
                &TranscribeOptions {
                    language: "auto".into(),
                    threads: 0,
                    translate: false,
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "recording_too_short");
    }

    #[test]
    fn engine_starts_unloaded() {
        assert!(WhisperEngine::new().loaded_path().is_none());
    }
}
