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
    MetadataInvalid,
    InstanceCorrupt,
    InstanceInvalid,
    InstanceNotFound,
    ConfigInvalid,
    AuthTokenExpired,
    JavaNotSuitable,
    JavaNotFound,
    IoFailure,
    Internal,
    /// User or system cancelled an operation — distinct from failure (§62).
    Cancelled,
    /// Disk full during download/write — actionable message possible.
    DiskFull,
    /// Permission denied writing an artifact.
    PermissionDenied,
    /// The platform secure store (Keychain/Credential Manager/secret-service)
    /// could not be used — the UI must communicate this, never fall back to
    /// plaintext (Phase 9 §11).
    CredentialsUnavailable,
    /// Authentication failed for a reason other than expiry (device flow
    /// denied, entitlement missing, malformed provider response).
    AuthFailed,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::NetworkTimeout => "network.timeout",
            ErrorCode::ChecksumMismatch => "download.checksum_mismatch",
            ErrorCode::MetadataInvalid => "metadata.invalid",
            ErrorCode::InstanceCorrupt => "instance.corrupt",
            ErrorCode::InstanceInvalid => "instance.invalid",
            ErrorCode::InstanceNotFound => "instance.not_found",
            ErrorCode::ConfigInvalid => "config.invalid",
            ErrorCode::AuthTokenExpired => "auth.token_expired",
            ErrorCode::JavaNotSuitable => "java.not_suitable",
            ErrorCode::JavaNotFound => "java.not_found",
            ErrorCode::IoFailure => "io.failure",
            ErrorCode::Internal => "internal.error",
            ErrorCode::Cancelled => "operation.cancelled",
            ErrorCode::DiskFull => "io.disk_full",
            ErrorCode::PermissionDenied => "io.permission_denied",
            ErrorCode::CredentialsUnavailable => "credentials.unavailable",
            ErrorCode::AuthFailed => "auth.failed",
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
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(
        code: ErrorCode,
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// Stable machine-readable category. Never changes meaning across releases.
    pub fn code(&self) -> ErrorCode {
        self.code
    }
}

/// Map an I/O error to the most precise stable category (spec §62): the UI can
/// say "disk full" instead of "something went wrong".
pub fn classify_io(err: &std::io::Error) -> ErrorCode {
    match err.kind() {
        std::io::ErrorKind::StorageFull => ErrorCode::DiskFull,
        std::io::ErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
        std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput => ErrorCode::IoFailure,
        _ => {
            // raw_os_error covers platform-specific full-disk codes that
            // ErrorKind misses on some targets.
            match err.raw_os_error() {
                Some(28) => ErrorCode::DiskFull,     // ENOSPC (linux/macos)
                Some(13) => ErrorCode::PermissionDenied, // EACCES
                _ => ErrorCode::IoFailure,
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn error_codes_are_stable_strings() {
        assert_eq!(ErrorCode::NetworkTimeout.as_str(), "network.timeout");
        assert_eq!(
            ErrorCode::ChecksumMismatch.as_str(),
            "download.checksum_mismatch"
        );
    }

    #[test]
    fn error_display_includes_category() {
        let e = Error::new(ErrorCode::InstanceCorrupt, "manifest schema mismatch");
        assert_eq!(e.to_string(), "instance.corrupt: manifest schema mismatch");
        assert_eq!(e.code(), ErrorCode::InstanceCorrupt);
    }

    #[test]
    fn io_errors_map_to_stable_category() {
        let e = Error::with_source(
            ErrorCode::IoFailure,
            "cannot write config",
            std::io::Error::other("disk full"),
        );
        assert_eq!(e.code().as_str(), "io.failure");
        assert!(e.source().is_some());
    }

    #[test]
    fn source_is_preserved() {
        let inner = Error::new(ErrorCode::Internal, "boom");
        let outer = Error::with_source(ErrorCode::Internal, "while validating", inner);
        assert!(outer.source().is_some());
    }
}
