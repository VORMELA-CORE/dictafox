use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::history::now_millis;
use crate::storage::Store;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub id: i64,
    pub spoken: String,
    pub written: String,
    pub category: Option<String>,
    pub case_sensitive: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryInput {
    pub id: Option<i64>,
    pub spoken: String,
    pub written: String,
    pub category: Option<String>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<DictionaryEntry> {
    Ok(DictionaryEntry {
        id: row.get("id")?,
        spoken: row.get("spoken")?,
        written: row.get("written")?,
        category: row.get("category")?,
        case_sensitive: row.get::<_, i64>("case_sensitive")? != 0,
        enabled: row.get::<_, i64>("enabled")? != 0,
    })
}

pub fn list(store: &Store) -> AppResult<Vec<DictionaryEntry>> {
    store.with_conn(|conn| {
        let mut stmt =
            conn.prepare("SELECT * FROM dictionary ORDER BY category, spoken COLLATE NOCASE")?;
        let rows = stmt.query_map([], row_to_entry)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    })
}

pub fn upsert(store: &Store, input: &DictionaryInput) -> AppResult<DictionaryEntry> {
    let spoken = input.spoken.trim();
    let written = input.written.trim();
    if spoken.is_empty() {
        return Err(AppError::new(
            "dictionary_invalid",
            "A dictionary entry needs a spoken form.",
        )
        .hint("Enter the word or phrase as you say it, for example \"frappe\"."));
    }
    if written.is_empty() {
        return Err(AppError::new(
            "dictionary_invalid",
            "A dictionary entry needs a written form.",
        )
        .hint("Enter how the word should appear in your text, for example \"Frappe\"."));
    }
    if spoken.len() > 200 || written.len() > 200 {
        return Err(AppError::new(
            "dictionary_invalid",
            "Dictionary entries are limited to 200 characters.",
        ));
    }
    let category = input
        .category
        .as_ref()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty());
    let case_sensitive = input.case_sensitive as i64;
    let enabled = input.enabled as i64;

    let id = match input.id {
        Some(id) => {
            store.with_conn(|conn| {
                conn.execute(
                    "UPDATE dictionary SET spoken=?1, written=?2, category=?3,
                        case_sensitive=?4, enabled=?5 WHERE id=?6",
                    rusqlite::params![spoken, written, category, case_sensitive, enabled, id],
                )?;
                Ok(id)
            })?;
            id
        }
        None => store.with_conn(|conn| {
            conn.execute(
                "INSERT INTO dictionary (spoken, written, category, case_sensitive, enabled, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(spoken) DO UPDATE SET written=excluded.written,
                    category=excluded.category, case_sensitive=excluded.case_sensitive,
                    enabled=excluded.enabled",
                rusqlite::params![spoken, written, category, case_sensitive, enabled, now_millis()],
            )?;
            Ok(conn.last_insert_rowid())
        })?,
    };

    let entry = store.with_conn(|conn| {
        conn.query_row("SELECT * FROM dictionary WHERE id = ?1", [id], row_to_entry)
    })?;
    Ok(entry)
}

pub fn delete(store: &Store, id: i64) -> AppResult<bool> {
    let changed =
        store.with_conn(|conn| conn.execute("DELETE FROM dictionary WHERE id = ?1", [id]))?;
    Ok(changed > 0)
}

pub fn clear(store: &Store) -> AppResult<usize> {
    store.with_conn(|conn| conn.execute("DELETE FROM dictionary", []))
}

/// Apply enabled entries, longest spoken form first. Matching is whole-word
/// (letters, digits and `_` count as word characters) so "type" never rewrites
/// "prototype".
pub fn apply(text: &str, entries: &[DictionaryEntry]) -> String {
    if text.is_empty() || entries.is_empty() {
        return text.to_string();
    }
    let mut ordered: Vec<&DictionaryEntry> = entries
        .iter()
        .filter(|e| e.enabled && !e.spoken.trim().is_empty() && !e.written.is_empty())
        .collect();
    ordered.sort_by_key(|e| std::cmp::Reverse(e.spoken.chars().count()));

    let mut result = text.to_string();
    for entry in ordered {
        result = replace_word(&result, &entry.spoken, &entry.written, entry.case_sensitive);
    }
    result
}

