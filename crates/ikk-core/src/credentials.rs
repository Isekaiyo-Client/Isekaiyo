//! Secure credential storage abstraction (Phase 9 §10–§11).
//!
//! Rules:
//! - Tokens live ONLY behind this trait — never in instance JSON, account
//!   metadata, logs, frontend state, or general config.
//! - There is NO plaintext-file fallback. If the platform cannot provide
//!   secure storage, operations fail with `credentials.unavailable` so the UI
//!   can clearly communicate the limitation (spec §11: fail safely, never
//!   silently save plaintext).
//! - The trait object lives in the application shell, which wires the
//!   platform backend (Windows Credential Manager / macOS Keychain /
//!   Linux secret-service via the `keyring` crate). Core stays dependency-free.

use crate::error::{Error, ErrorCode, Result};

/// Storage for one named secret. Keys are opaque strings namespaced by the
/// caller (e.g. `msa/<account-id>/refresh`).
pub trait CredentialStore: Send + Sync {
    /// Persist (or overwrite) a secret. Fails rather than degrading to
    /// plaintext storage.
    fn store(&self, key: &str, secret: &str) -> Result<()>;

    /// Read a secret back. `Ok(None)` = not present (not an error).
    fn retrieve(&self, key: &str) -> Result<Option<String>>;

    /// Remove a secret. Removing an absent key is success (idempotent logout,
    /// spec §13).
    fn delete(&self, key: &str) -> Result<()>;
}

/// Error helper for backends that cannot operate on this platform/session.
pub fn unavailable(what: &str) -> crate::Error {
    Error::new(
        ErrorCode::CredentialsUnavailable,
        format!("secure credential storage unavailable: {what}"),
    )
}

/// In-memory store for unit tests ONLY. Never used in production wiring;
/// its existence is documented here so nobody "helpfully" reaches for it as
/// a fallback (that would be silent-plaintext by another name).
#[cfg(test)]
pub struct MemoryCredentialStore {
    entries: std::sync::Mutex<std::collections::BTreeMap<String, String>>,
}

#[cfg(test)]
impl Default for MemoryCredentialStore {
    fn default() -> Self {
        Self {
            entries: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }
}

#[cfg(test)]
impl CredentialStore for MemoryCredentialStore {
    fn store(&self, key: &str, secret: &str) -> Result<()> {
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(key.to_owned(), secret.to_owned());
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(key)
            .cloned())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn roundtrip_and_idempotent_delete() {
        let store = MemoryCredentialStore::default();
        assert_eq!(store.retrieve("k").unwrap(), None);

        store.store("k", "secret").unwrap();
        assert_eq!(store.retrieve("k").unwrap().as_deref(), Some("secret"));
        // Overwrite semantics.
        store.store("k", "rotated").unwrap();
        assert_eq!(store.retrieve("k").unwrap().as_deref(), Some("rotated"));

        store.delete("k").unwrap();
        assert_eq!(store.retrieve("k").unwrap(), None);
        // Deleting absent keys succeeds (logout must never fail on absence).
        store.delete("k").unwrap();
    }
}
