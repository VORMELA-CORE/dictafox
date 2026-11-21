use std::path::{Path, PathBuf};

use dictafox_core::whisper::{TranscribeOptions, WhisperEngine};

/// Runs only when DICTAFOX_TEST_MODEL points at a real ggml model. This keeps
/// `cargo test` usable offline while allowing a genuine end-to-end check:
///
///   DICTAFOX_TEST_MODEL=~/.local/share/dictafox/models/ggml-tiny.en.bin \
///   DICTAFOX_TEST_WAV=/tmp/dictafox-test/jfk.wav \
///   cargo test -p dictafox-core --test real_transcription -- --nocapture
#[test]
fn transcribes_real_speech() {
    let Some(model) = std::env::var_os("DICTAFOX_TEST_MODEL").map(PathBuf::from) else {
        eprintln!("skipping real transcription test: DICTAFOX_TEST_MODEL is not set");
        return;
    };
    let wav = std::env::var_os("DICTAFOX_TEST_WAV")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/dictafox-test/jfk.wav"));
    assert!(model.is_file(), "model not found at {}", model.display());
    assert!(wav.is_file(), "test wav not found at {}", wav.display());
    // Real whisper.cpp files must pass the same header check that downloads
    // and custom model paths go through.
    dictafox_core::models::inspect_ggml(&model).expect("real model must pass the ggml header check");

    let engine = WhisperEngine::new();
    let transcript = engine
        .transcribe_file(
            &wav,
            &model,
            &TranscribeOptions {
                language: "en".into(),
                threads: 4,
                translate: false,
            },
        )
        .expect("real transcription should succeed");

    println!(
        "transcript: {:?}\nsegments={} audio={}ms elapsed={}ms",
        transcript.text, transcript.segments, transcript.audio_ms, transcript.elapsed_ms
    );
    assert!(!transcript.text.trim().is_empty());
    assert!(
        transcript.elapsed_ms > 0,
        "engine must report measured time"
    );
    let text = transcript.text.to_lowercase();
    assert!(
        text.contains("fellow americans") || text.contains("country"),
        "unexpected transcript: {}",
        transcript.text
    );

    // A second call must reuse the loaded context instead of reloading it.
    let again = engine
        .transcribe_file(
            &wav,
            &model,
            &TranscribeOptions {
                language: "en".into(),
                threads: 4,
                translate: false,
            },
        )
        .expect("second transcription should succeed");
    assert_eq!(again.text, transcript.text);
    assert_eq!(engine.loaded_path().as_deref(), Some(model.as_path()));
    let _ = Path::new(&model);
}
