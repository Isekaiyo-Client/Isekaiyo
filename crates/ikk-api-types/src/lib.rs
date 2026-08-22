//! ikk-api-types — the typed IPC contract between the launcher UI and core.
//!
//! These structs are the ONLY shapes that cross the boundary. The frontend
//! mirrors them in TypeScript (code generation lands in Phase 1 M2; until then
//! the shell keeps the mirror hand-written and tested by the smoke command).

use serde::{Deserialize, Serialize};

/// Response of the `get_system_info` shell command.
///
/// Proves: frontend -> Tauri -> Rust -> ikk-core, end to end.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemInfo {
    /// Workspace version of this build (from `CARGO_PKG_VERSION`).
    pub app_version: String,
    /// Target triple of the Rust core build.
    pub target: String,
    /// Stable build profile identifier ("debug" | "release").
    pub profile: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_info_is_json_shaped_for_the_frontend() {
        let info = SystemInfo {
            app_version: "0.1.0".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "debug".into(),
        };
        let json = serde_json::to_string(&info).expect("serialization is total here");
        let back: SystemInfo = serde_json::from_str(&json).expect("we just produced this shape");
        assert_eq!(back, info);
    }
}
