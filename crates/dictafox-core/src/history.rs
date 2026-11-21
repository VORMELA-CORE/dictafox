use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::settings::{Profile, RefinementMode};
use crate::storage::Store;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub text: String,
    pub raw_text: Option<String>,
    pub profile: Profile,
    pub refinement: RefinementMode,
    pub model: String,
    pub language: String,
    pub duration_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct NewHistoryEntry {
    pub text: String,
    pub raw_text: Option<String>,
    pub profile: Profile,
    pub refinement: RefinementMode,
    pub model: String,
    pub language: String,
    pub duration_ms: i64,
}

pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn insert(store: &Store, entry: &NewHistoryEntry) -> AppResult<i64> {
    let created_at = now_millis();
    let profile = serde_json::to_value(entry.profile)?;
    let refinement = serde_json::to_value(entry.refinement)?;
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO history (text, raw_text, profile, refinement, model, language, duration_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.text,
                entry.raw_text,
                profile.as_str().unwrap_or("general"),
                refinement.as_str().unwrap_or("clean"),
                entry.model,
                entry.language,
                entry.duration_ms,
                created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    })
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    let profile_str: String = row.get("profile")?;
    let refinement_str: String = row.get("refinement")?;
    Ok(HistoryEntry {
        id: row.get("id")?,
        text: row.get("text")?,
        raw_text: row.get("raw_text")?,
        profile: serde_json::from_value(serde_json::Value::String(profile_str))
            .unwrap_or(Profile::General),
        refinement: serde_json::from_value(serde_json::Value::String(refinement_str))
            .unwrap_or(RefinementMode::Clean),
        model: row.get("model")?,
        language: row.get("language")?,
        duration_ms: row.get("duration_ms")?,
        created_at: row.get("created_at")?,
    })
}

/// Newest first. `search` is a case-insensitive substring match.
pub fn list(store: &Store, search: Option<&str>, limit: u32) -> AppResult<Vec<HistoryEntry>> {
    let limit = limit.clamp(1, 500);
    let needle = search.map(str::trim).filter(|s| !s.is_empty());
    store.with_conn(|conn| {
        let mut entries = Vec::new();
        match needle {
            Some(needle) => {
                let pattern = format!("%{}%", needle.replace('%', "\\%").replace('_', "\\_"));
                let mut stmt = conn.prepare(
                    "SELECT * FROM history
                     WHERE text LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                     ORDER BY created_at DESC, id DESC LIMIT ?2",
                )?;
                let rows = stmt.query_map(rusqlite::params![pattern, limit], row_to_entry)?;
                for row in rows {
                    entries.push(row?);
                }
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT * FROM history ORDER BY created_at DESC, id DESC LIMIT ?1",
                )?;
                let rows = stmt.query_map([limit], row_to_entry)?;
                for row in rows {
                    entries.push(row?);
                }
            }
        }
        Ok(entries)
    })
}

pub fn get(store: &Store, id: i64) -> AppResult<Option<HistoryEntry>> {
    store.with_conn(|conn| {
        conn.query_row("SELECT * FROM history WHERE id = ?1", [id], row_to_entry)
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
    })
}

pub fn latest(store: &Store) -> AppResult<Option<HistoryEntry>> {
    Ok(list(store, None, 1)?.into_iter().next())
}

pub fn delete(store: &Store, id: i64) -> AppResult<bool> {
    let changed = store.with_conn(|conn| {
        conn.execute("DELETE FROM history WHERE id = ?1", [id])
    })?;
    Ok(changed > 0)
}

pub fn clear(store: &Store) -> AppResult<usize> {
    store.with_conn(|conn| conn.execute("DELETE FROM history", []))
}

pub fn count(store: &Store) -> AppResult<i64> {
    store.with_conn(|conn| conn.query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(text: &str) -> NewHistoryEntry {
        NewHistoryEntry {
            text: text.to_string(),
            raw_text: Some(format!(" raw {text} ")),
            profile: Profile::Coding,
            refinement: RefinementMode::Developer,
            model: "base.en".into(),
            language: "en".into(),
            duration_ms: 1234,
        }
    }

    #[test]
    fn insert_list_search_and_delete() {
        let store = Store::in_memory().unwrap();
        let first = insert(&store, &entry("Update the DocType API")).unwrap();
        insert(&store, &entry("Ship the release")).unwrap();
        assert_eq!(count(&store).unwrap(), 2);

        let all = list(&store, None, 50).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].text, "Ship the release");
        assert_eq!(all[0].duration_ms, 1234);
        assert_eq!(all[0].profile, Profile::Coding);
        assert_eq!(all[0].refinement, RefinementMode::Developer);

        let hits = list(&store, Some("doctype"), 50).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, first);

        assert!(delete(&store, first).unwrap());
        assert!(!delete(&store, first).unwrap());
        assert_eq!(count(&store).unwrap(), 1);
        assert_eq!(clear(&store).unwrap(), 1);
        assert_eq!(count(&store).unwrap(), 0);
    }

    #[test]
    fn search_treats_like_wildcards_literally() {
        let store = Store::in_memory().unwrap();
        insert(&store, &entry("100% complete")).unwrap();
        insert(&store, &entry("nothing here")).unwrap();
        assert_eq!(list(&store, Some("%"), 10).unwrap().len(), 1);
        assert_eq!(list(&store, Some("_"), 10).unwrap().len(), 0);
    }

    #[test]
    fn latest_returns_newest() {
        let store = Store::in_memory().unwrap();
        assert!(latest(&store).unwrap().is_none());
        insert(&store, &entry("one")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        insert(&store, &entry("two")).unwrap();
        assert_eq!(latest(&store).unwrap().unwrap().text, "two");
    }
}
