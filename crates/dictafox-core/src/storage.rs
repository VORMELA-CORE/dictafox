use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::{AppError, AppResult};

/// Thread-safe SQLite handle. DictaFox stores settings, history and the
/// personal dictionary locally; nothing in here ever leaves the machine.
pub struct Store {
    conn: Mutex<Connection>,
    path: Option<PathBuf>,
}

impl Store {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let store = Self {
            conn: Mutex::new(conn),
            path: Some(path.to_path_buf()),
        };
        store.configure()?;
        store.migrate()?;
        store.restrict_permissions();
        Ok(store)
    }

    pub fn in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Mutex::new(conn),
            path: None,
        };
        store.configure()?;
        store.migrate()?;
        Ok(store)
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn configure(&self) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.busy_timeout(std::time::Duration::from_secs(5))?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            if self.path.is_some() {
                conn.execute_batch("PRAGMA journal_mode = WAL;")?;
            }
            Ok(())
        })
    }

    fn migrate(&self) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS history (
                    id            INTEGER PRIMARY KEY AUTOINCREMENT,
                    text          TEXT NOT NULL,
                    raw_text      TEXT,
                    profile       TEXT NOT NULL,
                    refinement    TEXT NOT NULL,
                    model         TEXT NOT NULL,
                    language      TEXT NOT NULL,
                    duration_ms   INTEGER NOT NULL DEFAULT 0,
                    created_at    INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS history_created_at
                    ON history (created_at DESC);

                CREATE TABLE IF NOT EXISTS dictionary (
                    id             INTEGER PRIMARY KEY AUTOINCREMENT,
                    spoken         TEXT NOT NULL,
                    written        TEXT NOT NULL,
                    category       TEXT,
                    case_sensitive INTEGER NOT NULL DEFAULT 0,
                    enabled        INTEGER NOT NULL DEFAULT 1,
                    created_at     INTEGER NOT NULL
                );
                CREATE UNIQUE INDEX IF NOT EXISTS dictionary_spoken
                    ON dictionary (spoken COLLATE NOCASE);
                "#,
            )
        })
    }

    fn restrict_permissions(&self) {
        #[cfg(unix)]
        if let Some(path) = &self.path {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
    }

    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> AppResult<T> {
        let guard = self.conn.lock().map_err(|_| {
            AppError::new(
                "storage_unavailable",
                "DictaFox's local database is unavailable.",
            )
            .hint("Restart DictaFox. If the problem persists, check the log file in the data directory.")
        })?;
        f(&guard).map_err(AppError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_create_schema() {
        let store = Store::in_memory().unwrap();
        let count: i64 = store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('settings','history','dictionary')",
                    [],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn opens_on_disk_and_keeps_data() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("test.sqlite3");
        {
            let store = Store::open(&db).unwrap();
            store
                .with_conn(|conn| {
                    conn.execute(
                        "INSERT INTO settings (key, value) VALUES ('probe', 'ok')",
                        [],
                    )
                })
                .unwrap();
        }
        let store = Store::open(&db).unwrap();
        let value: String = store
            .with_conn(|conn| {
                conn.query_row("SELECT value FROM settings WHERE key='probe'", [], |r| {
                    r.get(0)
                })
            })
            .unwrap();
        assert_eq!(value, "ok");
    }
}
