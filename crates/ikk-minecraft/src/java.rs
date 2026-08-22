//! Java runtime discovery, version parsing, and Minecraft compatibility.
//!
//! We never "run java and hope": every candidate executable is probed with
//! `-XshowSettings:properties -version` and its real `java.version` property
//! is parsed. Parsing and compatibility are offline-testable; discovery is
//! plain filesystem + process work.

use ikk_core::error::{Error, ErrorCode, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A discovered Java runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaRuntime {
    /// Executable path used to launch (`…/bin/java`).
    pub executable: PathBuf,
    /// Parsed major version, e.g. 17 for `17.0.9`, 8 for `1.8.0_392`.
    pub major_version: u32,
    /// `java.home` reported by the runtime itself.
    pub home: Option<PathBuf>,
}

/// Parse a `java.version` property or `java -version` banner into a major
/// version. Handles the legacy `1.8.0_392` scheme and modern `17.0.9`/`21`.
pub fn parse_java_version(version_string: &str) -> Option<u32> {
    let s = version_string.trim();
    let s = s.strip_prefix('"').unwrap_or(s);
    let s = s.trim_end_matches('"');
    let mut parts = s.split(['.', '_']);
    let first = parts.next()?;
    let first_num: u32 = first.parse().ok()?;
    if first_num == 1 {
        // Legacy: 1.<major>.0_<patch>
        let second = parts.next()?;
        return second.parse().ok();
    }
    Some(first_num)
}

/// Extract `java.version` and `java.home` from `-XshowSettings:properties`
/// stderr output.
pub fn parse_show_settings(stderr: &str) -> Option<(u32, Option<PathBuf>)> {
    let mut version = None;
    let mut home = None;
    for line in stderr.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("java.version =") {
            version = parse_java_version(rest);
        } else if let Some(rest) = line.strip_prefix("java.home =") {
            home = Some(PathBuf::from(rest.trim()));
        }
    }
    version.map(|v| (v, home))
}

/// Probe an executable: run it, parse the real version. Never trusts the path
/// name (a file called `java21` may be Java 8).
pub fn probe(executable: &Path) -> Result<JavaRuntime> {
    let output = Command::new(executable)
        .arg("-XshowSettings:properties")
        .arg("-version")
        .output()
        .map_err(|e| {
            Error::with_source(
                ErrorCode::JavaNotFound,
                format!("cannot execute {}", executable.display()),
                e,
            )
        })?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let (major, home) = parse_show_settings(&stderr).ok_or_else(|| {
        Error::new(
            ErrorCode::JavaNotFound,
            format!(
                "{} did not report a parsable java.version (is it a JRE/JDK launcher?)",
                executable.display()
            ),
        )
    })?;
    Ok(JavaRuntime {
        executable: executable.to_path_buf(),
        major_version: major,
        home,
    })
}

/// Candidate executables to probe, per platform: PATH's `java` plus the usual
/// install locations. Order is irrelevant — selection picks the best.
pub fn discovery_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("java")];
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        candidates.push(PathBuf::from(home).join("bin").join(java_exe()));
    }
    match std::env::consts::OS {
        "windows" => {
            let pf = std::env::var("ProgramFiles").map(PathBuf::from);
            let pf86 = std::env::var("ProgramFiles(x86)").map(PathBuf::from);
            for base in [pf, pf86].into_iter().flatten() {
                for vendor in [
                    "Eclipse Adoptium",
                    "AdoptOpenJDK",
                    "Microsoft",
                    "Amazon Corretto",
                    "Zulu",
                    "Java",
                ] {
                    if let Ok(entries) = std::fs::read_dir(base.join(vendor)) {
                        for entry in entries.flatten() {
                            candidates.push(entry.path().join("bin").join(java_exe()));
                        }
                    }
                }
            }
        }
        "macos" => {
            if let Ok(entries) = std::fs::read_dir(
                std::env::home_dir()
                    .unwrap_or_default()
                    .join("Library/Java/JavaVirtualMachines"),
            ) {
                for entry in entries.flatten() {
                    candidates.push(entry.path().join("Contents/Home/bin").join(java_exe()));
                }
            }
        }
        _ => {
            // linux & friends
            for base in ["/usr/lib/jvm", "/opt/java"] {
                if let Ok(entries) = std::fs::read_dir(base) {
                    for entry in entries.flatten() {
                        candidates.push(entry.path().join("bin").join(java_exe()));
                    }
                }
            }
        }
    }
    candidates
}

fn java_exe() -> &'static str {
    if std::env::consts::OS == "windows" {
        "java.exe"
    } else {
        "java"
    }
}

/// Probe every candidate that exists; failures are skipped (a broken install
/// must not hide a working one).
pub fn discover() -> Vec<JavaRuntime> {
    let mut found = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for candidate in discovery_candidates() {
        let key = candidate.to_string_lossy().into_owned();
        if candidate.exists() && seen.insert(key) {
            if let Ok(runtime) = probe(&candidate) {
                found.push(runtime);
            }
        }
    }
    found.sort_by_key(|r| std::cmp::Reverse(r.major_version));
    found
}

/// Minecraft requires AT LEAST the stated major version.
pub fn is_compatible(found_major: u32, required_major: u32) -> bool {
    found_major >= required_major
}

/// Pick the newest compatible runtime from a discovery result.
pub fn select(runtimes: &[JavaRuntime], required_major: u32) -> Result<&JavaRuntime> {
    runtimes
        .iter()
        .find(|r| is_compatible(r.major_version, required_major))
        .ok_or_else(|| {
            Error::new(
                ErrorCode::JavaNotSuitable,
                format!(
                    "no Java runtime found providing version ≥ {required_major} \
                     (found: {})",
                    runtimes
                        .iter()
                        .map(|r| r.major_version.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn parses_legacy_and_modern_versions() {
        assert_eq!(parse_java_version("1.8.0_392"), Some(8));
        assert_eq!(parse_java_version("17.0.9"), Some(17));
        assert_eq!(parse_java_version("21"), Some(21));
        assert_eq!(parse_java_version("11.0.21+9"), Some(11));
        assert_eq!(parse_java_version("not-a-version"), None);
    }

    #[test]
    fn parses_show_settings_output() {
        let sample = r#"
            openjdk version "17.0.9" 2023-10-17
            Property settings:
                java.version = 17.0.9
                java.home = /usr/lib/jvm/java-17-openjdk
        "#;
        let (major, home) = parse_show_settings(sample).unwrap();
        assert_eq!(major, 17);
        assert_eq!(home, Some(PathBuf::from("/usr/lib/jvm/java-17-openjdk")));
    }

    #[test]
    fn compatibility_is_a_floor() {
        assert!(is_compatible(17, 17));
        assert!(is_compatible(21, 17));
        assert!(!is_compatible(8, 17));
        assert!(!is_compatible(11, 17));
    }

    #[test]
    fn select_prefers_newest_compatible_and_reports_gaps() {
        let runtimes = vec![
            JavaRuntime {
                executable: PathBuf::from("j8"),
                major_version: 8,
                home: None,
            },
            JavaRuntime {
                executable: PathBuf::from("j21"),
                major_version: 21,
                home: None,
            },
            JavaRuntime {
                executable: PathBuf::from("j17"),
                major_version: 17,
                home: None,
            },
        ];
        assert_eq!(select(&runtimes, 17).unwrap().major_version, 21);

        let old_only = &runtimes[..1];
        let err = select(old_only, 17).unwrap_err();
        assert_eq!(err.code(), ErrorCode::JavaNotSuitable);
        assert!(err.to_string().contains("≥ 17"));
    }
}
