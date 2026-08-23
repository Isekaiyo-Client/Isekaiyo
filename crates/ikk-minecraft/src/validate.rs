//! Instance validation and repair planning (Phase 8 §42–§43).
//!
//! The validator produces STRUCTURED findings — never a boolean buried in a
//! string — so the UI can show "client jar missing" next to the fix that
//! repairs it. Every finding maps to at most one [`RepairAction`]; nothing is
//! deleted automatically (spec §43: never blindly delete an instance).

use ikk_core::error::{Error, ErrorCode, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Cosmetic / recoverable later (e.g. missing empty directory).
    Warning,
    /// Blocks launch.
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    /// Stable machine-readable category (an `ErrorCode` string).
    pub code: &'static str,
    /// Filesystem location the finding is about, when meaningful.
    pub path: Option<PathBuf>,
    pub message: String,
}

impl Finding {
    fn error(code: &'static str, path: impl Into<Option<PathBuf>>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            path: path.into(),
            message: message.into(),
        }
    }
}

/// One repairable problem. Downloads reuse the verified download engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairAction {
    /// Fetch an artifact back into place (missing/corrupt client jar, library,
    /// asset index…).
    Redownload {
        url: String,
        dest: PathBuf,
        sha1: Option<String>,
    },
    /// Re-create an expected directory (mods/, saves/, natives root…).
    CreateDirectory(PathBuf),
}

#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub findings: Vec<Finding>,
    pub repairs: Vec<RepairAction>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.findings.iter().all(|f| f.severity != Severity::Error)
    }

    fn push(&mut self, finding: Finding, repair: Option<RepairAction>) {
        if let Some(action) = repair {
            self.repairs.push(action);
        }
        self.findings.push(finding);
    }
}

/// Input describing what a healthy installation of one artifact looks like.
#[derive(Debug, Clone)]
pub struct ExpectedArtifact {
    pub dest: PathBuf,
    pub url: String,
    pub sha1: Option<String>,
}

/// Validate an instance's on-disk state against its expectations.
///
/// Deliberately pure filesystem checks: no network, no process spawning, so
/// it runs in tests and before every launch without side effects.
pub struct InstanceValidator;

impl InstanceValidator {
    pub fn validate(
        game_dir: &Path,
        expected_artifacts: &[ExpectedArtifact],
        java_executable: &Path,
    ) -> Result<ValidationReport> {
        let mut report = ValidationReport::default();

        // 1. Game directory itself must exist and be writable-ish.
        if !game_dir.exists() {
            report.push(
                Finding::error(
                    "instance.corrupt",
                    game_dir.to_path_buf(),
                    format!("game directory {} does not exist", game_dir.display()),
                ),
                Some(RepairAction::CreateDirectory(game_dir.to_path_buf())),
            );
        }

        // 2. Every expected artifact present and hash-valid.
        for artifact in expected_artifacts {
            if !artifact.dest.exists() {
                report.push(
                    Finding::error(
                        "instance.corrupt",
                        artifact.dest.clone(),
                        format!("missing {}", artifact.dest.display()),
                    ),
                    Some(RepairAction::Redownload {
                        url: artifact.url.clone(),
                        dest: artifact.dest.clone(),
                        sha1: artifact.sha1.clone(),
                    }),
                );
                continue;
            }
            if let Some(hash) = &artifact.sha1 {
                let actual = crate::download::file_hash(&artifact.dest, crate::download::HashKind::Sha1)?;
                if actual != *hash {
                    report.push(
                        Finding::error(
                            "download.checksum_mismatch",
                            artifact.dest.clone(),
                            format!(
                                "{} is corrupt (sha1 mismatch)",
                                artifact.dest.display()
                            ),
                        ),
                        Some(RepairAction::Redownload {
                            url: artifact.url.clone(),
                            dest: artifact.dest.clone(),
                            sha1: artifact.sha1.clone(),
                        }),
                    );
                }
            }
        }

        // 3. Java must exist as a file we could execute.
        if !java_executable.exists() {
            report.push(Finding::error(
                "java.not_found",
                java_executable.to_path_buf(),
                format!(
                    "java runtime not found at {}",
                    java_executable.display()
                ),
            ), None);
        }

        Ok(report)
    }
}

/// Execute a repair plan. Only performs actions the validator proposed;
/// downloads go through the verified engine (hash-checked, atomic).
pub fn apply_repairs(
    agent: &ureq::Agent,
    actions: &[RepairAction],
    cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<usize> {
    let opts = crate::download::DownloadOptions {
        retries: 2,
        cancel: std::sync::Arc::clone(cancel),
    };
    let mut done = 0usize;
    for action in actions {
        match action {
            RepairAction::CreateDirectory(path) => {
                std::fs::create_dir_all(path).map_err(|e| {
                    Error::with_source(
                        ErrorCode::IoFailure,
                        format!("cannot create {}", path.display()),
                        e,
                    )
                })?;
                done += 1;
            }
            RepairAction::Redownload {
                url,
                dest,
                sha1,
            } => {
                let mut noop = |_| {};
                crate::download::download_verified(agent, url, dest, sha1.as_deref(), &opts, &mut noop)?;
                done += 1;
            }
        }
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_support::unique_temp_dir;
    use std::fs;

    #[test]
    fn clean_instance_validates_with_no_findings() {
        let dir = unique_temp_dir("val-clean");
        fs::create_dir_all(&dir).unwrap();
        let java = if cfg!(windows) { PathBuf::from("java") } else { PathBuf::from("/bin/sh") };
        let report = InstanceValidator::validate(&dir, &[], &java).unwrap();
        assert!(report.is_ok());
        assert!(report.findings.is_empty());
    }

    #[test]
    fn missing_and_corrupt_artifacts_produce_redownload_actions() {
        let dir = unique_temp_dir("val-artifacts");
        fs::create_dir_all(&dir).unwrap();
        let good = dir.join("good.jar");
        fs::write(&good, b"data").unwrap();
        let bad = dir.join("bad.jar");
        fs::write(&bad, b"tampered").unwrap();
        let missing = dir.join("missing.jar");

        let artifacts = vec![
            ExpectedArtifact {
                dest: good.clone(),
                url: "https://example/good.jar".into(),
                sha1: Some(crate::download::sha1_hex(b"data")),
            },
            ExpectedArtifact {
                dest: bad.clone(),
                url: "https://example/bad.jar".into(),
                sha1: Some(String::from("0".repeat(40))),
            },
            ExpectedArtifact {
                dest: missing.clone(),
                url: "https://example/missing.jar".into(),
                sha1: None,
            },
        ];

        let java = if cfg!(windows) { PathBuf::from("java") } else { PathBuf::from("/usr/bin/env") };
        let report = InstanceValidator::validate(&dir, &artifacts, &java).unwrap();
        assert!(!report.is_ok(), "corrupt + missing are errors");
        assert_eq!(report.repairs.len(), 2, "one redownload per broken artifact");
        match &report.repairs[0] {
            RepairAction::Redownload { dest, .. } => assert_eq!(dest, &bad),
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn missing_java_is_reported_without_a_repair_action() {
        let dir = unique_temp_dir("val-java");
        fs::create_dir_all(&dir).unwrap();
        let report =
            InstanceValidator::validate(&dir, &[], &PathBuf::from("/definitely/not/java")).unwrap();
        assert!(!report.is_ok());
        assert_eq!(report.findings[0].code, "java.not_found");
        assert!(report.repairs.is_empty(), "we cannot auto-install java here");
    }
}
