//! ikk-core — shared foundation for all Isekaiyo crates.
//!
//! Contains only what everything genuinely needs: the stable error taxonomy,
//! typed identifiers, and task-event types. Anything more belongs elsewhere.

pub mod error;
pub mod ids;
pub mod task;

pub use error::{Error, ErrorCode, Result};
pub use ids::InstanceId;
