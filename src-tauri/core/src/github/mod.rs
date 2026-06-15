//! GitHub API client. M0 uses the REST surface (user identity + repo discovery);
//! M1 adds GraphQL fast-mode. Blocking HTTP (ureq) so core needs no async runtime.

pub mod model;
mod rest;

pub use model::User;
pub use rest::GithubClient;
