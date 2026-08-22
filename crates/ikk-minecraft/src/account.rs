//! LaunchIdentity — the identity a launch actually uses.
//!
//! The planner consumes this and only this; it never sees account stores,
//! credential vaults, or tokens beyond the one-shot access token string it
//! must place into game args. Built from the launcher's account layer
//! (`ikk-core::account`) by the application layer — never by the UI.

use ikk_core::error::{Error, ErrorCode, Result};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdentityKind {
    /// Local profile: singleplayer/LAN only, no authenticated services.
    OfflineProfile,
    /// Microsoft-authenticated Minecraft identity.
    Microsoft,
}

/// Everything the launch planner needs to know about WHO is playing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchIdentity {
    pub kind: IdentityKind,
    pub username: String,
    pub uuid: String,
    /// One-shot token placed into `--accessToken`. For offline profiles this
    /// is the vanilla-convention `"0"` — it is NOT an authentication bypass;
    /// authenticated servers reject it, which is exactly the point.
    pub access_token: String,
}

/// Classic Mojang username constraints, enforced for offline profiles so a
/// typo cannot produce a broken session.
fn validate_username(name: &str) -> Result<()> {
    let ok_len = (1..=16).contains(&name.chars().count());
    let ok_chars = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ok_len && ok_chars {
        Ok(())
    } else {
        Err(Error::new(
            ErrorCode::InstanceInvalid,
            "username must be 1–16 characters of A–Z, a–z, 0–9, _",
        ))
    }
}

/// Deterministic offline UUID: MD5(`OfflinePlayer:<name>`) formatted as a
/// v3 UUID — the same convention vanilla's offline mode uses, so a given name
/// always maps to the same UUID on every machine.
pub fn offline_uuid(username: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{username}").as_bytes());
    let digest = hasher.finalize();
    let mut bytes = digest;
    bytes[6] = (bytes[6] & 0x0f) | 0x30; // version 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

impl LaunchIdentity {
    /// An offline profile identity. Honest by construction: token `"0"` gets
    /// rejected by authenticated servers; nothing pretends otherwise.
    pub fn offline(username: impl Into<String>) -> Result<Self> {
        let username = username.into();
        validate_username(&username)?;
        Ok(Self {
            kind: IdentityKind::OfflineProfile,
            uuid: offline_uuid(&username),
            username,
            access_token: "0".to_owned(),
        })
    }

    /// A Microsoft-authenticated identity resolved by the application layer
    /// (profile data from Mojang, token fetched from the secure store).
    pub fn microsoft(
        username: impl Into<String>,
        uuid: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Result<Self> {
        let username = username.into();
        let uuid = uuid.into();
        let access_token = access_token.into();
        validate_username(&username)?;
        if uuid.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::AuthTokenExpired,
                "Microsoft identity is missing its Minecraft profile UUID",
            ));
        }
        if access_token.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::AuthTokenExpired,
                "Microsoft identity has no access token — reauthentication required",
            ));
        }
        Ok(Self {
            kind: IdentityKind::Microsoft,
            uuid,
            username,
            access_token,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn offline_uuid_is_deterministic_and_v3() {
        let a = offline_uuid("Steve");
        let b = offline_uuid("Steve");
        assert_eq!(a, b);
        assert_ne!(offline_uuid("Alex"), a);
        // version 3 nibble + RFC variant
        assert!(a.chars().nth(14).unwrap() == '3');
        assert!(["8", "9", "a", "b"].contains(&a.chars().nth(19).unwrap().to_string().as_str()));
        assert_eq!(a.len(), 36);
    }

    #[test]
    fn offline_identity_uses_placeholder_token() {
        let id = LaunchIdentity::offline("Steve").unwrap();
        assert_eq!(id.kind, IdentityKind::OfflineProfile);
        assert_eq!(id.access_token, "0");
        assert_eq!(id.uuid, offline_uuid("Steve"));
    }

    #[test]
    fn invalid_usernames_are_rejected() {
        assert!(LaunchIdentity::offline("").is_err());
        assert!(LaunchIdentity::offline("this_name_is_way_too_long").is_err());
        assert!(LaunchIdentity::offline("bad name!").is_err());
        assert!(LaunchIdentity::offline("Valid_Name1").is_ok());
    }

    #[test]
    fn microsoft_identity_requires_all_parts() {
        assert!(LaunchIdentity::microsoft("Steve", "uuid", "token").is_ok());
        assert!(LaunchIdentity::microsoft("Steve", "", "token").is_err());
        assert!(LaunchIdentity::microsoft("Steve", "uuid", " ").is_err());
    }
}
