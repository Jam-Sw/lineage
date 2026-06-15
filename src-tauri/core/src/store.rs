//! SQLite cache so the dashboard and tray show the last result instantly on
//! launch, before a re-sync runs. Append-only migrations, mirroring the
//! InstantNotes store pattern. The GitHub token is NOT stored here - it lives in
//! the OS Keychain (see `credential`).

use crate::error::{AppError, Result};
use crate::types::{
    AppSettings, AppearanceSettings, AuthStatus, CachedRepo, RepoChurn, Snapshot, SyncStatus,
};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE IF NOT EXISTS settings (
        key        TEXT PRIMARY KEY,
        value      TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS kv_cache (
        key        TEXT PRIMARY KEY,
        value      TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS sync_state (
        id                INTEGER PRIMARY KEY CHECK (id = 1),
        syncing           INTEGER NOT NULL DEFAULT 0,
        phase             TEXT NOT NULL DEFAULT 'idle',
        repos_done        INTEGER NOT NULL DEFAULT 0,
        repos_total       INTEGER NOT NULL DEFAULT 0,
        current_repo      TEXT,
        last_finished_at  TEXT,
        last_error        TEXT
    );
    INSERT OR IGNORE INTO sync_state (id) VALUES (1);
    CREATE TABLE IF NOT EXISTS credential_meta (
        id           INTEGER PRIMARY KEY CHECK (id = 1),
        source       TEXT,
        login        TEXT,
        connected_at TEXT
    );
    INSERT OR IGNORE INTO credential_meta (id) VALUES (1);",
    // v2: per-repo churn cache for incremental sync
    "CREATE TABLE IF NOT EXISTS repo_cache (
        full_name  TEXT PRIMARY KEY,
        pushed_at  TEXT,
        churn_json TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );",
    // v3: clear the cache so churn is recomputed with commit counts.
    "DELETE FROM repo_cache;",
];

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(|e| AppError::Storage(e.to_string()))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| AppError::Storage(e.to_string()))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| AppError::Storage(e.to_string()))?;
        let store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| AppError::Storage(e.to_string()))?;
        let store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| AppError::Storage(e.to_string()))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            self.conn
                .execute_batch(sql)
                .map_err(|e| AppError::Storage(format!("migration {}: {e}", i + 1)))?;
            self.conn
                .pragma_update(None, "user_version", (i + 1) as i64)
                .map_err(|e| AppError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    fn now() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    // ---- settings (serde JSON under a single key) ----

    pub fn get_settings(&self) -> Result<AppSettings> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |r| r.get(0))
            .ok();
        match raw {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| AppError::Storage(format!("settings parse: {e}"))),
            None => Ok(AppSettings::default()),
        }
    }

    pub fn set_settings(&self, settings: &AppSettings) -> Result<()> {
        let value = serde_json::to_string(settings)
            .map_err(|e| AppError::Storage(e.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES ('app', ?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at = ?2",
                (&value, Self::now()),
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    // ---- appearance (separate key; never clears the churn cache) ----

    pub fn get_appearance(&self) -> Result<AppearanceSettings> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = 'appearance'", [], |r| r.get(0))
            .ok();
        match raw {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| AppError::Storage(format!("appearance parse: {e}"))),
            None => Ok(AppearanceSettings::default()),
        }
    }

    pub fn set_appearance(&self, a: &AppearanceSettings) -> Result<()> {
        let value = serde_json::to_string(a).map_err(|e| AppError::Storage(e.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES ('appearance', ?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at = ?2",
                (&value, Self::now()),
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    // ---- cached snapshot ----

    pub fn get_snapshot(&self) -> Result<Option<Snapshot>> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM kv_cache WHERE key = 'snapshot'", [], |r| r.get(0))
            .ok();
        match raw {
            Some(s) => Ok(Some(
                serde_json::from_str(&s)
                    .map_err(|e| AppError::Storage(format!("snapshot parse: {e}")))?,
            )),
            None => Ok(None),
        }
    }

    pub fn set_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let value = serde_json::to_string(snapshot)
            .map_err(|e| AppError::Storage(e.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO kv_cache (key, value, updated_at) VALUES ('snapshot', ?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at = ?2",
                (&value, Self::now()),
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    // ---- sync state ----

    pub fn sync_status(&self) -> Result<SyncStatus> {
        self.conn
            .query_row(
                "SELECT syncing, phase, repos_done, repos_total, current_repo, last_finished_at, last_error
                 FROM sync_state WHERE id = 1",
                [],
                |r| {
                    Ok(SyncStatus {
                        syncing: r.get::<_, i64>(0)? != 0,
                        phase: r.get(1)?,
                        repos_done: r.get::<_, i64>(2)? as u32,
                        repos_total: r.get::<_, i64>(3)? as u32,
                        current_repo: r.get(4)?,
                        last_finished_at: r.get(5)?,
                        last_error: r.get(6)?,
                    })
                },
            )
            .map_err(|e| AppError::Storage(e.to_string()))
    }

    pub fn sync_begin(&self, total: u32) -> Result<()> {
        self.conn
            .execute(
                "UPDATE sync_state SET syncing = 1, phase = 'syncing', repos_done = 0,
                 repos_total = ?1, current_repo = NULL, last_error = NULL WHERE id = 1",
                [total as i64],
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn sync_progress(&self, done: u32, current: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE sync_state SET repos_done = ?1, current_repo = ?2 WHERE id = 1",
                (done as i64, current),
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn sync_finish(&self) -> Result<()> {
        self.conn
            .execute(
                "UPDATE sync_state SET syncing = 0, phase = 'idle', current_repo = NULL,
                 last_finished_at = ?1 WHERE id = 1",
                [Self::now()],
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn sync_error(&self, msg: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE sync_state SET syncing = 0, phase = 'idle', last_error = ?1 WHERE id = 1",
                [msg],
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    // ---- credential metadata (token itself is in the Keychain) ----

    pub fn auth_status(&self) -> Result<AuthStatus> {
        self.conn
            .query_row(
                "SELECT source, login FROM credential_meta WHERE id = 1",
                [],
                |r| {
                    let source: Option<String> = r.get(0)?;
                    let login: Option<String> = r.get(1)?;
                    Ok(AuthStatus {
                        connected: source.is_some(),
                        source,
                        login,
                    })
                },
            )
            .map_err(|e| AppError::Storage(e.to_string()))
    }

    pub fn set_credential_meta(&self, source: &str, login: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE credential_meta SET source = ?1, login = ?2, connected_at = ?3 WHERE id = 1",
                (source, login, Self::now()),
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn clear_credential_meta(&self) -> Result<()> {
        self.conn
            .execute(
                "UPDATE credential_meta SET source = NULL, login = NULL, connected_at = NULL WHERE id = 1",
                [],
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    // ---- per-repo churn cache (incremental sync) ----

    pub fn load_repo_cache(&self) -> Result<HashMap<String, CachedRepo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT full_name, pushed_at, churn_json FROM repo_cache")
            .map_err(|e| AppError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| AppError::Storage(e.to_string()))?;
        let mut out = HashMap::new();
        for row in rows {
            let (full_name, pushed_at, churn_json) =
                row.map_err(|e| AppError::Storage(e.to_string()))?;
            if let Ok(churn) = serde_json::from_str::<RepoChurn>(&churn_json) {
                out.insert(full_name, CachedRepo { pushed_at, churn });
            }
        }
        Ok(out)
    }

    pub fn save_repo_cache(
        &self,
        full_name: &str,
        pushed_at: Option<&str>,
        churn: &RepoChurn,
    ) -> Result<()> {
        let json = serde_json::to_string(churn).map_err(|e| AppError::Storage(e.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO repo_cache (full_name, pushed_at, churn_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(full_name) DO UPDATE SET pushed_at = ?2, churn_json = ?3, updated_at = ?4",
                (full_name, pushed_at, &json, Self::now()),
            )
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn clear_repo_cache(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM repo_cache", [])
            .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Drop cache rows for repos no longer in scope.
    pub fn prune_repo_cache(&self, keep: &HashSet<String>) -> Result<()> {
        let existing: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT full_name FROM repo_cache")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| AppError::Storage(e.to_string()))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        for name in existing {
            if !keep.contains(&name) {
                let _ = self
                    .conn
                    .execute("DELETE FROM repo_cache WHERE full_name = ?1", [&name]);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_defaults() {
        let s = Store::open_in_memory().unwrap();
        assert!(!s.auth_status().unwrap().connected);
        assert!(s.get_snapshot().unwrap().is_none());
        let st = s.sync_status().unwrap();
        assert!(!st.syncing);
        assert_eq!(st.phase, "idle");
        assert!(s.get_settings().unwrap().exclude_generated);
    }

    #[test]
    fn settings_round_trip() {
        let s = Store::open_in_memory().unwrap();
        let mut cfg = AppSettings::default();
        cfg.include_forks = true;
        cfg.extra_emails = vec!["a@b.com".into()];
        s.set_settings(&cfg).unwrap();
        let got = s.get_settings().unwrap();
        assert!(got.include_forks);
        assert_eq!(got.extra_emails, vec!["a@b.com".to_string()]);
    }

    #[test]
    fn sync_lifecycle_and_cred() {
        let s = Store::open_in_memory().unwrap();
        s.sync_begin(10).unwrap();
        assert!(s.sync_status().unwrap().syncing);
        s.sync_progress(3, "o/r").unwrap();
        assert_eq!(s.sync_status().unwrap().repos_done, 3);
        s.sync_finish().unwrap();
        assert!(!s.sync_status().unwrap().syncing);
        assert!(s.sync_status().unwrap().last_finished_at.is_some());

        s.set_credential_meta("gh", "jamubc").unwrap();
        let a = s.auth_status().unwrap();
        assert!(a.connected);
        assert_eq!(a.login.as_deref(), Some("jamubc"));
        s.clear_credential_meta().unwrap();
        assert!(!s.auth_status().unwrap().connected);
    }
}