/// Whole-word, case-insensitive replacement used by the refinement rules.
pub(crate) fn replace_phrase(haystack: &str, from: &str, to: &str) -> String {
    replace_word(haystack, from, to, false)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn replace_word(haystack: &str, needle: &str, replacement: &str, case_sensitive: bool) -> String {
    let needle = needle.trim();
    if needle.is_empty() {
        return haystack.to_string();
    }
    let lower_haystack = haystack.to_lowercase();
    let lower_needle = needle.to_lowercase();
    // Byte offsets only line up when lowercasing preserves length. For the rare
    // scripts where it does not (e.g. Turkish dotted I) fall back to exact match.
    if !case_sensitive && lower_haystack.len() != haystack.len() {
        return haystack.replace(needle, replacement);
    }
    let (search_in, search_for) = if case_sensitive {
        (haystack.to_string(), needle.to_string())
    } else {
        (lower_haystack, lower_needle)
    };

    let mut output = String::with_capacity(haystack.len());
    let mut cursor = 0usize;
    while let Some(found) = search_in[cursor..].find(&search_for) {
        let start = cursor + found;
        let end = start + search_for.len();
        let before_ok = haystack[..start]
            .chars()
            .next_back()
            .map(|c| !is_word_char(c))
            .unwrap_or(true);
        let after_ok = haystack[end..]
            .chars()
            .next()
            .map(|c| !is_word_char(c))
            .unwrap_or(true);
        if before_ok && after_ok {
            output.push_str(&haystack[cursor..start]);
            output.push_str(replacement);
            cursor = end;
        } else {
            output.push_str(&haystack[cursor..end]);
            cursor = end;
        }
    }
    output.push_str(&haystack[cursor..]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn crud_roundtrip_and_replacement() {
        let store = Store::in_memory().unwrap();
        let created = upsert(
            &store,
            &DictionaryInput {
                id: None,
                spoken: "frappe".into(),
                written: "Frappe".into(),
                category: Some("Frameworks".into()),
                case_sensitive: false,
                enabled: true,
            },
        )
        .unwrap();
        assert!(created.id > 0);
        assert_eq!(created.category.as_deref(), Some("Frameworks"));

        let all = list(&store).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].written, "Frappe");

        let updated = upsert(
            &store,
            &DictionaryInput {
                id: Some(created.id),
                spoken: "frappe".into(),
                written: "FRAPPE".into(),
                category: None,
                case_sensitive: true,
                enabled: true,
            },
        )
        .unwrap();
        assert_eq!(updated.written, "FRAPPE");
        assert!(updated.case_sensitive);

        assert!(delete(&store, created.id).unwrap());
        assert!(list(&store).unwrap().is_empty());
    }

    #[test]
    fn duplicate_spoken_form_updates_instead_of_failing() {
        let store = Store::in_memory().unwrap();
        let input = DictionaryInput {
            id: None,
            spoken: "api".into(),
            written: "API".into(),
            category: None,
            case_sensitive: false,
            enabled: true,
        };
        upsert(&store, &input).unwrap();
        upsert(&store, &input).unwrap();
        assert_eq!(list(&store).unwrap().len(), 1);
    }

    #[test]
    fn replacement_respects_word_boundaries_and_case() {
        let entries = vec![
            entry("api", "API"),
            entry("postgres", "PostgreSQL"),
            entry("type", "DocType"),
        ];
        assert_eq!(
            apply("call the api from postgres, not the prototype", &entries),
            "call the API from PostgreSQL, not the prototype"
        );
        assert_eq!(
            apply("update the DocType TypeScript file", &entries),
            "update the DocType TypeScript file"
        );
    }

    #[test]
    fn case_sensitive_entry_only_matches_exact_case() {
        let mut entries = vec![entry("react", "React")];
        assert_eq!(apply("react and React", &entries), "React and React");
        entries[0].case_sensitive = true;
        assert_eq!(apply("react and React", &entries), "React and React");
        entries[0].case_sensitive = false;
        entries[0].written = "REACT".into();
        assert_eq!(apply("react", &entries), "REACT");
    }

    #[test]
    fn longest_phrase_wins() {
        let entries = vec![entry("server", "server"), entry("frappe server script", "server script")];
        assert_eq!(
            apply("the frappe server script is slow", &entries),
            "the server script is slow"
        );
    }

    #[test]
    fn invalid_entries_are_rejected() {
        let store = Store::in_memory().unwrap();
        let input = DictionaryInput {
            id: None,
            spoken: " ".into(),
            written: "x".into(),
            category: None,
            case_sensitive: false,
            enabled: true,
        };
        assert_eq!(upsert(&store, &input).unwrap_err().code, "dictionary_invalid");
    }
}
