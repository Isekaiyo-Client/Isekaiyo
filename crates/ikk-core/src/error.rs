//! Stable error taxonomy.
//!
//! Rules (docs/architecture.md §7):
//! - `code()` values are stable string categories — UI and diagnostics match on them.
//! - User-facing messages carry recovery suggestions where possible.
//! - No `unwrap`/`expect` in production paths (workspace lint enforces).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    NetworkTimeout,
    ChecksumMismatch,
    InstanceCorrupt,
    AuthTokenExpired,
    JavaNotSuitable,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::NetworkTimeout => "network.timeout",
            ErrorCode::ChecksumMismatch => "download.checksum_mismatch",
            ErrorCode::InstanceCorrupt => "instance.corrupt",
            ErrorCode::AuthTokenExpired => "auth.token_expired",
            ErrorCode::JavaNotSuitable => "java.not_suitable",
            ErrorCode::Internal => "internal.error",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    code: ErrorCode,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), source: None }
    }

    pub fn with_source(
        code: ErrorCode,
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self { code, message: message.into(), source: Some(source.into()) }
    }

    /// Stable machine-readable category. Never changes meaning across releases.
    pub fn code(&self) -> ErrorCode {
        self.code
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable_strings() {
        assert_eq!(ErrorCode::NetworkTimeout.as_str(), "network.timeout");
        assert_eq!(ErrorCode::ChecksumMismatch.as_str(), "download.checksum_mismatch");
    }

    #[test]
    fn error_display_includes_category() {
        let e = Error::new(ErrorCode::InstanceCorrupt, "manifest schema mismatch");
        assert_eq!(e.to_string(), "instance.corrupt: manifest schema mismatch");
        assert_eq!(e.code(), ErrorCode::InstanceCorrupt);
    }

    #[test]
    fn source_is_preserved() {
        let inner = Error::new(ErrorCode::Internal, "boom");
        let outer = Error::with_source(ErrorCode::Internal, "while validating", inner);
        assert!(outer.source().is_some());
    }
}
