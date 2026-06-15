//! Master Diff core - pure domain logic with no Tauri dependency.
//! GitHub repo discovery (REST), the churn engine (clone + `git log --numstat`),
//! language mapping, scope rules, and aggregation into the lifetime "master diff".

pub mod aggregate;
pub mod credential;
pub mod engine;
pub mod error;
pub mod github;
pub mod ignore;
pub mod languages;
pub mod numstat;
pub mod scope;
pub mod sensitive;
pub mod store;
pub mod types;

pub use error::{AppError, Result};
pub use store::Store;
