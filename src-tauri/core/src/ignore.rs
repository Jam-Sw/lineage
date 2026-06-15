//! Identify generated/vendored paths that inflate churn (node_modules, lock files,
//! minified bundles, build output). M0 proved these dominate raw numbers - excluding
//! them turns a misleading net of -853k into the real authored net of +389k.
//! Patterns mirror the common cases from GitHub Linguist's vendored/generated lists.

/// Directory segments whose contents are vendored or build output.
const VENDORED_DIRS: &[&str] = &[
    "node_modules",
    "bower_components",
    "vendor",
    "third_party",
    "dist",
    "build",
    "out",
    "target",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "coverage",
    "Pods",
    "Carthage",
    "__pycache__",
    ".venv",
    "venv",
    "site-packages",
    ".cache",
];

/// Exact filenames that are machine-generated dependency lockfiles.
const LOCKFILES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "composer.lock",
    "Cargo.lock",
    "Gemfile.lock",
    "poetry.lock",
    "Podfile.lock",
];

/// Filename suffixes that mark generated/minified/log/backup artifacts.
const GENERATED_SUFFIXES: &[&str] =
    &[".min.js", ".min.css", ".bundle.js", ".map", ".log", ".bak", ".lock"];

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// True if the path is vendored, a lockfile, or a generated artifact.
pub fn is_generated(path: &str) -> bool {
    // Normalize a numstat rename path down to the resulting file first.
    let norm = crate::languages::normalize_rename(path);

    for seg in norm.split('/') {
        if VENDORED_DIRS.iter().any(|d| seg == *d) {
            return true;
        }
    }
    let base = basename(&norm);
    if LOCKFILES.iter().any(|f| base == *f) {
        return true;
    }
    let lower = base.to_ascii_lowercase();
    if GENERATED_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_vendored_dirs() {
        assert!(is_generated("node_modules/x/y.js"));
        assert!(is_generated("app/dist/bundle.js"));
        assert!(is_generated("src-tauri/target/debug/foo.rs"));
        assert!(is_generated("third_party/lib/a.c"));
    }

    #[test]
    fn flags_lockfiles_and_artifacts() {
        assert!(is_generated("package-lock.json"));
        assert!(is_generated("sub/yarn.lock"));
        assert!(is_generated("a/b.min.js"));
        assert!(is_generated("dist/app.js.map"));
        assert!(is_generated("server.log"));
    }

    #[test]
    fn passes_real_source() {
        assert!(!is_generated("src/app.ts"));
        assert!(!is_generated("lib/store.rs"));
        assert!(!is_generated("README.md"));
        assert!(!is_generated("config.json"));
    }
}
