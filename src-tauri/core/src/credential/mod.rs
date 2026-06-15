//! Credential layer: one token store (macOS Keychain via `keyring`) fed by three
//! sources - OAuth device flow, the `gh` CLI, or a pasted PAT. The token never
//! touches SQLite or logs; only its source/login metadata is persisted.

pub mod device_flow;

use crate::error::{AppError, Result};
use crate::github::{GithubClient, User};
use crate::sensitive::Sensitive;

const KEYRING_SERVICE: &str = "com.masterdiff.app";
const KEYRING_ACCOUNT: &str = "github-token";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    Device,
    GhCli,
    Pat,
}

impl CredentialSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CredentialSource::Device => "device",
            CredentialSource::GhCli => "gh",
            CredentialSource::Pat => "pat",
        }
    }
}

/// Keychain-backed token storage.
pub struct TokenStore;

impl TokenStore {
    fn entry() -> Result<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map_err(|e| AppError::Storage(format!("keychain: {e}")))
    }

    pub fn save(token: &Sensitive<String>) -> Result<()> {
        Self::entry()?
            .set_password(token.expose())
            .map_err(|e| AppError::Storage(format!("keychain save: {e}")))
    }

    pub fn load() -> Result<Option<Sensitive<String>>> {
        match Self::entry()?.get_password() {
            Ok(t) => Ok(Some(Sensitive(t))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AppError::Storage(format!("keychain load: {e}"))),
        }
    }

    pub fn clear() -> Result<()> {
        match Self::entry()?.delete_password() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Storage(format!("keychain clear: {e}"))),
        }
    }
}

/// Validate a token against the GitHub API, returning the authenticated user.
pub fn validate(token: &Sensitive<String>) -> Result<User> {
    GithubClient::new(token.expose().clone()).get_user()
}

/// Resolve the `gh` binary. A Finder-launched app gets a minimal PATH that omits
/// Homebrew, so check the common install locations before falling back to PATH.
fn gh_bin() -> String {
    for candidate in ["/opt/homebrew/bin/gh", "/usr/local/bin/gh"] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    "gh".to_string()
}

/// Read a token from the `gh` CLI (`gh auth token`).
pub fn from_gh_cli() -> Result<Sensitive<String>> {
    let out = std::process::Command::new(gh_bin())
        .args(["auth", "token"])
        .output()
        .map_err(|_| AppError::Auth("gh CLI not found on PATH".into()))?;
    if !out.status.success() {
        return Err(AppError::Auth(
            "gh CLI is not logged in (run `gh auth login`)".into(),
        ));
    }
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if token.is_empty() {
        return Err(AppError::Auth("gh CLI returned an empty token".into()));
    }
    Ok(Sensitive(token))
}

/// True if the `gh` CLI is available and logged in (for showing the fast path).
pub fn gh_cli_available() -> bool {
    std::process::Command::new(gh_bin())
        .args(["auth", "token"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Connect using a given token + source: validate, store in Keychain, return user.
pub fn connect(token: Sensitive<String>, source: CredentialSource) -> Result<(User, CredentialSource)> {
    let user = validate(&token)?;
    TokenStore::save(&token)?;
    Ok((user, source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_strings_are_stable() {
        assert_eq!(CredentialSource::Device.as_str(), "device");
        assert_eq!(CredentialSource::GhCli.as_str(), "gh");
        assert_eq!(CredentialSource::Pat.as_str(), "pat");
    }
}
