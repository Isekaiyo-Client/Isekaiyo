//! The account domain (Phase 9): who is launching Minecraft.
//!
//! Architectural rules:
//! - Accounts and instances are separate aggregates; an instance never stores
//!   account data, an account never stores instance data (spec §1).
//! - This model is PUBLIC metadata only — no tokens, no secrets, ever
//!   (spec §3/§4). Secrets live behind [`crate::credentials::CredentialStore`].
//! - Persistence is versioned (`schema_version`) with corrupt-file backup,
//!   mirroring [`crate::config`]'s safety contract: a malformed file is moved
//!   aside and defaults returned, never destroyed, and the caller is told.

use crate::error::{Error, ErrorCode, Result};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Current account-storage schema version (spec §36). Bump + migrate; never
/// make future schema changes destructive.
pub const ACCOUNTS_SCHEMA_VERSION: u32 = 1;

/// What kind of identity this is. New kinds (Demo, Custom…) are additive enum
/// variants — nothing else in the system may hard-code this list (spec §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    /// Microsoft-authenticated Minecraft identity.
    Microsoft,
    /// Explicit local profile. Clearly labeled Offline everywhere; it must
    /// never pretend to possess Microsoft entitlements (spec §17–§18).
    Offline,
}

impl AccountKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountKind::Microsoft => "microsoft",
            AccountKind::Offline => "offline",
        }
    }
}

/// Explicit authentication states (spec §8). The frontend reads these; it
/// never guesses state from absence of data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    SignedOut,
    Authenticated,
    Refreshing,
    Expired,
    /// Metadata exists but secure credentials do not (spec §35): the user must
    /// sign in again. Not an error state — a recoverable condition.
    ReauthRequired,
    Error,
}

impl AccountStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountStatus::SignedOut => "signed-out",
            AccountStatus::Authenticated => "authenticated",
            AccountStatus::Refreshing => "refreshing",
            AccountStatus::Expired => "expired",
            AccountStatus::ReauthRequired => "reauth-required",
            AccountStatus::Error => "error",
        }
    }
}

/// PUBLIC account metadata. Deliberately contains zero credential fields —
/// this exact struct is safe to send to the frontend, cache, log, or persist
/// (spec §4/§33). Anything secret lives only in the credential store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
    pub display_name: String,
    pub username: String,
    /// Minecraft profile UUID (stable across launches — spec §19).
    pub uuid: String,
    /// Avatar/skin URL when a legitimate API provided one; None → UI renders
    /// its deterministic fallback (spec §20/§56).
    pub avatar_url: Option<String>,
    pub status: AccountStatus,
    pub created_at_unix: u64,
    pub last_used_at_unix: u64,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Deterministic offline UUID — MD5(`OfflinePlayer:<name>`) as a v3 UUID, the
/// vanilla offline-mode convention. Same name ⇒ same UUID on every machine,
/// forever (spec §19: never regenerate per launch).
///
/// This is THE single implementation in the workspace;
/// `ikk_minecraft::account` delegates here.
pub fn offline_uuid(username: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{username}").as_bytes());
    let mut bytes: [u8; 16] = hasher.finalize().into();
    bytes[6] = (bytes[6] & 0x0f) | 0x30; // version 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32]
    )
}

/// Classic Mojang username constraints, enforced at creation so a typo cannot
/// produce a broken session later.
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

/// On-disk document. Versioned for future migrations (spec §36); unknown
/// fields ignored via serde so additive changes never invalidate old files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountsFile {
    pub schema_version: u32,
    pub accounts: Vec<Account>,
    /// Selected account that launch preparation resolves; `None` until the
    /// user picks one (never silently defaulted — spec §16).
    pub active_account_id: Option<String>,
}

impl Default for AccountsFile {
    fn default() -> Self {
        Self {
            schema_version: ACCOUNTS_SCHEMA_VERSION,
            accounts: Vec::new(),
            active_account_id: None,
        }
    }
}

/// Outcome of loading the accounts file — surfaced to the UI like config's
/// recovery events instead of hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedAccounts {
    pub file: AccountsFile,
    /// Set when the previous file was malformed and preserved elsewhere.
    pub corrupt_backup: Option<PathBuf>,
}

/// Persistence + management of public account metadata.
///
/// Secrets NEVER pass through this type — see [`crate::credentials`].
pub struct AccountStore {
    path: PathBuf,
}

