//! JSON-file persistence for instances: one file per instance under
//! `<data>/instances/<id>.json`.
//!
//! Contract (Milestone 001 spec §14):
//! - Full CRUD over the [`crate::instance::Instance`] aggregate.
//! - One unreadable file never hides the others: `list` skips it and reports
//!   how many were skipped so the UI can warn.
//! - Updates validate before writing; writes are atomic (temp + rename).

use crate::error::{Error, ErrorCode, Result};
use crate::ids::{InstanceId, MinecraftVersionId};
use crate::instance::{Instance, LoaderKind, LoaderSpec};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// Result of listing: instances plus a count of unreadable files so the UI can
/// surface "N instance files could not be read" instead of hiding damage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceListing {
    pub instances: Vec<Instance>,
    pub unreadable_files: u32,
}

/// Create-instance input — the only place where IDs are minted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewInstance {
    pub name: String,
    #[serde(default)]
    pub loader_kind: Option<LoaderKindInput>,
}

/// Loader selection as accepted from the UI. `version` is required unless the
/// kind is vanilla (validated in [`InstanceStore::create`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderKindInput {
    pub kind: LoaderKind,
    pub version: Option<String>,
}

impl Default for LoaderKindInput {
    fn default() -> Self {
        Self {
            kind: LoaderKind::Vanilla,
            version: None,
        }
    }
}

pub struct InstanceStore {
    root: PathBuf,
    counter: AtomicU32,
}

