use thiserror::Error;

/// Structured application error shared by the core and (later) the desktop IPC layer.
/// `code()` returns a stable string the frontend can switch on.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NotConnected(String),
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    Network(String),
    #[error("{0}")]
    RateLimited(String),
    #[error("{0}")]
    Git(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Storage(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotConnected(_) => "NOT_CONNECTED",
            AppError::Auth(_) => "AUTH_ERROR",
            AppError::Network(_) => "NETWORK_ERROR",
            AppError::RateLimited(_) => "RATE_LIMITED",
            AppError::Git(_) => "GIT_ERROR",
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::Storage(_) => "STORAGE_ERROR",
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(AppError::NotConnected("x".into()).code(), "NOT_CONNECTED");
        assert_eq!(AppError::Auth("x".into()).code(), "AUTH_ERROR");
        assert_eq!(AppError::Network("x".into()).code(), "NETWORK_ERROR");
        assert_eq!(AppError::RateLimited("x".into()).code(), "RATE_LIMITED");
        assert_eq!(AppError::Git("x".into()).code(), "GIT_ERROR");
    }
}
