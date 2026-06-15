use std::fmt;

/// Wrapper for secrets (the GitHub token) that must never reach logs.
/// `Debug` and `Display` print `<redacted>`; `expose()` is the only accessor
/// and is greppable in review. Serialization is intentionally NOT implemented.
pub struct Sensitive<T>(pub T);

impl<T> Sensitive<T> {
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Sensitive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T> fmt::Display for Sensitive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_redact() {
        let s = Sensitive("ghp_secret".to_string());
        assert_eq!(format!("{:?}", s), "<redacted>");
        assert_eq!(format!("{}", s), "<redacted>");
        assert_eq!(s.expose(), "ghp_secret");
    }
}
