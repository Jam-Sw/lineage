# Changelog

All notable changes to Lineage are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- The graph view now shows your growth: when a sync adds lines, opening the tree plays
  an energy wave out from your avatar along every limb that grew, and those limbs keep
  a soft ambient glow for the session. Tap your avatar to replay the wave.
- Lineage remembers the tree as you last saw it, so growth collects quietly across
  launches and waves in the first time you look at it.

## [0.3.0] - 2026-09-25

### Added
- The Lifetime Contributions tree is now a zoomable fractal: you, then languages,
  repositories, folders and files, each branching from the last with the same rule.
  Finer limbs grow in as you zoom, down to individual files.
- Hovering a branch lights its whole lineage back to you, with a readout of lines
  added and removed and its share of everything you have written.
- Click any branch to dive into it; click your avatar to return.
- Double-click to zoom, and a gentle glide when you release a drag.
- The sync engine records where each repository's churn landed as a pruned folder tree.

### Changed
- Tree layout is built entirely from your data (no random angles or decorative leaves):
  angle by language share, thickness by lines changed, repository length by commits.
- Pan and zoom are smoother: updates are batched per frame and zoom steps ease.
- Save image draws the same tree as the dashboard.
- The first sync after updating reads every repository again (from the local clones)
  to build folder trees.

### Fixed
- Dragging the tree no longer selects text or leaves visual artifacts.
- Hover highlights no longer stick after the pointer moves away.

## [0.2.0] - 2026-07-06

### Added
- Windows (NSIS installer) and Linux (AppImage) builds alongside macOS.
- Live tray number drawn into the icon on Windows and Linux.
- CI builds and tests all three platforms on every pull request.

### Changed
- External links and OAuth open through the platform opener instead of `open`.
- The in-app uninstall is macOS only; other platforms use their native uninstallers.

## [0.1.0] - 2026-06-15

### Added
- Lifetime GitHub master diff in the menu bar: lines added, removed and net across
  every repository you have committed to, public and private.
- Language breakdown, language treemap and the Lifetime Contributions tree.
- Appearance settings and a plus-minus tray icon.
- Onboarding, settings, first-run tour and window lifecycle.
- One-click GitHub sign-in via OAuth device flow.
- In-app updater via GitHub Releases.

[Unreleased]: https://github.com/Jam-Sw/lineage/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Jam-Sw/lineage/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Jam-Sw/lineage/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Jam-Sw/lineage/releases/tag/v0.1.0