impl AccountStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Load with the corrupt-file safety contract: malformed data is backed
    /// up (`accounts.json.corrupt-<stamp>`), defaults returned, caller told.
    pub fn load(&self) -> LoadedAccounts {
        match fs::read_to_string(&self.path) {
            Ok(raw) => match serde_json::from_str::<AccountsFile>(&raw) {
                Ok(mut file) => {
                    if file.schema_version < ACCOUNTS_SCHEMA_VERSION {
                        // v1 is the first version; migrations land here when
                        // v2 exists. Serde defaults already filled new fields.
                        file.schema_version = ACCOUNTS_SCHEMA_VERSION;
                    }
                    LoadedAccounts {
                        file,
                        corrupt_backup: None,
                    }
                }
                Err(_) => {
                    let backup = self.backup_corrupt();
                    LoadedAccounts {
                        file: AccountsFile::default(),
                        corrupt_backup: Some(backup),
                    }
                }
            },
            Err(_) => LoadedAccounts {
                file: AccountsFile::default(),
                corrupt_backup: None,
            }, // first run / unreadable dir → empty store
        }
    }

    fn backup_corrupt(&self) -> PathBuf {
        let stamp = unix_now();
        let backup = self.path.with_extension(format!("json.corrupt-{stamp}"));
        let _ = fs::rename(&self.path, &backup);
        backup
    }

    /// Persist atomically (temp + rename).
    pub fn save(&self, file: &AccountsFile) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Error::with_source(
                    ErrorCode::IoFailure,
                    format!("cannot create {}", parent.display()),
                    e,
                )
            })?;
        }
        let json =
            serde_json::to_string_pretty(file).map_err(|e| {
                Error::with_source(ErrorCode::Internal, "accounts failed to serialize", e)
            })?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot write {}", tmp.display()),
                e,
            )
        })?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot finalize {}", self.path.display()),
                e,
            )
        })
    }

    /// Add an offline/local profile. Stable UUID derived from the username;
    /// clearly typed `Offline`, never presented as authenticated (spec §17).
    pub fn add_offline(
        &self,
        file: &mut AccountsFile,
        display_name: impl Into<String>,
        username: impl Into<String>,
    ) -> Result<Account> {
        let username = username.into();
        validate_username(&username)?;
        let uuid = offline_uuid(&username);
        if file.accounts.iter().any(|a| a.uuid == uuid && a.kind == AccountKind::Offline) {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                format!("an offline profile for {username} already exists"),
            ));
        }
        let now = unix_now();
        let id = format!("acct-offline-{:x}", now ^ rand_suffix());
        let account = Account {
            id,
            kind: AccountKind::Offline,
            display_name: display_name.into(),
            username,
            uuid,
            avatar_url: None,
            status: AccountStatus::Authenticated,
            created_at_unix: now,
            last_used_at_unix: now,
        };
        file.accounts.push(account.clone());
        self.save(file)?;
        Ok(account)
    }

    /// Record a Microsoft account from validated profile data (profile name +
    /// UUID come from Mojang's services API, never user input). Credentials
    /// were already stored by the CALLER before this runs.
    #[allow(clippy::too_many_arguments)]
    pub fn add_microsoft(
        &self,
        file: &mut AccountsFile,
        display_name: impl Into<String>,
        username: impl Into<String>,
        uuid: impl Into<String>,
        avatar_url: Option<String>,
    ) -> Result<Account> {
        let username = username.into();
        let uuid = uuid.into();
        validate_username(&username)?;
        if uuid.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::MetadataInvalid,
                "refusing to record a Microsoft account without its profile UUID",
            ));
        }
        if file.accounts.iter().any(|a| a.kind == AccountKind::Microsoft && a.uuid == uuid) {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "this Microsoft account has already been added",
            ));
        }
        let now = unix_now();
        let id = format!("acct-msa-{:x}", now ^ rand_suffix());
        let account = Account {
            id,
            kind: AccountKind::Microsoft,
            display_name: display_name.into(),
            username,
            uuid,
            avatar_url,
            status: AccountStatus::Authenticated,
            created_at_unix: now,
            last_used_at_unix: now,
        };
        file.accounts.push(account.clone());
        self.save(file)?;
        Ok(account)
    }

    /// Remove account metadata. The CALLER deletes credentials first (this
    /// layer cannot see them by design). Clearing the active selection when
    /// it pointed at the removed account happens automatically (spec §37).
    pub fn remove(&self, file: &mut AccountsFile, account_id: &str) -> Result<bool> {
        let before = file.accounts.len();
        file.accounts.retain(|a| a.id != account_id);
        let removed = file.accounts.len() != before;
        if removed && file.active_account_id.as_deref() == Some(account_id) {
            file.active_account_id = None;
        }
        if removed {
            self.save(file)?;
        }
        Ok(removed)
    }

    /// Select the active account (spec §14–§15). Unknown ids are refused.
    pub fn select(&self, file: &mut AccountsFile, account_id: Option<&str>) -> Result<()> {
        if let Some(id) = account_id {
            if !file.accounts.iter().any(|a| a.id == id) {
                return Err(Error::new(
                    ErrorCode::InstanceNotFound,
                    format!("no account with id {id}"),
                ));
            }
        }
        file.active_account_id = account_id.map(str::to_owned);
        self.save(file)
    }

    /// Bump `last_used` after a successful launch preparation.
    pub fn touch(&self, file: &mut AccountsFile, account_id: &str) -> Result<()> {
        if let Some(account) = file.accounts.iter_mut().find(|a| a.id == account_id) {
            account.last_used_at_unix = unix_now();
            self.save(file)?;
        }
        Ok(())
    }

    /// Set a new authentication status and persist.
    pub fn set_status(
        &self,
        file: &mut AccountsFile,
        account_id: &str,
        status: AccountStatus,
    ) -> Result<()> {
        match file.accounts.iter_mut().find(|a| a.id == account_id) {
            Some(account) => {
                account.status = status;
                self.save(file)
            }
            None => Err(Error::new(
                ErrorCode::InstanceNotFound,
                format!("no account with id {account_id}"),
            )),
        }
    }
}

