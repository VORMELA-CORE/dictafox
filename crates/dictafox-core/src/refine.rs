use serde::Serialize;

use crate::dictionary::{self, DictionaryEntry};
use crate::error::{AppError, AppResult};
use crate::settings::{provider_api_key, Profile, ProviderSettings, RefinementMode, Settings};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefineOutcome {
    pub text: String,
    pub mode: RefinementMode,
    /// `Some` when an optional cloud provider produced the final text.
    pub provider: Option<String>,
    /// A non-fatal problem, e.g. cloud refinement was unreachable. Text is
    /// always returned when this is set.
    pub warning: Option<String>,
}

/// Refine a raw transcript. Local refinement always runs first, so dictation
/// keeps working with no API key, no network and no provider configured.
pub fn refine(settings: &Settings, entries: &[DictionaryEntry], raw: &str) -> RefineOutcome {
    let mode = settings.effective_refinement();
    let source = raw.trim();
    if source.is_empty() {
        return RefineOutcome {
            text: String::new(),
            mode,
            provider: None,
            warning: None,
        };
    }
    if mode == RefinementMode::Raw {
        return RefineOutcome {
            text: source.to_string(),
            mode,
            provider: None,
            warning: None,
        };
    }
    let dictionary = if settings.dictionary_enabled {
        entries
    } else {
        &[]
    };
    let local = local_refine(source, mode, dictionary, settings.profile);
    if !settings.provider.enabled {
        return RefineOutcome {
            text: local,
            mode,
            provider: None,
            warning: None,
        };
    }

    match refine_with_provider(&settings.provider, dictionary, source, mode) {
        Ok(text) if !text.trim().is_empty() => RefineOutcome {
            text,
            mode,
            provider: Some(settings.provider.model.clone()),
            warning: None,
        },
        Ok(_) => RefineOutcome {
            text: local,
            mode,
            provider: None,
            warning: Some(
                "The cloud refinement provider returned an empty response, so the local transcript was kept."
                    .to_string(),
            ),
        },
        Err(err) => RefineOutcome {
            text: local,
            mode,
            provider: None,
            warning: Some(format!(
                "Cloud refinement was unavailable ({}). DictaFox kept the locally cleaned transcript.",
                err.message
            )),
        },
    }
}

/// Rule-based cleanup. It is deliberately conservative: fillers and spacing are
/// fixed, meaning is never added, and technical identifiers are never rewritten.
pub fn local_refine(
    text: &str,
    mode: RefinementMode,
    entries: &[DictionaryEntry],
    _profile: Profile,
) -> String {
    if mode == RefinementMode::Raw {
        return text.trim().to_string();
    }
    let mut out = text.trim().to_string();
    if !entries.is_empty() {
        out = dictionary::apply(&out, entries);
    }

    let preserve_lines = matches!(mode, RefinementMode::Developer | RefinementMode::Email);
    out = normalize_spacing(&out, preserve_lines);

    let aggressive = !matches!(mode, RefinementMode::Developer);
    out = strip_fillers(&out, aggressive);
    out = collapse_repeated_words(&out);

    if matches!(
        mode,
        RefinementMode::Professional | RefinementMode::Email
    ) {
        out = professionalize(&out);
    }

    if matches!(
        mode,
        RefinementMode::Clean
            | RefinementMode::Professional
            | RefinementMode::Casual
            | RefinementMode::Email
    ) {
        out = capitalize_sentences(&out);
        out = uppercase_standalone_i(&out);
        out = add_terminal_punctuation(&out);
    } else if mode == RefinementMode::Technical {
        out = capitalize_sentences(&out);
        out = add_terminal_punctuation(&out);
    }

    // Iterate: dictionary terms at sentence start should keep their casing.
    out.trim().to_string()
}

fn normalize_spacing(text: &str, preserve_lines: bool) -> String {
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    while lines.first().map(|l| l.is_empty()).unwrap_or(false) {
        lines.remove(0);
    }
    while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    if preserve_lines {
        lines.join("\n")
    } else {
        lines.join(" ")
    }
}

