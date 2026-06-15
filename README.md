# Master Diff

Your lifetime GitHub "master diff" in the macOS menu bar.

## What it is

Master Diff computes every line you have ever added and removed across your entire
GitHub account, by language, and shows the net number live in your menu bar. Click
through for a dashboard: the headline diff, a per-language breakdown with the real
GitHub language colors, and a per-repository table.

It works by cloning your repositories locally and parsing `git log --numstat` for
commits you authored, which is the only accurate way to get per-language lifetime
churn (the GitHub API cannot split authored diffs by language, and returns nothing
for repositories with 10,000+ commits). Generated and vendored files (node_modules,
lockfiles, minified bundles) are excluded by default so the number reflects code you
actually wrote.

Apple Silicon, local-first: your token lives in the macOS Keychain and every GitHub
call and clone happens on your machine.

## Install

Download the latest `.dmg` from Releases, open it, and drag Master Diff to
Applications. (Coming with M3; until then, build from source below.)

The app is not notarized, so macOS blocks the first launch. Run
`xattr -d com.apple.quarantine "/Applications/Master Diff.app"` or use System
Settings > Privacy & Security > "Open Anyway".

## Develop

Prerequisites: Rust (stable), Node 22+, and the macOS command line tools.

```sh
npm install
npm run tauri dev          # run the app
npm run build              # build the web assets
npm run tauri build        # build the .app / .dmg
npm run check              # type-check the frontend
cargo test --manifest-path src-tauri/core/Cargo.toml   # core tests
```

Prove the engine against your own account without the UI:

```sh
cd src-tauri/core
GH_TOKEN=$(gh auth token) cargo run --example master_diff_m0
```

## Connect

On first launch, connect GitHub via the `gh` CLI (if installed) or a personal access
token with the `repo` scope. One-click "Sign in with GitHub" (OAuth device flow)
arrives with M3.

## Status

Early development. The data engine is proven and tested; the menu-bar app and
dashboard are taking shape. Apple Silicon only.

A Jam-Sw project.
