//! Fold per-repo churn into the lifetime diff: a `Summary`, a per-language
//! breakdown, and a per-repo table. Includes fork/mirror dedup. All pure.

use crate::languages;
use crate::types::{LanguageStat, RepoChurn, RepoMeta, RepoStat, Summary};
use std::collections::BTreeMap;

/// Dedup repos that share a network root (forks/mirrors), keeping one per root so
/// the same upstream commits are not counted twice. A repo with no `source`/`fork_parent`
/// is keyed by its own `full_name`. Non-fork canonical repos win over forks.
pub fn dedup(repos: Vec<RepoMeta>) -> Vec<RepoMeta> {
    let mut chosen: BTreeMap<String, RepoMeta> = BTreeMap::new();
    for repo in repos {
        let key = repo
            .source
            .clone()
            .or_else(|| repo.fork_parent.clone())
            .unwrap_or_else(|| repo.full_name.clone());
        match chosen.get(&key) {
            Some(existing) if !existing.is_fork && repo.is_fork => {} // keep canonical
            _ => {
                chosen.insert(key, repo);
            }
        }
    }
    chosen.into_values().collect()
}

/// Build per-language stats (color + share), sorted by churn descending. Shared by
/// the final rollup and the live sync tick.
pub fn language_stats(per_language: &BTreeMap<String, (u64, u64)>) -> Vec<LanguageStat> {
    let total_churn: u64 = per_language.values().map(|(a, r)| a + r).sum();
    let denom = total_churn.max(1) as f64;
    let mut out: Vec<LanguageStat> = per_language
        .iter()
        .map(|(language, (added, removed))| LanguageStat {
            language: language.clone(),
            color: languages::color_of(language).to_string(),
            added: *added,
            removed: *removed,
            net: *added as i64 - *removed as i64,
            share: (added + removed) as f64 / denom,
        })
        .collect();
    out.sort_by(|a, b| (b.added + b.removed).cmp(&(a.added + a.removed)));
    out
}

/// The full rollup. Languages and repos are returned sorted by churn descending.
pub struct Rollup {
    pub summary: Summary,
    pub languages: Vec<LanguageStat>,
    pub repos: Vec<RepoStat>,
}

pub fn rollup(churns: &[RepoChurn]) -> Rollup {
    let mut lang_totals: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut total_added = 0u64;
    let mut total_removed = 0u64;
    let mut total_commits = 0u64;
    let mut repos: Vec<RepoStat> = Vec::new();

    for churn in churns {
        total_added += churn.added;
        total_removed += churn.removed;
        total_commits += churn.commits;

        let mut top: Option<(String, u64)> = None;
        for (lang, (a, r)) in &churn.per_language {
            let entry = lang_totals.entry(lang.clone()).or_insert((0, 0));
            entry.0 += a;
            entry.1 += r;
            let churn_for_lang = a + r;
            if top.as_ref().map(|(_, c)| churn_for_lang > *c).unwrap_or(true) {
                top = Some((lang.clone(), churn_for_lang));
            }
        }

        repos.push(RepoStat {
            full_name: churn.full_name.clone(),
            added: churn.added,
            removed: churn.removed,
            net: churn.net(),
            commits: churn.commits,
            top_language: top.map(|(l, _)| l),
        });
    }

    let languages = language_stats(&lang_totals);
    repos.sort_by(|a, b| (b.added + b.removed).cmp(&(a.added + a.removed)));

    let summary = Summary {
        added: total_added,
        removed: total_removed,
        net: total_added as i64 - total_removed as i64,
        commits: total_commits,
        repo_count: churns.len(),
        language_count: languages.len(),
    };

    Rollup { summary, languages, repos }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn churn(name: &str, langs: &[(&str, u64, u64)]) -> RepoChurn {
        let mut per_language = BTreeMap::new();
        let mut added = 0;
        let mut removed = 0;
        for (l, a, r) in langs {
            per_language.insert((*l).to_string(), (*a, *r));
            added += a;
            removed += r;
        }
        RepoChurn { full_name: name.into(), per_language, added, removed, commits: 1 }
    }

    #[test]
    fn rolls_up_totals_and_sorts() {
        let churns = vec![
            churn("o/a", &[("TypeScript", 100, 10), ("Rust", 5, 1)]),
            churn("o/b", &[("TypeScript", 50, 5), ("Python", 200, 20)]),
        ];
        let r = rollup(&churns);
        assert_eq!(r.summary.added, 355);
        assert_eq!(r.summary.removed, 36);
        assert_eq!(r.summary.net, 319);
        assert_eq!(r.summary.commits, 2);
        assert_eq!(r.summary.repo_count, 2);
        // Python has the most churn (220), then TypeScript (165), then Rust (6).
        assert_eq!(r.languages[0].language, "Python");
        assert_eq!(r.languages[1].language, "TypeScript");
        // shares sum to ~1.0
        let s: f64 = r.languages.iter().map(|l| l.share).sum();
        assert!((s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn top_language_per_repo() {
        let churns = vec![churn("o/a", &[("TypeScript", 100, 10), ("Rust", 5, 1)])];
        let r = rollup(&churns);
        assert_eq!(r.repos[0].top_language.as_deref(), Some("TypeScript"));
    }

    #[test]
    fn dedup_keeps_canonical_over_fork() {
        let canonical = RepoMeta {
            full_name: "up/x".into(), owner: "up".into(), clone_url: "".into(),
            default_branch: "main".into(), is_fork: false, fork_parent: None,
            source: None, is_private: false, is_archived: false, size_kb: 1, pushed_at: None,
        };
        let fork = RepoMeta {
            full_name: "me/x".into(), owner: "me".into(), clone_url: "".into(),
            default_branch: "main".into(), is_fork: true, fork_parent: Some("up/x".into()),
            source: Some("up/x".into()), is_private: false, is_archived: false, size_kb: 1,
            pushed_at: None,
        };
        let out = dedup(vec![fork, canonical]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].full_name, "up/x");
    }
}
