//! The deep sync engine: discover repos, clone/fetch, run author-filtered numstat,
//! bucket by language, and aggregate into the lifetime diff.

pub mod git;

use crate::error::Result;
use crate::github::GithubClient;
use crate::numstat::{self, ChurnOptions};
use crate::sensitive::Sensitive;
use crate::types::{CachedRepo, RepoChurn, RepoMeta, Scope};
use crate::{aggregate, scope};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Progress for one processed repo, for the UI/CLI.
#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub done: usize,
    pub total: usize,
    pub repo: String,
    pub added: u64,
    pub removed: u64,
}

/// One repo's result from a sync, with enough to update the cache and stream a tick.
#[derive(Debug, Clone)]
pub struct RepoResult {
    pub churn: RepoChurn,
    pub pushed_at: Option<String>,
    /// True when reused from cache (unchanged since last sync) rather than re-cloned.
    pub from_cache: bool,
}

/// Build a git `--author` BRE from the user's emails (joined with `\|`).
pub fn authors_regex(emails: &[String]) -> String {
    emails.join("\\|")
}

/// Discover in-scope repos for the authenticated user.
pub fn discover(client: &GithubClient, scope_cfg: &Scope) -> Result<Vec<RepoMeta>> {
    let repos = client.list_repos()?;
    let deduped = aggregate::dedup(repos);
    Ok(scope::filter(deduped, scope_cfg))
}

/// Run the deep sync over the given repos, calling `progress` after each.
pub fn deep_sync(
    cache_dir: &Path,
    token: &Sensitive<String>,
    repos: &[RepoMeta],
    emails: &[String],
    opts: &ChurnOptions,
    mut progress: impl FnMut(SyncProgress),
) -> Result<Vec<RepoChurn>> {
    let authors = authors_regex(emails);
    let total = repos.len();
    let mut churns = Vec::with_capacity(total);
    for (i, repo) in repos.iter().enumerate() {
        let dir = git::clone_or_fetch(cache_dir, repo, token)?;
        let raw = git::numstat(&dir, &authors)?;
        let churn = numstat::churn_for_repo(&repo.full_name, &raw, opts);
        progress(SyncProgress {
            done: i + 1,
            total,
            repo: repo.full_name.clone(),
            added: churn.added,
            removed: churn.removed,
        });
        churns.push(churn);
    }
    Ok(churns)
}

/// Process one repo: reuse the cache if `pushed_at` is unchanged, else clone/fetch
/// and run numstat.
fn process_repo(
    cache_dir: &Path,
    token: &Sensitive<String>,
    repo: &RepoMeta,
    authors: &str,
    opts: &ChurnOptions,
    cached: &HashMap<String, CachedRepo>,
) -> Result<RepoResult> {
    if let Some(c) = cached.get(&repo.full_name) {
        if c.pushed_at.is_some() && c.pushed_at == repo.pushed_at {
            return Ok(RepoResult {
                churn: c.churn.clone(),
                pushed_at: repo.pushed_at.clone(),
                from_cache: true,
            });
        }
    }
    let dir = git::clone_or_fetch(cache_dir, repo, token)?;
    let raw = git::numstat(&dir, authors)?;
    let mut churn = numstat::churn_for_repo(&repo.full_name, &raw, opts);
    churn.commits = git::commit_count(&dir, authors);
    Ok(RepoResult { churn, pushed_at: repo.pushed_at.clone(), from_cache: false })
}

/// Sync all repos in parallel (bounded worker pool), reusing the cache for repos
/// unchanged since last sync. `on_repo` is called once per completed repo (in
/// completion order) so the UI can stream a live tally. Repos that error are
/// skipped, not fatal. Returns the successful results.
pub fn sync(
    cache_dir: &Path,
    token: &Sensitive<String>,
    repos: &[RepoMeta],
    emails: &[String],
    opts: &ChurnOptions,
    cached: &HashMap<String, CachedRepo>,
    threads: usize,
    on_repo: impl Fn(&RepoResult, usize, usize) + Sync + Send,
) -> Result<Vec<RepoResult>> {
    let authors = authors_regex(emails);
    let total = repos.len();
    let done = AtomicUsize::new(0);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .build()
        .map_err(|e| crate::AppError::Git(format!("thread pool: {e}")))?;

    let results: Vec<RepoResult> = pool.install(|| {
        repos
            .par_iter()
            .filter_map(|repo| {
                let result = process_repo(cache_dir, token, repo, &authors, opts, cached).ok();
                let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                if let Some(ref rr) = result {
                    on_repo(rr, n, total);
                }
                result
            })
            .collect()
    });
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authors_regex_joins_with_bre_alt() {
        let r = authors_regex(&["a@x.com".into(), "b@y.com".into()]);
        assert_eq!(r, "a@x.com\\|b@y.com");
    }
}
