# Lineage

Your lifetime of GitHub contributions on one page, live in the menu bar or system tray.

<img width="944" height="638" alt="image" src="https://github.com/user-attachments/assets/a63b597a-4f26-4888-b334-58f2295befd8" />


Lineage computes every line you have ever added and removed across your entire
GitHub account, by language, and shows the net number live in your menu bar
(macOS) or system tray (Windows and Linux). Click through for a dashboard: the
headline diff, a per-language breakdown with the real GitHub language colors, and
a sortable per-repository table.

## Features

- **Lifetime diff in the menu bar**: the net of every line you have added and removed, by language, updated as it syncs; on Windows and Linux the number is drawn into the tray icon itself
- **Accurate by construction**: clones your repositories and parses `git log --numstat` for the commits you authored, the only way to split lifetime churn by language
- **Real code, not noise**: generated and vendored files (node_modules, lockfiles, minified bundles) are excluded by default, with a toggle to include them
- **Scope you control**: owned, organization, and collaborator repositories, with forks excluded by default and owner-only and archived toggles
- **Incremental**: per-repository results are cached by the API `pushed_at`, so re-syncs only touch what changed
- **Local and private**: your token lives in the OS credential store (macOS Keychain, Windows Credential Manager, or the Linux Secret Service) and the analysis runs entirely on your machine; nothing you own is uploaded to any server of ours
- **Self-updating**: the app checks GitHub Releases and installs new versions from inside the app

## Installation

Download the latest build for your platform from the [releases page](../../releases).
The builds are unsigned, so each OS asks for a one-time confirmation on first
launch; the in-app updater applies later versions without any of it.

### macOS (Apple Silicon)

Download the `.dmg`, open it, and drag Lineage to Applications. The app is not
notarized, so macOS blocks the first launch with an "Apple could not verify"
message. Clear the quarantine flag and it opens normally from then on:

```sh
xattr -d com.apple.quarantine /Applications/Lineage.app
```

Alternatively, after the blocked first launch, open System Settings, go to Privacy
and Security, scroll down, and click "Open Anyway". On macOS 14 and earlier,
right-click the app and choose Open instead.

### Windows (x64)

Download and run the `-setup.exe` installer. SmartScreen flags the unsigned build:
click "More info", then "Run anyway". Lineage shells out to `git` during sync, so
[Git for Windows](https://git-scm.com/download/win) must be installed and on PATH.

### Linux (x64)

Download the `.AppImage`, make it executable, and run it:

```sh
chmod +x Lineage_*.AppImage
./Lineage_*.AppImage
```

Lineage idles in the system tray, so the desktop needs tray support (stock GNOME
wants the AppIndicator extension). Token storage needs a Secret Service
(gnome-keyring or KWallet), and `git` must be on PATH.

To build from source instead, see [Development](#development).

## Development

### Prerequisites

- macOS, Windows, or Linux, with `git` on PATH
- [Rust](https://rustup.rs/) via rustup (the version is pinned by `rust-toolchain.toml`)
- Node.js 22+
- Linux only: the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/#linux) (webkit2gtk 4.1 and friends)
- Windows only: the Visual Studio Build Tools with the C++ workload

### Run the app

```sh
npm install
npm run tauri dev
```

This builds the Rust core, starts the Vite dev server, and launches the app.
Frontend changes hot-reload; Rust changes trigger an incremental rebuild.

### Test

```sh
cargo test --manifest-path src-tauri/core/Cargo.toml   # Rust core tests
npm run check                                           # type-check the frontend
npm test                                                # frontend unit tests (Vitest)
```

Prove the engine against your own account without the UI:

```sh
cd src-tauri/core
GH_TOKEN=$(gh auth token) cargo run --example lineage_m0
```

### Build

```sh
npm run tauri build
```

Produces the platform's bundles under `src-tauri/target/release/bundle/`: an
`.app` and `.dmg` on macOS, an NSIS `-setup.exe` on Windows, and an `.AppImage`
on Linux. The builds are unsigned (macOS is ad-hoc signed, not notarized), so
downloaded copies require the first-launch steps under [Installation](#installation).
Builds made locally on your own machine open normally.

### Release (with self-update)

The app checks GitHub Releases for updates on launch and every 6 hours via
`latest.json` attached to the latest release. Releases are built and published by CI:

```sh
# 1. Bump the version in package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json.
# 2. Tag and push; .github/workflows/release.yml does the rest.
git tag vX.Y.Z && git push origin vX.Y.Z
# 3. Review the draft release on GitHub and publish it. Publishing is what
#    makes existing installs see the update.
```

CI signs the updater artifact with the minisign key stored in the repo secrets
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and the app
verifies downloads against the matching public key in `tauri.conf.json`. Release
downloads must be publicly reachable for the in-app check to work.

For a fully local release without CI (macOS-only fallback: `make-update-manifest.sh`
writes just the `darwin-aarch64` entry), build with `TAURI_SIGNING_PRIVATE_KEY` set,
run `./scripts/make-update-manifest.sh`, and upload the dmg, `Lineage.app.tar.gz`,
and `latest.json` with `gh release create`.

## Connect

On first launch, connect GitHub via the `gh` CLI (if installed) or a personal access
token with the `repo` scope. One-click "Sign in with GitHub" (OAuth device flow)
arrives once the OAuth App client id is registered.

## Project structure

```
src/          Svelte 5 frontend (dashboard, onboarding, settings)
src-tauri/    Rust core and Tauri 2 shell
  core/       lineage-core: the Tauri-free domain and engine
openspec/     Product specification and project conventions
```

## Tech stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| Core (API, git engine, aggregation) | Rust (`lineage-core`) |
| Storage | SQLite via rusqlite |
| HTTP / clone | `ureq` (blocking) and the `git` CLI |
| Token storage | OS credential store via `keyring` |
| UI | Svelte 5 + TypeScript |
| Testing | cargo test, Vitest, svelte-check |

## Architecture

A Tauri-free Rust core sits behind a thin desktop shell. The core owns API access,
the git churn engine, language mapping, and aggregation; TypeScript owns view state
and typed IPC calls. The heavy sync runs on a background thread and reports progress
through change events, and the tray title is updated via `TrayIcon::set_title`. UI
code calls the API client (`src/lib/api/client.ts`) rather than invoking Tauri
commands directly.

Product requirements and conventions live in [`openspec/`](openspec/):

- [`openspec/specs/lineage/spec.md`](openspec/specs/lineage/spec.md): requirements and scenarios
- [`openspec/project.md`](openspec/project.md): tech stack, architecture, and workflow conventions

## Contributing

A Jam-Sw project, developed privately.

- Each branch should contain one clear product step
- All changes land via pull request into `main`
- Commit subjects stay short and concrete; bodies explain what changed in one or two sentences
- Core behavior is covered with Rust tests, frontend utilities with Vitest, and components must pass `svelte-check`

## Status

macOS (Apple Silicon), Windows (x64), and Linux (x64). The data engine is proven
and tested; the menu-bar app, dashboard, and release pipeline are in place.
