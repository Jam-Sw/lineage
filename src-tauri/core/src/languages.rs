//! Map a file path to a language + Linguist color. Curated extension table for
//! M0 (covers the languages in this account plus the common set); M1 swaps the
//! data source for a build-time-generated map from a vendored Linguist
//! `languages.yml` behind this same `classify()` interface.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lang {
    pub name: &'static str,
    pub color: &'static str,
}

const OTHER: Lang = Lang { name: "Other", color: "#bdbdbd" };

/// (extension-without-dot, language, color). Lowercased lookup.
const EXTENSIONS: &[(&str, &str, &str)] = &[
    ("ts", "TypeScript", "#3178c6"),
    ("tsx", "TypeScript", "#3178c6"),
    ("mts", "TypeScript", "#3178c6"),
    ("cts", "TypeScript", "#3178c6"),
    ("js", "JavaScript", "#f1e05a"),
    ("jsx", "JavaScript", "#f1e05a"),
    ("mjs", "JavaScript", "#f1e05a"),
    ("cjs", "JavaScript", "#f1e05a"),
    ("svelte", "Svelte", "#ff3e00"),
    ("vue", "Vue", "#41b883"),
    ("py", "Python", "#3572A5"),
    ("pyi", "Python", "#3572A5"),
    ("ipynb", "Jupyter Notebook", "#DA5B0B"),
    ("rs", "Rust", "#dea584"),
    ("swift", "Swift", "#F05138"),
    ("go", "Go", "#00ADD8"),
    ("rb", "Ruby", "#701516"),
    ("java", "Java", "#b07219"),
    ("kt", "Kotlin", "#A97BFF"),
    ("kts", "Kotlin", "#A97BFF"),
    ("c", "C", "#555555"),
    ("h", "C", "#555555"),
    ("cpp", "C++", "#f34b7d"),
    ("cc", "C++", "#f34b7d"),
    ("cxx", "C++", "#f34b7d"),
    ("hpp", "C++", "#f34b7d"),
    ("cs", "C#", "#178600"),
    ("m", "Objective-C", "#438eff"),
    ("mm", "Objective-C++", "#6866fb"),
    ("php", "PHP", "#4F5D95"),
    ("dart", "Dart", "#00B4AB"),
    ("lua", "Lua", "#000080"),
    ("pl", "Perl", "#0298c3"),
    ("r", "R", "#198CE7"),
    ("sh", "Shell", "#89e051"),
    ("bash", "Shell", "#89e051"),
    ("zsh", "Shell", "#89e051"),
    ("fish", "Shell", "#89e051"),
    ("ps1", "PowerShell", "#012456"),
    ("html", "HTML", "#e34c26"),
    ("htm", "HTML", "#e34c26"),
    ("css", "CSS", "#563d7c"),
    ("scss", "SCSS", "#c6538c"),
    ("sass", "Sass", "#a53b70"),
    ("less", "Less", "#1d365d"),
    ("md", "Markdown", "#083fa1"),
    ("markdown", "Markdown", "#083fa1"),
    ("mdx", "MDX", "#fcb32c"),
    ("json", "JSON", "#cccccc"),
    ("jsonc", "JSON", "#cccccc"),
    ("yml", "YAML", "#cb171e"),
    ("yaml", "YAML", "#cb171e"),
    ("toml", "TOML", "#9c4221"),
    ("xml", "XML", "#0060ac"),
    ("sql", "SQL", "#e38c00"),
    ("graphql", "GraphQL", "#e10098"),
    ("gql", "GraphQL", "#e10098"),
    ("proto", "Protocol Buffer", "#3D9970"),
    ("vim", "Vim Script", "#199f4b"),
    ("ex", "Elixir", "#6e4a7e"),
    ("exs", "Elixir", "#6e4a7e"),
    ("scala", "Scala", "#c22d40"),
    ("clj", "Clojure", "#db5855"),
    ("hs", "Haskell", "#5e5086"),
    ("elm", "Elm", "#60B5CC"),
    ("ml", "OCaml", "#3be133"),
    ("nim", "Nim", "#ffc200"),
    ("zig", "Zig", "#ec915c"),
    ("astro", "Astro", "#ff5a03"),
    ("tf", "HCL", "#844FBA"),
    ("hcl", "HCL", "#844FBA"),
    ("dockerfile", "Dockerfile", "#384d54"),
    ("txt", "Text", "#bdbdbd"),
    ("csv", "CSV", "#bdbdbd"),
    ("plist", "XML", "#0060ac"),
    ("xcconfig", "Xcode Config", "#bdbdbd"),
    ("entitlements", "XML", "#0060ac"),
];

/// Some files are identified by name, not extension.
const FILENAMES: &[(&str, &str, &str)] = &[
    ("dockerfile", "Dockerfile", "#384d54"),
    ("makefile", "Makefile", "#427819"),
    ("rakefile", "Ruby", "#701516"),
    ("gemfile", "Ruby", "#701516"),
    ("cargo.lock", "TOML", "#9c4221"),
    (".gitignore", "Ignore List", "#000000"),
];

/// Strip a numstat rename path (`old => new`, possibly `pre{old => new}post`)
/// down to the resulting path so the extension reflects the final file.
pub fn normalize_rename(path: &str) -> String {
    if let Some(idx) = path.find(" => ") {
        let after = &path[idx + 4..];
        let after = after.trim_end_matches('}');
        // Handle `pre{old => new}post`: drop the trailing `}post` already trimmed;
        // a leading `pre{` (before the brace) is dropped via the after-arrow split.
        return after.replace('}', "");
    }
    path.to_string()
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Recover the canonical color for a language name (reverse of `classify`).
/// Returns the `Other` grey for unknown names.
pub fn color_of(language: &str) -> &'static str {
    for (_, name, color) in EXTENSIONS {
        if *name == language {
            return color;
        }
    }
    for (_, name, color) in FILENAMES {
        if *name == language {
            return color;
        }
    }
    OTHER.color
}

/// Classify a (possibly rename-formatted) numstat path into a language.
pub fn classify(path: &str) -> Lang {
    let norm = normalize_rename(path);
    let base = basename(&norm).to_ascii_lowercase();

    for (name, lang, color) in FILENAMES {
        if base == *name {
            return Lang { name: lang, color };
        }
    }

    if let Some(dot) = base.rfind('.') {
        let ext = &base[dot + 1..];
        for (e, lang, color) in EXTENSIONS {
            if ext == *e {
                return Lang { name: lang, color };
            }
        }
    }
    OTHER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_extensions() {
        assert_eq!(classify("src/app.ts").name, "TypeScript");
        assert_eq!(classify("a/b/c.rs").name, "Rust");
        assert_eq!(classify("Page.svelte").name, "Svelte");
        assert_eq!(classify("deep/main.py").name, "Python");
        assert_eq!(classify("style.scss").name, "SCSS");
    }

    #[test]
    fn classifies_by_filename() {
        assert_eq!(classify("Dockerfile").name, "Dockerfile");
        assert_eq!(classify("services/Makefile").name, "Makefile");
    }

    #[test]
    fn unknown_is_other() {
        assert_eq!(classify("weird.qqq").name, "Other");
        assert_eq!(classify("README").name, "Other");
    }

    #[test]
    fn handles_rename_paths() {
        assert_eq!(normalize_rename("old.ts => new.ts"), "new.ts");
        assert_eq!(classify("src/{old.js => new.ts}"), Lang { name: "TypeScript", color: "#3178c6" });
        assert_eq!(classify("lib/old.py => lib/new.py").name, "Python");
    }
}
