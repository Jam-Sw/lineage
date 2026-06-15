# Project Context

## Purpose
Master Diff is a macOS menu-bar app that shows your lifetime GitHub "master diff" -
the net of every line you have added and removed across your whole account - and a
dashboard with a per-language and per-repository breakdown. The number should be
meaningful (your real authored code), not raw churn inflated by generated files.

## Tech Stack
- Tauri 2 desktop shell (menu-bar / tray first)
- Rust core (`masterdiff-core`) for API access, the git churn engine, language mapping, and aggregation
- SQLite via rusqlite for the cached result and settings
- `ureq` (blocking) for the GitHub REST/GraphQL API; the `git` CLI for cloning
- `keyring` for token storage in the macOS Keychain
- Svelte 5 and TypeScript for the webview UI
- cargo test, vitest, and svelte-check for verification

## Project Conventions

### Code Style
Rust owns API access, git, language rules, and aggregation. TypeScript owns view
state and typed IPC calls. UI code calls the API client instead of `invoke` directly.

### Architecture Patterns
A local Rust core behind a thin Tauri shell. The deep engine clones each in-scope
repo and parses `git log --numstat` for the user's commits, buckets by language, and
aggregates into the master diff. Heavy work runs on a background thread and reports
progress through change events; the tray title is updated via `TrayIcon::set_title`.

### Testing Strategy
Pure core modules (numstat parsing, language mapping, generated-file filtering, scope
rules, aggregation, dedup) are covered with Rust unit tests. The engine is validated
end-to-end against the real account via the `master_diff_m0` example. Frontend types
pass `svelte-check`; web assets build through Vite.

## Domain Context
A repository's churn is the lines the user authored, matched across all their emails
(`--no-merges`, all branches). "Master diff" is the net (added minus removed) summed
across in-scope repos. Default scope is everything the user authored across owned,
org, and collaborator repos, with forks excluded. Generated and vendored files are
excluded by default because they dominate and distort raw churn.

## Important Constraints
- Accurate per-language lifetime churn requires cloning and `git log --numstat`; the
  GitHub API cannot provide it.
- Use a full bare clone (not blobless) so numstat is fast.
- The token must never appear in logs, SQLite, or error output; it lives in the Keychain.
- Apple Silicon, ad-hoc signed. No LICENSE file (proprietary).
- Generated build output and the clone cache are not committed.

## External Dependencies
The GitHub API (REST now, GraphQL fast-mode later) and the user's GitHub token.