fn strip_fillers(text: &str, aggressive: bool) -> String {
    const FILLERS: &[&str] = &["um", "umm", "uh", "uhh", "erm", "hmm", "hm", "ah", "eh"];
    const EXTRA: &[&str] = &["you know", "i mean", "sort of", "kind of"];
    let mut joined = text.to_string();
    if aggressive {
        for phrase in EXTRA {
            joined = dictionary::replace_phrase(&joined, phrase, "");
        }
    }
    let cleaned = joined
        .lines()
        .map(|line| {
            let mut kept: Vec<&str> = Vec::new();
            for token in line.split_whitespace() {
                let comparison = token
                    .trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == ':')
                    .to_lowercase();
                if FILLERS.contains(&comparison.as_str()) {
                    continue;
                }
                kept.push(token);
            }
            kept.join(" ")
                .replace(" ,", ",")
                .replace(" .", ".")
                .replace(" ;", ";")
                .replace(" :", ":")
                .trim_start_matches([',', '.', ';', ':'])
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    cleaned
        .trim_start_matches([',', '.', ';', ':'])
        .trim()
        .to_string()
}

fn collapse_repeated_words(text: &str) -> String {
    text.lines()
        .map(|line| {
            let mut output: Vec<&str> = Vec::new();
            for token in line.split_whitespace() {
                let is_repeat = output.last().map_or(false, |previous| {
                    previous.eq_ignore_ascii_case(token)
                        && previous.chars().any(|c| c.is_alphanumeric())
                        && token.chars().any(|c| c.is_alphanumeric())
                });
                if !is_repeat {
                    output.push(token);
                }
            }
            output.join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn professionalize(text: &str) -> String {
    const REPLACEMENTS: &[(&str, &str)] = &[
        ("i think we should", "I recommend that we"),
        ("i think we need to", "We need to"),
        ("i guess", "I estimate"),
        ("can't", "cannot"),
        ("won't", "will not"),
        ("don't", "do not"),
        ("doesn't", "does not"),
        ("isn't", "is not"),
        ("aren't", "are not"),
        ("couldn't", "could not"),
        ("shouldn't", "should not"),
        ("wouldn't", "would not"),
        ("it's", "it is"),
        ("i'm", "I am"),
        ("we're", "we are"),
        ("gonna", "going to"),
        ("wanna", "want to"),
        ("kinda", "kind of"),
    ];
    let mut output = text.to_string();
    for (from, to) in REPLACEMENTS {
        output = dictionary::replace_phrase(&output, from, to);
    }
    output
}

fn capitalize_sentences(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let mut new_sentence = true;
    for index in 0..chars.len() {
        let ch = chars[index];
        if new_sentence && ch.is_alphabetic() {
            chars[index] = ch.to_uppercase().next().unwrap_or(ch);
            new_sentence = false;
        } else if matches!(ch, '.' | '!' | '?') {
            new_sentence = true;
        } else if ch == '\n' {
            new_sentence = true;
        } else if !ch.is_whitespace() && !matches!(ch, '"' | '\'' | '(' | '[') {
            new_sentence = false;
        }
    }
    chars.into_iter().collect()
}

fn uppercase_standalone_i(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            if token == "i" {
                "I".to_string()
            } else if let Some(rest) = token.strip_prefix("i'") {
                return format!("I'{rest}");
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn add_terminal_punctuation(text: &str) -> String {
    let trimmed = text.trim_end();
    let Some(last) = trimmed.chars().next_back() else {
        return trimmed.to_string();
    };
    if matches!(last, '.' | '!' | '?' | ':' | ';' | ')' | ']' | '}' | '"' | '\'') {
        return trimmed.to_string();
    }
    let last_token = trimmed.split_whitespace().next_back().unwrap_or("");
    // Never append a period to command-like or identifier-like endings.
    if last_token.contains('/')
        || last_token.contains('=')
        || last_token.contains('_')
        || last_token.contains("()")
        || last_token.starts_with("--")
        || last_token.starts_with('$')
    {
        return trimmed.to_string();
    }
    format!("{trimmed}.")
}

fn refine_with_provider(
    provider: &ProviderSettings,
    entries: &[DictionaryEntry],
    text: &str,
    mode: RefinementMode,
) -> AppResult<String> {
    let key = provider_api_key(provider).ok_or_else(|| {
        AppError::new(
            "provider_missing_key",
            "Cloud text refinement requires an API key.",
        )
        .hint("Add a key in Settings → Text refinement, or turn cloud refinement off.")
    })?;
    let endpoint = format!("{}/chat/completions", provider.base_url.trim_end_matches('/'));

    let glossary = entries
        .iter()
        .filter(|entry| entry.enabled && !entry.written.trim().is_empty())
        .take(200)
        .map(|entry| format!("{} -> {}", entry.spoken, entry.written))
        .collect::<Vec<_>>()
        .join("\n");
    let system = system_prompt(mode);
    let user = if glossary.is_empty() {
        text.to_string()
    } else {
        format!(
            "{text}\n\nUse these exact spellings when they occur naturally:\n{glossary}"
        )
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(provider.timeout_seconds))
        .build()
        .map_err(|err| {
            AppError::new("provider_error", "The cloud refinement client could not start.")
                .with_detail(err)
        })?;
    let response = client
        .post(&endpoint)
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": provider.model,
            "temperature": 0.2,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ]
        }))
        .send()
        .map_err(|err| {
            AppError::new(
                "provider_unavailable",
                "The cloud refinement provider could not be reached.",
            )
            .hint("Check the endpoint URL and your network connection. Dictation still works locally.")
            .with_detail(err)
        })?;
    if !response.status().is_success() {
        return Err(AppError::new(
            "provider_error",
            format!(
                "The cloud refinement provider returned HTTP {}.",
                response.status()
            ),
        )
        .hint("Check the API key, model name and endpoint in Settings → Text refinement."));
    }
    let body: serde_json::Value = response.json().map_err(|err| {
        AppError::new(
            "provider_error",
            "The cloud refinement provider returned an unexpected response.",
        )
        .with_detail(err)
    })?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    Ok(sanitize_provider_text(&content))
}

fn sanitize_provider_text(content: &str) -> String {
    let mut text = content.trim();
    if text.starts_with("```") {
        text = text.trim_start_matches("```");
        text = text.strip_prefix("text").unwrap_or(text);
        text = text.trim_start_matches('\n');
        text = text.trim_end_matches("```");
    }
    let trimmed = text.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        return trimmed[1..trimmed.len() - 1].trim().to_string();
    }
    trimmed.to_string()
}

fn system_prompt(mode: RefinementMode) -> &'static str {
    match mode {
        RefinementMode::Developer => {
            "You clean up dictated developer notes. Preserve every technical token exactly: identifiers, \
             camelCase, snake_case, file paths, flags, commands and code. Do not execute or explain commands. \
             Remove filler words and fix punctuation. Never add facts. Return only the cleaned text."
        }
        RefinementMode::Technical => {
            "You clean up dictated technical writing. Keep technical terms, product names and code identifiers \
             intact. Fix grammar and punctuation without changing meaning. Never invent details. Return only the text."
        }
        RefinementMode::Professional => {
            "You rewrite dictated text in a clear, professional tone for a workplace audience. Keep the speaker's \
             meaning, facts and level of detail. Do not add greetings, sign-offs or information. Return only the text."
        }
        RefinementMode::Email => {
            "You format dictated text as a concise professional email body. Fix punctuation and paragraph breaks. \
             Do not invent a recipient, greeting, subject or sign-off. Keep all facts. Return only the text."
        }
        RefinementMode::Casual => {
            "You lightly clean dictated conversational text. Keep the casual tone and contractions, remove filler \
             words, and fix punctuation. Never add information. Return only the text."
        }
        RefinementMode::Clean | RefinementMode::Profile | RefinementMode::Raw => {
            "You clean dictated text. Remove filler words, fix punctuation and capitalisation, and preserve the \
             speaker's words and meaning exactly. Do not summarise, add or remove facts. Return only the text."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::DictionaryEntry;

    fn entry(spoken: &str, written: &str) -> DictionaryEntry {
        DictionaryEntry {
            id: 1,
            spoken: spoken.into(),
            written: written.into(),
            category: None,
            case_sensitive: false,
            enabled: true,
        }
    }

    #[test]
    fn clean_removes_fillers_and_capitalises() {
        let out = local_refine(
            "um i think we should update the frappe server script and then test it",
            RefinementMode::Clean,
            &[],
            Profile::General,
        );
        assert_eq!(
            out,
            "I think we should update the frappe server script and then test it."
        );
    }

    #[test]
    fn developer_preserves_identifiers_and_adds_no_period() {
        let out = local_refine(
            "um run npm install and update user_service.py then check getUserById",
            RefinementMode::Developer,
            &[],
            Profile::Coding,
        );
        assert_eq!(
            out,
            "run npm install and update user_service.py then check getUserById"
        );
        assert!(!out.ends_with('.'));
    }

    #[test]
    fn developer_keeps_line_breaks() {
        let out = local_refine(
            "git status\num then git diff",
            RefinementMode::Developer,
            &[],
            Profile::Coding,
        );
        assert_eq!(out, "git status\nthen git diff");
    }

    #[test]
    fn professional_rewrites_conservatively() {
        let out = local_refine(
            "um i think we should update the frappe server script and then test it",
            RefinementMode::Professional,
            &[],
            Profile::Professional,
        );
        assert_eq!(
            out,
            "I recommend that we update the frappe server script and then test it."
        );
        let contractions = local_refine(
            "we don't know if it's ready",
            RefinementMode::Professional,
            &[],
            Profile::Professional,
        );
        assert_eq!(contractions, "We do not know if it is ready.");
    }

    #[test]
    fn dictionary_is_applied_before_styling() {
        let out = local_refine(
            "um call the api from postgres",
            RefinementMode::Clean,
            &[entry("api", "API"), entry("postgres", "PostgreSQL")],
            Profile::General,
        );
        assert_eq!(out, "Call the API from PostgreSQL.");
    }

    #[test]
    fn repeated_words_are_collapsed() {
        assert_eq!(
            local_refine(
                "the the server is is down",
                RefinementMode::Clean,
                &[],
                Profile::General
            ),
            "The server is down."
        );
    }

    #[test]
    fn question_marks_are_preserved() {
        assert_eq!(
            local_refine("is the deploy done?", RefinementMode::Clean, &[], Profile::General),
            "Is the deploy done?"
        );
    }

    #[test]
    fn raw_mode_changes_nothing() {
        let out = local_refine(
            " um hello   world ",
            RefinementMode::Raw,
            &[entry("hello", "HELLO")],
            Profile::General,
        );
        assert_eq!(out, "um hello   world");
    }

    #[test]
    fn technical_mode_does_not_add_period_to_a_command() {
        let out = local_refine(
            "run cargo test --all",
            RefinementMode::Technical,
            &[],
            Profile::Coding,
        );
        assert!(out.ends_with("--all"), "got {out}");
    }

    #[test]
    fn profile_mode_selects_the_profile_default() {
        let mut settings = Settings::default();
        settings.profile = Profile::Coding;
        settings.refinement = RefinementMode::Profile;
        assert_eq!(settings.effective_refinement(), RefinementMode::Developer);
        let outcome = refine(&settings, &[], "um run cargo build");
        assert_eq!(outcome.mode, RefinementMode::Developer);
        assert_eq!(outcome.text, "run cargo build");
        assert!(outcome.provider.is_none());
    }

    #[test]
    fn provider_text_is_sanitised() {
        assert_eq!(sanitize_provider_text("```text\nHello.\n```"), "Hello.");
        assert_eq!(sanitize_provider_text("\"Hello.\""), "Hello.");
        assert_eq!(sanitize_provider_text("  Hello.  "), "Hello.");
    }

    #[test]
    fn cloud_failure_falls_back_to_local_text() {
        let mut settings = Settings::default();
        settings.provider.enabled = true;
        settings.provider.api_key = Some("test-key".into());
        settings.provider.base_url = "http://127.0.0.1:9/v1".into();
        settings.provider.timeout_seconds = 1;
        let outcome = refine(&settings, &[], "um hello there");
        assert!(outcome.provider.is_none());
        assert!(outcome.warning.is_some());
        assert_eq!(outcome.text, "Hello there.");
    }
}
