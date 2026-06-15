//! Domain types. serde camelCase so the same structs cross the IPC boundary
//! to the Svelte frontend in M1 without a translation layer.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One repository as discovered from the GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoMeta {
    pub full_name: String,
    pub owner: String,
    pub clone_url: String,
    pub default_branch: String,
    pub is_fork: bool,
    /// `owner/name` of the fork's upstream, when known (for dedup).
    pub fork_parent: Option<String>,
    /// `owner/name` of the network root, when known (for dedup).
    pub source: Option<String>,
    pub is_private: bool,
    pub is_archived: bool,
    pub size_kb: u64,
    /// Last push time from the API; drives incremental sync (skip unchanged repos).
    #[serde(default)]
    pub pushed_at: Option<String>,
}

/// What counts toward the master diff. Defaults to "everything I authored".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    /// Include forks (default false - avoids double-counting upstream history).
    pub include_forks: bool,
    /// Restrict to repos the user personally owns (drops org/collaborator repos).
    pub owner_only: bool,
    /// Include archived repos (default true).
    pub include_archived: bool,
    /// The user's login, used for `owner_only`.
    pub login: String,
}

impl Scope {
    pub fn default_for(login: &str) -> Self {
        Scope {
            include_forks: false,
            owner_only: false,
            include_archived: true,
            login: login.to_string(),
        }
    }
}

/// Per-repository churn, bucketed by language. Serializable so it can be cached
/// per-repo in SQLite for incremental re-syncs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoChurn {
    pub full_name: String,
    /// language -> (added, removed)
    pub per_language: BTreeMap<String, (u64, u64)>,
    pub added: u64,
    pub removed: u64,
}

impl RepoChurn {
    pub fn net(&self) -> i64 {
        self.added as i64 - self.removed as i64
    }

    /// The language with the most churn in this repo.
    pub fn top_language(&self) -> Option<String> {
        self.per_language
            .iter()
            .max_by_key(|(_, (a, r))| a + r)
            .map(|(lang, _)| lang.clone())
    }
}

/// A cached per-repo churn keyed by the repo's `pushed_at`. If the API reports the
/// same `pushed_at`, the cached churn is reused instead of re-cloning.
#[derive(Debug, Clone)]
pub struct CachedRepo {
    pub pushed_at: Option<String>,
    pub churn: RepoChurn,
}

/// Per-language rollup across all repos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageStat {
    pub language: String,
    pub color: String,
    pub added: u64,
    pub removed: u64,
    pub net: i64,
    /// Fraction of total churn (added+removed), 0.0..=1.0.
    pub share: f64,
}

/// Per-repo rollup for the dashboard table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStat {
    pub full_name: String,
    pub added: u64,
    pub removed: u64,
    pub net: i64,
    pub top_language: Option<String>,
}

/// The headline numbers - the lifetime "master diff".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub added: u64,
    pub removed: u64,
    pub net: i64,
    pub repo_count: usize,
    pub language_count: usize,
}

/// The full cached result the dashboard renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub summary: Summary,
    pub languages: Vec<LanguageStat>,
    pub repos: Vec<RepoStat>,
    /// True when generated/vendored files were excluded (the meaningful number).
    pub filtered: bool,
    pub last_synced_at: Option<String>,
}

/// Connection state surfaced to the UI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub connected: bool,
    pub source: Option<String>,
    pub login: Option<String>,
}

/// Background sync state for the progress UI and tray.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub syncing: bool,
    pub phase: String,
    pub repos_done: u32,
    pub repos_total: u32,
    pub current_repo: Option<String>,
    pub last_finished_at: Option<String>,
    pub last_error: Option<String>,
}

/// User-configurable settings, persisted to SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub include_forks: bool,
    pub owner_only: bool,
    pub include_archived: bool,
    pub exclude_generated: bool,
    /// Extra author emails beyond the derived noreply address.
    pub extra_emails: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            include_forks: false,
            owner_only: false,
            include_archived: true,
            exclude_generated: true,
            extra_emails: Vec::new(),
        }
    }
}
