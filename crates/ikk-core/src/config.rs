//! Versioned application configuration.
//!
//! Safety contract (Milestone 001 spec §9):
//! - Missing file  → safe defaults (first run).
//! - Malformed file → backed up, never destroyed; defaults returned and the
//!   caller is told via [`LoadSource::RecoveredCorrupt`] so it can inform the
//!   user.
//! - Unknown fields → ignored (forward compatible); missing fields → defaults
//!   (`#[serde(default)]`), so new fields never invalidate old files.
//! - Future schema changes → add a migration step in [`ConfigStore::load`]
//!   before bumping [`CONFIG_SCHEMA_VERSION`].

use crate::error::{Error, ErrorCode, Result};
use crate::ids::InstanceId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Current configuration schema version. Bump when the shape changes and add
/// a migration branch in `load`.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Amoled,
    Modern,
    Sakura,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StartPage {
    #[default]
    Home,
    Instances,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub schema_version: u32,
    pub theme: Theme,
    pub start_page: StartPage,
    pub selected_instance: Option<InstanceId>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            theme: Theme::Amoled,
            start_page: StartPage::Home,
            selected_instance: None,
        }
    }
}

/// How the loaded configuration came into being — surfaced to the user so a
/// recovered file is never a silent event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadSource {
    Defaults,
    File,
    RecoveredCorrupt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub config: AppConfig,
    pub source: LoadSource,
    /// Present when the previous file was moved aside (`config.json.corrupt`).
    pub corrupt_backup: Option<PathBuf>,
}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            path: dir.into().join("config.json"),
        }
    }

    /// Load configuration with the full safety contract. Never fails: every
    /// failure mode degrades to defaults plus a diagnostic.
    pub fn load(&self) -> LoadedConfig {
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(_) => {
                return LoadedConfig {
                    config: AppConfig::default(),
                    source: LoadSource::Defaults,
                    corrupt_backup: None,
                }
            }
        };

        match serde_json::from_str::<AppConfig>(&raw) {
            Ok(mut config) => {
                // Migration point: future versions branch on config.schema_version
                // here, transform the value, and only then adopt the new constant.
                config.schema_version = CONFIG_SCHEMA_VERSION;
                LoadedConfig {
                    config,
                    source: LoadSource::File,
                    corrupt_backup: None,
                }
            }
            Err(_parse_error) => {
                let backup = self.path.with_extension("json.corrupt");
                // Best-effort backup; if even this fails we still do not crash.
                let _ = fs::rename(&self.path, &backup);
                LoadedConfig {
                    config: AppConfig::default(),
                    source: LoadSource::RecoveredCorrupt,
                    // The parse error itself is reported by the app layer from
                    // the preserved backup file — core stays free of logging.
                    corrupt_backup: Some(backup),
                }
            }
        }
    }

    /// Persist configuration atomically (write temp, rename over target).
    pub fn save(&self, config: &AppConfig) -> Result<()> {
        let mut to_write = config.clone();
        to_write.schema_version = CONFIG_SCHEMA_VERSION;
        let json = serde_json::to_string_pretty(&to_write).map_err(|e| {
            Error::with_source(
                ErrorCode::ConfigInvalid,
                "configuration failed to serialize",
                e,
            )
        })?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Error::with_source(
                    ErrorCode::IoFailure,
                    format!("cannot create config directory {}", parent.display()),
                    e,
                )
            })?;
        }
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
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_support::unique_temp_dir;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = unique_temp_dir("cfg-missing");
        let store = ConfigStore::new(&dir);
        let loaded = store.load();
        assert_eq!(loaded.source, LoadSource::Defaults);
        assert_eq!(loaded.config, AppConfig::default());
        assert_eq!(loaded.config.theme, Theme::Amoled);
    }

    #[test]
    fn save_then_reload_roundtrips() {
        let dir = unique_temp_dir("cfg-roundtrip");
        let store = ConfigStore::new(&dir);
        let config = AppConfig {
            theme: Theme::Sakura,
            start_page: StartPage::Instances,
            ..AppConfig::default()
        };
        store.save(&config).unwrap();

        let loaded = store.load();
        assert_eq!(loaded.source, LoadSource::File);
        assert_eq!(loaded.config, config);
    }

    #[test]
    fn corrupt_file_is_backed_up_and_recovered() {
        let dir = unique_temp_dir("cfg-corrupt");
        let store = ConfigStore::new(&dir);
        store.save(&AppConfig::default()).unwrap();
        fs::write(dir.join("config.json"), "{ not json !!!").unwrap();

        let loaded = store.load();
        assert_eq!(loaded.source, LoadSource::RecoveredCorrupt);
        assert_eq!(loaded.config, AppConfig::default());
        let backup = loaded.corrupt_backup.unwrap();
        assert!(
            backup.exists(),
            "corrupt file must be preserved for diagnosis"
        );
    }

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let dir = unique_temp_dir("cfg-forward-compat");
        fs::create_dir_all(&dir).unwrap();
        // `future_field` does not exist in the schema; `theme` is missing.
        fs::write(
            dir.join("config.json"),
            r#"{ "schema_version": 1, "future_field": true, "start_page": "instances" }"#,
        )
        .unwrap();

        let loaded = ConfigStore::new(&dir).load();
        assert_eq!(loaded.source, LoadSource::File);
        assert_eq!(
            loaded.config.theme,
            Theme::Amoled,
            "missing field takes default"
        );
        assert_eq!(loaded.config.start_page, StartPage::Instances);
    }
}
