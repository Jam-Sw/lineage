# Changelog

All notable changes to Lineage are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

## [0.4.0] - 2026-09-29

### Added
- The graph view now shows growth.
- Icon was updated, new style.
- Save Image function restored.

## [0.3.0] - 2026-09-25

### Added
- The Lifetime Contributions tree is zoomable.
- Explore your github lineage through branches

### Changed
- Tree layout derived entirely from your github stats:
  angle by language, thickness by change size, repository length by commits.
- Pan and zoom are more defined
- The first sync after updating = rebuild.

### Fixed
- No more visual artifacting when panning.
- Hover highlights no longer stick.

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

[Unreleased]: https://github.com/Jam-Sw/lineage/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/Jam-Sw/lineage/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Jam-Sw/lineage/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Jam-Sw/lineage/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Jam-Sw/lineage/releases/tag/v0.1.0