impl InstanceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            counter: AtomicU32::new(0),
        }
    }

    fn path_for(&self, id: &InstanceId) -> PathBuf {
        // IDs are generated here with a restricted alphabet; defensively strip
        // path separators in case a hand-crafted id ever reaches persistence.
        let safe: String = id
            .as_str()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        self.root.join(format!("{safe}.json"))
    }

    fn next_id(&self) -> InstanceId {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        InstanceId::new(format!("inst-{secs:x}-{n:x}"))
    }

    /// Create, persist, and return a new instance. `minecraft_version` is
    /// free-form at this milestone (metadata discovery is a later milestone).
    pub fn create(
        &self,
        name: impl Into<String>,
        minecraft_version: impl Into<String>,
        loader: Option<LoaderKindInput>,
    ) -> Result<Instance> {
        let loader_spec = match loader {
            None => LoaderSpec::default(),
            Some(input) => LoaderSpec {
                kind: input.kind,
                version: input.version,
            },
        };
        if loader_spec.kind != LoaderKind::Vanilla && loader_spec.version.is_none() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                format!("loader {:?} requires a version", loader_spec.kind),
            ));
        }
        let mut instance = Instance::new(
            self.next_id(),
            name,
            MinecraftVersionId::new(minecraft_version.into()),
        )?;
        instance.loader = loader_spec;
        instance.validate()?;
        self.write(&instance)?;
        Ok(instance)
    }

    pub fn list(&self) -> InstanceListing {
        let mut instances = Vec::new();
        let mut unreadable = 0u32;
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(_) => {
                return InstanceListing {
                    instances,
                    unreadable_files: 0,
                }
            } // empty store
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            match fs::read_to_string(&path)
                .map_err(|e| {
                    Error::with_source(
                        ErrorCode::InstanceCorrupt,
                        format!("cannot read {}", path.display()),
                        e,
                    )
                })
                .and_then(|raw| {
                    serde_json::from_str::<Instance>(&raw).map_err(|e| {
                        Error::with_source(
                            ErrorCode::InstanceCorrupt,
                            format!("cannot parse {}", path.display()),
                            e,
                        )
                    })
                }) {
                Ok(instance) => instances.push(instance),
                Err(_) => unreadable += 1,
            }
        }
        instances.sort_by(|a, b| {
            b.created_at_unix
                .cmp(&a.created_at_unix)
                .then(a.id.as_str().cmp(b.id.as_str()))
        });
        InstanceListing {
            instances,
            unreadable_files: unreadable,
        }
    }

    pub fn get(&self, id: &InstanceId) -> Result<Instance> {
        let path = self.path_for(id);
        let raw = fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::new(
                    ErrorCode::InstanceNotFound,
                    format!("no instance with id {id}"),
                )
            } else {
                Error::with_source(
                    ErrorCode::IoFailure,
                    format!("cannot read {}", path.display()),
                    e,
                )
            }
        })?;
        serde_json::from_str::<Instance>(&raw).map_err(|e| {
            Error::with_source(
                ErrorCode::InstanceCorrupt,
                format!("corrupt instance file {}", path.display()),
                e,
            )
        })
    }

    /// Validate and persist an updated instance. The instance must already exist.
    pub fn update(&self, instance: &Instance) -> Result<()> {
        instance.validate()?;
        if !self.path_for(&instance.id).exists() {
            return Err(Error::new(
                ErrorCode::InstanceNotFound,
                format!("no instance with id {}", instance.id),
            ));
        }
        self.write(instance)
    }

    /// Delete an instance file. Returns whether anything was deleted (false =
    /// already gone, which is not an error).
    pub fn delete(&self, id: &InstanceId) -> Result<bool> {
        let path = self.path_for(id);
        match fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot delete {}", path.display()),
                e,
            )),
        }
    }

    fn write(&self, instance: &Instance) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot create {}", self.root.display()),
                e,
            )
        })?;
        let json = serde_json::to_string_pretty(instance).map_err(|e| {
            Error::with_source(ErrorCode::Internal, "instance failed to serialize", e)
        })?;
        let target = self.path_for(&instance.id);
        let tmp = target.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot write {}", tmp.display()),
                e,
            )
        })?;
        fs::rename(&tmp, &target).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot finalize {}", target.display()),
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
    fn create_list_get_roundtrip() {
        let store = InstanceStore::new(unique_temp_dir("inst-roundtrip"));
        let created = store.create("My PvP", "1.21.x", None).unwrap();

        let listing = store.list();
        assert_eq!(listing.instances.len(), 1);
        assert_eq!(listing.unreadable_files, 0);

        let fetched = store.get(&created.id).unwrap();
        assert_eq!(fetched, created);
        assert_eq!(fetched.loader.kind, LoaderKind::Vanilla);
    }

    #[test]
    fn create_with_loader_version_persists() {
        let store = InstanceStore::new(unique_temp_dir("inst-loader"));
        let inst = store
            .create(
                "Fabric dev",
                "1.21.x",
                Some(LoaderKindInput {
                    kind: LoaderKind::Fabric,
                    version: Some("0.16.0".into()),
                }),
            )
            .unwrap();
        assert_eq!(
            store.get(&inst.id).unwrap().loader.version.as_deref(),
            Some("0.16.0")
        );
    }

    #[test]
    fn loader_without_version_is_rejected() {
        let store = InstanceStore::new(unique_temp_dir("inst-badloader"));
        let err = store
            .create(
                "X",
                "1.21.x",
                Some(LoaderKindInput {
                    kind: LoaderKind::Forge,
                    version: None,
                }),
            )
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InstanceInvalid);
    }

    #[test]
    fn update_validates_and_requires_existence() {
        let store = InstanceStore::new(unique_temp_dir("inst-update"));
        let mut inst = store.create("Before", "1.21.x", None).unwrap();

        inst.name = "After".to_owned();
        store.update(&inst).unwrap();
        assert_eq!(store.get(&inst.id).unwrap().name, "After");

        inst.name = "   ".to_owned();
        assert_eq!(
            store.update(&inst).unwrap_err().code(),
            ErrorCode::InstanceInvalid
        );

        inst.name = "Ghost".to_owned();
        inst.id = InstanceId::new("inst-does-not-exist");
        assert_eq!(
            store.update(&inst).unwrap_err().code(),
            ErrorCode::InstanceNotFound
        );
    }

    #[test]
    fn delete_is_idempotent() {
        let store = InstanceStore::new(unique_temp_dir("inst-delete"));
        let inst = store.create("Doomed", "1.21.x", None).unwrap();
        assert!(store.delete(&inst.id).unwrap());
        assert!(
            !store.delete(&inst.id).unwrap(),
            "second delete is a no-op, not an error"
        );
        assert_eq!(
            store.get(&inst.id).unwrap_err().code(),
            ErrorCode::InstanceNotFound
        );
    }

    #[test]
    fn one_corrupt_file_does_not_hide_the_rest() {
        let dir = unique_temp_dir("inst-corrupt");
        let store = InstanceStore::new(&dir);
        store.create("Good", "1.21.x", None).unwrap();
        std::fs::write(dir.join("inst-broken.json"), "{ nope").unwrap();
        std::fs::write(dir.join("notes.txt"), "not an instance").unwrap();

        let listing = store.list();
        assert_eq!(listing.instances.len(), 1);
        assert_eq!(
            listing.unreadable_files, 1,
            "corrupt file counted, others unaffected"
        );
    }
}
