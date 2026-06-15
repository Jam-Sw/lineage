# Lineage Specification

## Requirements

### Requirement: Lifetime Diff Computation
The app SHALL compute the net of every line the user has added and removed across their in-scope GitHub repositories by cloning each repository and parsing `git log --numstat` for the commits the user authored.

#### Scenario: Compute the lifetime diff
- **WHEN** a connected user runs a sync
- **THEN** the app clones or fetches each in-scope repository
- **AND** parses authored commits with `git log --numstat`
- **AND** reports a net total (added minus removed) bucketed by language

#### Scenario: Match the user across identities
- **WHEN** matching authored commits
- **THEN** the app matches all of the user's known emails
- **AND** excludes merge commits and commits authored by anyone else

### Requirement: Repository Scope
The app SHALL let the user control which repositories count. By default it SHALL include owned, organization, and collaborator repositories and SHALL exclude forks.

#### Scenario: Exclude forks by default
- **WHEN** the default scope is active
- **THEN** forked repositories are not counted
- **AND** the user MAY enable forks in settings

#### Scenario: Owner-only scope
- **WHEN** the user enables owner-only
- **THEN** repositories owned by organizations or other accounts are dropped from the total

### Requirement: Generated-File Filtering
The app SHALL exclude generated and vendored files (dependencies, lockfiles, minified bundles) by default so the total reflects authored code, and it SHALL offer a toggle to include them.

#### Scenario: Filter generated files
- **WHEN** the default filter is active
- **THEN** files matching the generated and vendored rules are excluded from the total
- **AND** enabling "include generated" recomputes the total with them included

### Requirement: Incremental Sync
The app SHALL cache per-repository results keyed by the API `pushed_at` value so unchanged repositories are reused, and it SHALL clear the cache when scope or filter settings change.

#### Scenario: Reuse unchanged repositories
- **WHEN** a repository's `pushed_at` is unchanged since the last sync
- **THEN** the app reuses the cached churn instead of re-cloning

#### Scenario: Invalidate on settings change
- **WHEN** the user changes scope or filtering
- **THEN** the cache is cleared
- **AND** the next sync recomputes from the repositories

### Requirement: Menu-Bar Presence
The app SHALL run as a macOS menu-bar app and SHALL show the current net total as the tray title, updating it as a sync progresses.

#### Scenario: Show the headline in the tray
- **WHEN** a sync completes
- **THEN** the tray title shows the net total in the user's chosen format

#### Scenario: Indicate sync in progress
- **WHEN** a sync is running
- **THEN** the tray shows an animated indicator until it finishes

### Requirement: Dashboard
The app SHALL provide a dashboard with the headline total, a per-language breakdown using the real GitHub language colors, and a sortable per-repository table.

#### Scenario: Browse the breakdown
- **WHEN** the user opens the dashboard
- **THEN** the app shows the net total, the per-language bars, and the per-repository table
- **AND** the table can be sorted

### Requirement: Account Connection
The app SHALL connect to GitHub via the `gh` CLI or a personal access token carrying the `repo` scope. A token MUST NOT appear in logs, SQLite, or error output; it SHALL be stored in the macOS Keychain.

#### Scenario: Connect with a token
- **WHEN** the user provides a personal access token
- **THEN** the app stores it in the macOS Keychain
- **AND** never writes it to logs or the database

### Requirement: Self-Update
The app SHALL check its public release endpoint for updates and SHALL let the user install an available update and relaunch. When no update exists or the endpoint is unreachable, the app MUST fail silently and MUST NOT interrupt the user.

#### Scenario: Offer an available update
- **WHEN** a newer signed release exists at the update endpoint
- **THEN** the app surfaces the available version
- **AND** the user can download, install, and relaunch into it

#### Scenario: Stay silent when unreachable
- **WHEN** the update endpoint is unreachable or no newer release exists
- **THEN** the app does not surface anything and does not block use
