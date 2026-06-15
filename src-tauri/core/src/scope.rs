//! Pure scope rules: does a repository qualify under the active `Scope`.

use crate::types::{RepoMeta, Scope};

pub fn in_scope(repo: &RepoMeta, scope: &Scope) -> bool {
    if repo.is_fork && !scope.include_forks {
        return false;
    }
    if scope.owner_only && repo.owner != scope.login {
        return false;
    }
    if repo.is_archived && !scope.include_archived {
        return false;
    }
    true
}

/// Filter a discovered repo list to the in-scope set.
pub fn filter(repos: Vec<RepoMeta>, scope: &Scope) -> Vec<RepoMeta> {
    repos.into_iter().filter(|r| in_scope(r, scope)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(full: &str, owner: &str, fork: bool, archived: bool) -> RepoMeta {
        RepoMeta {
            full_name: full.into(),
            owner: owner.into(),
            clone_url: format!("https://github.com/{full}.git"),
            default_branch: "main".into(),
            is_fork: fork,
            fork_parent: None,
            source: None,
            is_private: false,
            is_archived: archived,
            size_kb: 1,
            pushed_at: None,
        }
    }

    #[test]
    fn excludes_forks_by_default() {
        let s = Scope::default_for("jamubc");
        assert!(in_scope(&repo("jamubc/a", "jamubc", false, false), &s));
        assert!(!in_scope(&repo("jamubc/fork", "jamubc", true, false), &s));
    }

    #[test]
    fn owner_only_drops_org_repos() {
        let mut s = Scope::default_for("jamubc");
        s.owner_only = true;
        assert!(in_scope(&repo("jamubc/a", "jamubc", false, false), &s));
        assert!(!in_scope(&repo("Jam-Sw/x", "Jam-Sw", false, false), &s));
    }

    #[test]
    fn include_forks_toggle() {
        let mut s = Scope::default_for("jamubc");
        s.include_forks = true;
        assert!(in_scope(&repo("jamubc/fork", "jamubc", true, false), &s));
    }
}
