//! Parse the output of `git log --no-merges --author=<re> --pretty=tformat: --numstat`
//! into per-language churn. Pure and unit-tested - the heart of the deep engine.
//!
//! Each changed-file row is `added\tremoved\tpath`. Binary files emit `-` for the
//! counts and are skipped. Rename rows carry an `old => new` path which
//! `languages::classify` normalizes.

use crate::types::{PathNode, RepoChurn};
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
    let mut tree = Builder::default();
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
        let entry = per_language.entry(lang.clone()).or_insert((0, 0));
        entry.0 += row.added;
        entry.1 += row.removed;
        tree.add(&final_path(&row.path), &lang, row.added, row.removed);
        added += row.added;
        removed += row.removed;
    }
    RepoChurn {
        full_name: full_name.to_string(),
        per_language,
        added,
        removed,
        commits: 0,
        tree: Some(tree.finish(full_name.rsplit('/').next().unwrap_or(full_name), added + removed)),
    }
}

/// The path a numstat row ends up at. Renames come as `old => new` or
/// `pre/{old => new}/post`; keep the prefix and suffix around the new part.
fn final_path(path: &str) -> String {
    let Some(arrow) = path.find(" => ") else { return path.to_string() };
    match (path[..arrow].rfind('{'), path[arrow..].find('}')) {
        (Some(open), Some(close)) => {
            let close = arrow + close;
            let joined = format!("{}{}{}", &path[..open], &path[arrow + 4..close], &path[close + 1..]);
            joined.replace("//", "/")
        }
        _ => path[arrow + 4..].to_string(),
    }
}

/// Deepest level kept, counting the repo root as 0. Files below it fold into their folder.
const MAX_DEPTH: usize = 5;
/// Children kept per node; the rest fold into one "N more" node.
const MAX_CHILDREN: usize = 8;
/// Nodes under this fraction of the whole repo's churn fold away too.
const MIN_SHARE: f64 = 0.004;

#[derive(Default)]
struct Builder {
    added: u64,
    removed: u64,
    langs: BTreeMap<String, u64>,
    children: BTreeMap<String, Builder>,
}

impl Builder {
    fn add(&mut self, path: &str, lang: &str, added: u64, removed: u64) {
        let mut node = self;
        let mut depth = 0;
        loop {
            node.added += added;
            node.removed += removed;
            *node.langs.entry(lang.to_string()).or_default() += added + removed;
            if depth == MAX_DEPTH {
                break;
            }
            let Some(part) = path.split('/').filter(|p| !p.is_empty()).nth(depth) else { break };
            node = node.children.entry(part.to_string()).or_default();
            depth += 1;
        }
    }

    fn finish(self, name: &str, repo_total: u64) -> PathNode {
        let language = self
            .langs
            .into_iter()
            .max_by_key(|(_, n)| *n)
            .map(|(l, _)| l)
            .unwrap_or_else(|| "Other".into());
        let floor = (repo_total as f64 * MIN_SHARE) as u64;
        let mut kids: Vec<(String, Builder)> = self.children.into_iter().collect();
        kids.sort_by(|a, b| (b.1.added + b.1.removed).cmp(&(a.1.added + a.1.removed)));
        let mut children = Vec::new();
        let (mut rest, mut rest_a, mut rest_r) = (0usize, 0u64, 0u64);
        for (i, (kname, k)) in kids.into_iter().enumerate() {
            if i < MAX_CHILDREN && k.added + k.removed > floor {
                children.push(k.finish(&kname, repo_total));
            } else {
                rest += 1;
                rest_a += k.added;
                rest_r += k.removed;
            }
        }
        if rest > 0 && rest_a + rest_r > floor {
            children.push(PathNode {
                name: format!("{rest} more"),
                added: rest_a,
                removed: rest_r,
                language: language.clone(),
                children: Vec::new(),
            });
        }
        PathNode { name: name.to_string(), added: self.added, removed: self.removed, language, children }
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
    fn builds_a_pruned_folder_tree() {
        let raw = "10\t0\tsrc/a.ts\n5\t5\tsrc/lib/b.rs\n3\t0\tsrc/{old => new}/c.ts\n1\t0\tREADME.md\n";
        let churn = churn_for_repo("me/app", raw, &ChurnOptions::raw());
        let tree = churn.tree.unwrap();
        assert_eq!(tree.name, "app");
        assert_eq!(tree.added + tree.removed, 24);
        let src = tree.children.iter().find(|c| c.name == "src").unwrap();
        assert_eq!(src.added + src.removed, 23);
        assert_eq!(src.language, "TypeScript");
        assert!(src.children.iter().any(|c| c.name == "new"));
        assert!(src.children.iter().all(|c| c.name != "{old"));
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
