//! REST client: authenticated user + paginated repo discovery.

use super::model::{RepoDto, User};
use crate::error::{AppError, Result};
use crate::sensitive::Sensitive;
use crate::types::RepoMeta;
use std::collections::BTreeMap;

const API: &str = "https://api.github.com";
const UA: &str = "lineage (https://github.com/Jam-Sw/lineage)";

pub struct GithubClient {
    pub(super) token: Sensitive<String>,
}

impl GithubClient {
    pub fn new(token: String) -> Self {
        GithubClient { token: Sensitive(token) }
    }

    fn get(&self, url: &str) -> Result<ureq::Response> {
        let resp = ureq::get(url)
            .set("Authorization", &format!("Bearer {}", self.token.expose()))
            .set("User-Agent", UA)
            .set("Accept", "application/vnd.github+json")
            .set("X-GitHub-Api-Version", "2022-11-28")
            .call();
        match resp {
            Ok(r) => Ok(r),
            Err(ureq::Error::Status(401, _)) => {
                Err(AppError::Auth("GitHub rejected the token (401)".into()))
            }
            Err(ureq::Error::Status(403, r)) => {
                let remaining = r.header("x-ratelimit-remaining").unwrap_or("");
                if remaining == "0" {
                    Err(AppError::RateLimited("GitHub rate limit reached".into()))
                } else {
                    Err(AppError::Auth("GitHub forbidden (403) - check token scopes".into()))
                }
            }
            Err(ureq::Error::Status(code, r)) => Err(AppError::Network(format!(
                "GitHub {code}: {}",
                r.status_text()
            ))),
            Err(ureq::Error::Transport(t)) => Err(AppError::Network(t.to_string())),
        }
    }

    pub fn get_user(&self) -> Result<User> {
        let resp = self.get(&format!("{API}/user"))?;
        resp.into_json::<User>()
            .map_err(|e| AppError::Network(format!("parse /user: {e}")))
    }

    /// All repos across owner + collaborator + org-member affiliations, deduped by
    /// `full_name`. Forks and scope filtering are applied later by `scope`.
    pub fn list_repos(&self) -> Result<Vec<RepoMeta>> {
        let mut next = Some(format!(
            "{API}/user/repos?affiliation=owner,collaborator,organization_member\
             &visibility=all&per_page=100"
        ));
        let mut by_name: BTreeMap<String, RepoMeta> = BTreeMap::new();
        while let Some(url) = next {
            let resp = self.get(&url)?;
            next = link_next(resp.header("link"));
            let page: Vec<RepoDto> = resp
                .into_json()
                .map_err(|e| AppError::Network(format!("parse /user/repos: {e}")))?;
            for dto in page {
                let meta = dto.into_meta();
                by_name.insert(meta.full_name.clone(), meta);
            }
        }
        Ok(by_name.into_values().collect())
    }
}

/// Extract the `rel="next"` URL from a GitHub Link header, if present.
fn link_next(header: Option<&str>) -> Option<String> {
    let header = header?;
    for part in header.split(',') {
        let part = part.trim();
        if part.contains("rel=\"next\"") {
            let start = part.find('<')? + 1;
            let end = part.find('>')?;
            return Some(part[start..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_link_next() {
        let h = "<https://api.github.com/user/repos?page=2>; rel=\"next\", \
                 <https://api.github.com/user/repos?page=5>; rel=\"last\"";
        assert_eq!(
            link_next(Some(h)).as_deref(),
            Some("https://api.github.com/user/repos?page=2")
        );
        assert_eq!(link_next(Some("<x>; rel=\"last\"")), None);
        assert_eq!(link_next(None), None);
    }
}
