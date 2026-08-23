//! ikk-core — shared foundation for all Isekaiyo crates.
//!
//! Contains only what everything genuinely needs: the stable error taxonomy,
//! typed identifiers, task-event types, platform paths, the versioned
//! configuration store, and the Milestone-001 instance domain + persistence.
//! Anything more belongs elsewhere.

pub mod accounts;
pub mod config;
pub mod credentials;
pub mod error;
pub mod ids;
pub mod instance;
pub mod platform;
pub mod store;
pub mod task;
pub mod tasks;
pub mod version;

#[cfg(test)]
pub(crate) mod test_support;

pub use accounts::{offline_uuid, Account, AccountKind, AccountStatus, AccountStore, AccountsFile};
pub use credentials::CredentialStore;
pub use error::{Error, ErrorCode, Result};
pub use ids::{InstanceId, MinecraftVersionId};
pub use instance::{Instance, LoaderKind, LoaderSpec};
pub use version::{MinecraftVersion, VersionKind};
