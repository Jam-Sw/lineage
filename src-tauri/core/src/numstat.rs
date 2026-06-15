//! Parse the output of `git log --no-merges --author=<re> --pretty=tformat: --numstat`
//! into per-language churn. Pure and unit-tested - the heart of the deep engine.
//!
//! Each changed-file row is `added\tremoved\tpath`. Binary files emit `-` for the
//! counts and are skipped. Rename rows carry an `old => new` path which
//! `languages::classify` normalizes.

use crate::types::RepoChurn;
use crate::{ignore, languages};
use std::collections::BTreeMap;

/// One parsed numstat row (a changed file in one commit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub added: u64,
    pub removed: u64,
    pub path: String,
}

/// How to count churn. The default excludes generated/vendored paths and caps
/// single-file mega-changes (both proved essential by M0).
#[derive(Debug, Clone)]
pub struct ChurnOptions {
    /// Skip generated/vendored paths (node_modules, lockfiles, minified, ...).
    pub exclude_generated: bool,
    /// Skip any single-file change larger than this many lines (vendored/data dumps).
    pub max_file_lines: Option<u64>,
}

impl ChurnOptions {
    /// Count everything, exactly as `git log --numstat` reports it.
    pub fn raw() -> Self {
        ChurnOptions { exclude_generated: false, max_file_lines: None }
    }

    /// The meaningful default: real authored code only.
    pub fn filtered() -> Self {
        ChurnOptions { exclude_generated: true, max_file_lines: Some(50_000) }
    }
}

impl Default for ChurnOptions {
    fn default() -> Self {
        ChurnOptions::filtered()
    }
}

/// Parse raw numstat text into rows, skipping blank lines and binary (`-`) rows.
pub fn parse_rows(raw: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for line in raw.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let (a, r, p) = match (parts.next(), parts.next(), parts.next()) {
            (Some(a), Some(r), Some(p)) => (a, r, p),
            _ => continue,
        };
        // Binary files show "-" for both columns.
        let (added, removed) = match (a.parse::<u64>(), r.parse::<u64>()) {
            (Ok(added), Ok(removed)) => (added, removed),
            _ => continue,
        };
        rows.push(Row {
            added,
            removed,
            path: p.to_string(),
        });
    }
    rows
}

/// Fold numstat text for one repo into language-bucketed churn under `opts`.
pub fn churn_for_repo(full_name: &str, raw: &str, opts: &ChurnOptions) -> RepoChurn {
    let mut per_language: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut added = 0u64;
    let mut removed = 0u64;
    for row in parse_rows(raw) {
        if opts.exclude_generated && ignore::is_generated(&row.path) {
            continue;
        }
        if let Some(cap) = opts.max_file_lines {
            if row.added > cap || row.removed > cap {
                continue;
            }
        }
        let lang = languages::classify(&row.path).name.to_string();
        let entry = per_language.entry(lang).or_insert((0, 0));
        entry.0 += row.added;
        entry.1 += row.removed;
        added += row.added;
        removed += row.removed;
    }
    RepoChurn {
        full_name: full_name.to_string(),
        per_language,
        added,
        removed,
        commits: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_skips_binary() {
        let raw = "10\t2\tsrc/a.ts\n-\t-\timg/logo.png\n5\t0\tlib/b.rs\n";
        let rows = parse_rows(raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], Row { added: 10, removed: 2, path: "src/a.ts".into() });
        assert_eq!(rows[1], Row { added: 5, removed: 0, path: "lib/b.rs".into() });
    }

    #[test]
    fn buckets_by_language() {
        let raw = "10\t2\tsrc/a.ts\n3\t1\tsrc/b.tsx\n5\t0\tlib/c.rs\n-\t-\tx.png\n";
        let churn = churn_for_repo("o/r", raw, &ChurnOptions::raw());
        assert_eq!(churn.added, 18);
        assert_eq!(churn.removed, 3);
        assert_eq!(churn.net(), 15);
        assert_eq!(churn.per_language.get("TypeScript"), Some(&(13, 3)));
        assert_eq!(churn.per_language.get("Rust"), Some(&(5, 0)));
    }

    #[test]
    fn handles_rename_rows() {
        let raw = "4\t1\tsrc/{old.js => new.ts}\n";
        let churn = churn_for_repo("o/r", raw, &ChurnOptions::raw());
        assert_eq!(churn.per_language.get("TypeScript"), Some(&(4, 1)));
    }

    #[test]
    fn empty_input_is_zero() {
        let churn = churn_for_repo("o/r", "\n\n", &ChurnOptions::raw());
        assert_eq!(churn.added, 0);
        assert_eq!(churn.removed, 0);
        assert!(churn.per_language.is_empty());
    }

    #[test]
    fn filtered_excludes_generated_and_megafiles() {
        let raw = concat!(
            "10\t2\tsrc/a.ts\n",                                  // kept
            "900\t100\tnode_modules/dep/index.js\n",             // vendored -> dropped
            "5\t0\tpackage-lock.json\n",                          // lockfile -> dropped
            "261480\t0\tdeps/sqlite3.c\n",                        // mega-file -> dropped
        );
        let churn = churn_for_repo("o/r", raw, &ChurnOptions::filtered());
        assert_eq!(churn.added, 10);
        assert_eq!(churn.removed, 2);
        assert_eq!(churn.per_language.keys().collect::<Vec<_>>(), vec!["TypeScript"]);
    }
}
