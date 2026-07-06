//! Thin `git` process wrappers for the deep engine. Full bare clone (no filter)
//! then `git log --numstat` - M0 measured this at ~2s/repo, vs ~25s with blobless
//! lazy-fetch. Re-sync is an incremental `git fetch`.

use crate::error::{AppError, Result};
use crate::proc;
use crate::sensitive::Sensitive;
use crate::types::RepoMeta;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Turn a spawn failure into a Git error, with a friendly message when the git
/// binary itself is missing (the common case on a fresh Windows machine).
fn spawn_error(e: std::io::Error) -> AppError {
    if e.kind() == std::io::ErrorKind::NotFound {
        AppError::Git("git not found. Install Git and make sure it is on PATH".into())
    } else {
        AppError::Git(format!("git failed to start: {e}"))
    }
}

/// Cache path for a repo's bare clone, e.g. `<cache>/owner__name.git`.
pub fn clone_dir(cache_dir: &Path, full_name: &str) -> PathBuf {
    cache_dir.join(format!("{}.git", full_name.replace('/', "__")))
}

/// Clone the repo (full, bare) if absent, else fetch updates. Returns the clone dir.
///
/// NOTE (M1): the token is embedded in the clone URL for M0 simplicity, which puts
/// it in the bare repo's `remote.origin.url` and in `ps`. M1 should switch to
/// GIT_ASKPASS / a credential helper so the token only lives in the Keychain.
pub fn clone_or_fetch(cache_dir: &Path, repo: &RepoMeta, token: &Sensitive<String>) -> Result<PathBuf> {
    let dir = clone_dir(cache_dir, &repo.full_name);
    let token_str = token.expose();
    if dir.exists() {
        run(
            proc::command("git").args([
                "-C",
                &dir.to_string_lossy(),
                "fetch",
                "--quiet",
                "--prune",
                "origin",
                "+refs/heads/*:refs/heads/*",
            ]),
            token_str,
        )?;
    } else {
        std::fs::create_dir_all(cache_dir)
            .map_err(|e| AppError::Storage(format!("cannot create cache dir: {e}")))?;
        let url = format!(
            "https://x-access-token:{}@github.com/{}.git",
            token_str, repo.full_name
        );
        run(
            proc::command("git").args([
                "clone",
                "--quiet",
                "--bare",
                "--no-tags",
                &url,
                &dir.to_string_lossy(),
            ]),
            token_str,
        )?;
    }
    Ok(dir)
}

/// Count author-filtered commits across all branches (no merges).
pub fn commit_count(repo_dir: &Path, authors_regex: &str) -> u64 {
    let out = proc::command("git")
        .args([
            "-C",
            &repo_dir.to_string_lossy(),
            "rev-list",
            "--count",
            "--all",
            "--no-merges",
            &format!("--author={authors_regex}"),
        ])
        .output();
    out.ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .unwrap_or(0)
}

/// Author-filtered churn across all branches. `authors_regex` is a git BRE
/// (emails joined with `\|`).
pub fn numstat(repo_dir: &Path, authors_regex: &str) -> Result<String> {
    let out = proc::command("git")
        .args([
            "-C",
            &repo_dir.to_string_lossy(),
            "log",
            "--all",
            "--no-merges",
            &format!("--author={authors_regex}"),
            "--pretty=tformat:",
            "--numstat",
        ])
        .output()
        .map_err(spawn_error)?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(AppError::Git(format!("git log: {}", err.trim())));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Run a command, mapping failure to `AppError::Git` with the token redacted.
fn run(cmd: &mut Command, token: &str) -> Result<()> {
    let out = cmd.output().map_err(spawn_error)?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(AppError::Git(redact(err.trim(), token)));
    }
    Ok(())
}

fn redact(s: &str, token: &str) -> String {
    if token.is_empty() {
        s.to_string()
    } else {
        s.replace(token, "***")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_dir_is_flat_and_safe() {
        let d = clone_dir(Path::new("/cache"), "Jam-Sw/lineage");
        assert_eq!(d, PathBuf::from("/cache/Jam-Sw__lineage.git"));
    }

    #[test]
    fn redact_hides_token() {
        assert_eq!(redact("fatal: https://x-access-token:ghp_abc@github.com", "ghp_abc"), "fatal: https://x-access-token:***@github.com");
    }
}
