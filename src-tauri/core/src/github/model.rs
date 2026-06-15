//! Deserialization structs for the GitHub REST responses we consume.

use crate::types::RepoMeta;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
    pub node_id: String,
}

impl User {
    /// The derivable `noreply` commit email GitHub assigns to a user.
    pub fn noreply_email(&self) -> String {
        format!("{}+{}@users.noreply.github.com", self.id, self.login)
    }
}

#[derive(Debug, Deserialize)]
struct OwnerDto {
    login: String,
}

/// A repo row from `GET /user/repos`. Note the list endpoint does not include
/// `parent`/`source`, so fork lineage is left None here (forks are excluded by
/// default scope anyway).
#[derive(Debug, Deserialize)]
pub struct RepoDto {
    full_name: String,
    owner: OwnerDto,
    clone_url: String,
    #[serde(default)]
    default_branch: Option<String>,
    #[serde(default)]
    fork: bool,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    pushed_at: Option<String>,
}

impl RepoDto {
    pub fn into_meta(self) -> RepoMeta {
        RepoMeta {
            full_name: self.full_name,
            owner: self.owner.login,
            clone_url: self.clone_url,
            default_branch: self.default_branch.unwrap_or_else(|| "HEAD".to_string()),
            is_fork: self.fork,
            fork_parent: None,
            source: None,
            is_private: self.private,
            is_archived: self.archived,
            size_kb: self.size,
            pushed_at: self.pushed_at,
        }
    }
}