/// Tiny non-crypto uniqueness salt so two accounts created in the same second
/// still get distinct ids. NOT a security mechanism — just an id disambiguator.
fn rand_suffix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_support::unique_temp_dir;

    fn store(tag: &str) -> (AccountStore, PathBuf) {
        let dir = unique_temp_dir(tag);
        (AccountStore::new(dir.join("accounts.json")), dir.join("accounts.json"))
    }

    #[test]
    fn offline_uuid_is_stable_and_v3_shaped() {
        let a = offline_uuid("Steve");
        assert_eq!(a, offline_uuid("Steve"));
        assert_ne!(a, offline_uuid("Alex"));
        assert_eq!(a.len(), 36);
        assert_eq!(a.chars().nth(14).unwrap(), '3', "version 3 nibble");
        assert!(
            ["8", "9", "a", "b"].contains(&a.chars().nth(19).unwrap().to_string().as_str()),
            "RFC variant bits"
        );
    }

    #[test]
    fn add_select_remove_roundtrip_and_active_clearing() {
        let (store, _) = store("acct-roundtrip");
        let mut file = AccountsFile::default();

        let steve = store.add_offline(&mut file, "Steve", "Steve").unwrap();
        let alex = store.add_offline(&mut file, "Alex", "Alex").unwrap();
        assert_ne!(steve.id, alex.id);
        assert_ne!(steve.uuid, alex.uuid);
        assert_eq!(file.accounts.len(), 2);

        store.select(&mut file, Some(&alex.id)).unwrap();
        assert_eq!(file.active_account_id.as_deref(), Some(alex.id.as_str()));

        // Removing the ACTIVE account clears the selection (spec §37).
        assert!(store.remove(&mut file, &alex.id).unwrap());
        assert_eq!(file.active_account_id, None);

        // Selecting an unknown id is refused, not silently accepted.
        assert!(store.select(&mut file, Some("ghost")).is_err());
    }

    #[test]
    fn duplicate_profiles_are_rejected_with_clear_errors() {
        let (store, _) = store("acct-dup");
        let mut file = AccountsFile::default();
        store.add_offline(&mut file, "Steve", "Steve").unwrap();
        assert!(store.add_offline(&mut file, "Steve", "Steve").is_err());

        let msa = store
            .add_microsoft(&mut file, "Steve", "Steve", "msa-uuid-1", None)
            .unwrap();
        assert_eq!(msa.kind, AccountKind::Microsoft);
        assert!(store
            .add_microsoft(&mut file, "Steve", "Steve", "msa-uuid-1", None)
            .is_err());
        assert!(store
            .add_microsoft(&mut file, "Steve", "Steve", "", None)
            .is_err(), "no UUID, no record");
    }

    #[test]
    fn corrupt_file_is_backed_up_not_destroyed() {
        let dir = unique_temp_dir("acct-corrupt");
        let path = dir.join("accounts.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "{ definitely not json").unwrap();

        let loaded = AccountStore::new(&path).load();
        assert_eq!(loaded.file.accounts.len(), 0, "defaults on corrupt");
        let backup = loaded.corrupt_backup.expect("corrupt file preserved");
        assert!(backup.exists(), "backup exists at {}", backup.display());
        assert!(!path.exists(), "original moved aside");
    }

    #[test]
    fn persistence_survives_a_store_restart() {
        let dir = unique_temp_dir("acct-restart");
        let path = dir.join("accounts.json");
        let s1 = AccountStore::new(&path);
        let mut f1 = s1.load().file;
        let acct = s1.add_offline(&mut f1, "Herobrine", "NotchFan99").unwrap();
        s1.select(&mut f1, Some(&acct.id)).unwrap();

        // Fresh store over the same file behaves like app restart (spec: #9).
        let s2 = AccountStore::new(&path);
        let loaded = s2.load();
        assert_eq!(loaded.file.accounts.len(), 1);
        assert_eq!(loaded.file.active_account_id.as_deref(), Some(acct.id.as_str()));
        assert_eq!(loaded.file.accounts[0].uuid, offline_uuid("NotchFan99"));
    }

    #[test]
    fn status_transitions_persist() {
        let (store, _) = store("acct-status");
        let mut file = AccountsFile::default();
        let acct = store.add_offline(&mut file, "S", "Steve").unwrap();
        store.set_status(&mut file, &acct.id, AccountStatus::Expired).unwrap();
        assert_eq!(file.accounts[0].status, AccountStatus::Expired);
        assert!(store.set_status(&mut file, "ghost", AccountStatus::Error).is_err());
    }
}
