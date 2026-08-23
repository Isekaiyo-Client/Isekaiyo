//! LaunchPlan construction. The planner is the ONLY place Minecraft arguments
//! are built; the UI hands it an instance + identity + options and receives a
//! ready-to-run plan.
//!
//! Handles both modern (`arguments.game`/`arguments.jvm` with rules) and
//! legacy (`minecraftArguments` string) metadata shapes.

use ikk_core::error::{Error, ErrorCode, Result};
use std::path::PathBuf;

use crate::account::LaunchIdentity;
use crate::metadata::VersionMetadata;
use crate::rules;

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// Instance game directory (`--gameDir`, cwd for the process).
    pub game_dir: PathBuf,
    /// Shared assets root (`<cache>/assets`).
    pub assets_dir: PathBuf,
    /// Extracted natives directory for this run.
    pub natives_dir: PathBuf,
    /// Classpath jars in order: client last, per vanilla convention the
    /// classpath string joins with the platform separator.
    pub classpath: Vec<PathBuf>,
    /// Downloaded log4j config path, when the version ships one.
    pub logging_config: Option<PathBuf>,
    /// `-Xmx` heap in MiB, when configured.
    pub memory_mb: Option<u32>,
    /// Extra JVM args from profile settings — appended after ours.
    pub jvm_extra: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub java_executable: PathBuf,
    pub jvm_args: Vec<String>,
    pub main_class: String,
    pub game_args: Vec<String>,
}

impl LaunchPlan {
    /// Flat argv for logging/debugging. Never contains secrets other than the
    /// access token slot, which callers must not log — see process.rs which
    /// writes only stdout/stderr of the game, not argv.
    pub fn argv(&self) -> Vec<String> {
        let mut argv = vec![self.java_executable.to_string_lossy().into_owned()];
        argv.extend(self.jvm_args.iter().cloned());
        argv.push(self.main_class.clone());
        argv.extend(self.game_args.iter().cloned());
        argv
    }

    /// Dry-run / developer-inspection view (spec §40–§41): the exact argv that
    /// WOULD run, with every supplied secret replaced by `[redacted]`. This is
    /// the only argv form allowed to cross into logs or the UI.
    pub fn argv_redacted(&self, secrets: &[String]) -> Vec<String> {
        self.argv()
            .into_iter()
            .map(|arg| {
                let mut out = arg;
                for secret in secrets {
                    if !secret.is_empty() {
                        out = out.replace(secret.as_str(), "[redacted]");
                    }
                }
                out
            })
            .collect()
    }
}

/// Replace every occurrence of any secret in `text` with `[redacted]`
/// (spec §59). Used for crash reports and debug output.
pub fn redact_secrets(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_owned();
    for secret in secrets {
        if !secret.is_empty() {
            out = out.replace(secret.as_str(), "[redacted]");
        }
    }
    out
}

fn classpath_separator() -> &'static str {
    if std::env::consts::OS == "windows" {
        ";"
    } else {
        ":"
    }
}

/// Substitution table shared by modern placeholders and legacy strings.
fn substitute(
    template: &str,
    identity: &LaunchIdentity,
    opts: &LaunchOptions,
    version_id: &str,
    asset_index_id: &str,
) -> String {
    let cp = opts
        .classpath
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(classpath_separator());
    // `user_properties` is a legacy empty-JSON placeholder.
    let map = [
        ("${auth_player_name}", identity.username.clone()),
        ("${auth_uuid}", identity.uuid.clone()),
        ("${auth_access_token}", identity.access_token.clone()),
        (
            "${auth_session}",
            format!("token:{}:{}", identity.access_token, identity.uuid),
        ),
        ("${auth_xuid}", String::new()),
        ("${clientid}", String::new()),
        (
            "${user_type}",
            match identity.kind {
                crate::account::IdentityKind::Microsoft => "msa".to_owned(),
                crate::account::IdentityKind::OfflineProfile => "legacy".to_owned(),
            },
        ),
        ("${user_properties}", "{}".to_owned()),
        ("${version_name}", version_id.to_string()),
        ("${version_type}", String::new()),
        (
            "${game_directory}",
            opts.game_dir.to_string_lossy().into_owned(),
        ),
        (
            "${assets_root}",
            opts.assets_dir.to_string_lossy().into_owned(),
        ),
        (
            "${game_assets}",
            // Legacy virtual assets dir; pointing it at the index-specific
            // folder keeps old versions functional without duplicating data.
            opts.assets_dir
                .join("virtual")
                .join(asset_index_id)
                .to_string_lossy()
                .into_owned(),
        ),
        ("${assets_index_name}", asset_index_id.to_owned()),
        (
            "${natives_directory}",
            opts.natives_dir.to_string_lossy().into_owned(),
        ),
        ("${launcher_name}", "Isekaiyo".to_owned()),
        ("${launcher_version}", env!("CARGO_PKG_VERSION").to_owned()),
        ("${classpath}", cp),
        ("${classpath_separator}", classpath_separator().to_owned()),
    ];
    let mut out = template.to_owned();
    for (key, value) in map {
        out = out.replace(key, &value);
    }
    out
}

pub fn build_plan(
    meta: &VersionMetadata,
    identity: &LaunchIdentity,
    java: &crate::java::JavaRuntime,
    opts: &LaunchOptions,
) -> Result<LaunchPlan> {
    if meta.main_class.trim().is_empty() {
        return Err(Error::new(
            ErrorCode::MetadataInvalid,
            "version metadata has an empty mainClass",
        ));
    }

    let feature = |_name: &str| false; // demo/user-feature gates off for normal launches
    let ctx = rules::EvalContext {
        os_name: rules::os_name(),
        arch: std::env::consts::ARCH,
        feature: &feature,
    };

    let args = meta.arguments.as_ref();
    let legacy = meta.minecraft_arguments.as_deref();

    if args.is_none() && legacy.is_none() {
        return Err(Error::new(
            ErrorCode::MetadataInvalid,
            "version metadata has neither arguments nor minecraftArguments",
        ));
    }

    let asset_index_id: String = meta
        .assets
        .clone()
        .or_else(|| meta.asset_index.as_ref().map(|a| a.id.clone()))
        .unwrap_or_else(|| meta.id.clone());

    // --- JVM arguments ------------------------------------------------------
    let mut jvm_args: Vec<String> = Vec::new();

    // Memory first so later flags can override if a user insists.
    if let Some(mb) = opts.memory_mb {
        if !(512..=32768).contains(&mb) {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                format!("memory allocation {mb} MiB is outside the supported 512–32768 range"),
            ));
        }
        jvm_args.push(format!("-Xmx{mb}M"));
    }

    // log4j configuration when the version provides one.
    if let Some(config_path) = &opts.logging_config {
        if let Some(template) = meta.logging_argument() {
            jvm_args.push(template.replace("${path}", &config_path.to_string_lossy()));
        }
    }

    match args.map(|a| &a.jvm) {
        Some(jvm_entries) => {
            for entry in jvm_entries {
                // `expand` already applies rule gating: conditional entries
                // yield None when their rules reject this platform.
                if let Some(values) = entry.expand(&ctx) {
                    for value in values {
                        jvm_args.push(substitute(
                            &value,
                            identity,
                            opts,
                            &meta.id,
                            &asset_index_id,
                        ));
                    }
                }
            }
        }
        None => {
            // Legacy defaults (pre-1.13 shape has no jvm list).
            jvm_args.push(
                "-Djava.library.path=${natives_directory}"
                    .replace("${natives_directory}", &opts.natives_dir.to_string_lossy()),
            );
            let cp = opts
                .classpath
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(classpath_separator());
            jvm_args.push("-cp".to_owned());
            jvm_args.push(cp);
        }
    }

    jvm_args.extend(opts.jvm_extra.iter().cloned());

    // --- game arguments -----------------------------------------------------
    let mut game_args: Vec<String> = Vec::new();
    match args.map(|a| &a.game) {
        Some(game_entries) => {
            for entry in game_entries {
                if let Some(values) = entry.expand(&ctx) {
                    for value in values {
                        game_args.push(substitute(
                            &value,
                            identity,
                            opts,
                            &meta.id,
                            &asset_index_id,
                        ));
                    }
                }
            }
        }
        None => {
            let raw = legacy.unwrap_or_default();
            for token in raw.split_whitespace() {
                game_args.push(substitute(token, identity, opts, &meta.id, &asset_index_id));
            }
        }
    }

    Ok(LaunchPlan {
        java_executable: java.executable.clone(),
        jvm_args,
        main_class: meta.main_class.clone(),
        game_args,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_fixtures;

    fn fixture_meta() -> VersionMetadata {
        VersionMetadata::parse(test_fixtures::VERSION_METADATA_JSON).unwrap()
    }

    fn opts(natives: bool) -> LaunchOptions {
        LaunchOptions {
            game_dir: PathBuf::from("/inst/game"),
            assets_dir: PathBuf::from("/data/assets"),
            natives_dir: PathBuf::from(if natives {
                "/inst/game/natives"
            } else {
                "/inst/game/natives-win"
            }),
            classpath: vec![
                PathBuf::from("/data/libraries/a.jar"),
                PathBuf::from("/data/versions/1.20.4/1.20.4.jar"),
            ],
            logging_config: Some(PathBuf::from("/data/log-configs/client-1.12.xml")),
            memory_mb: Some(2048),
            jvm_extra: vec!["-XX:+UseG1GC".to_owned()],
        }
    }

    #[test]
    fn builds_modern_plan_with_substitutions() {
        let meta = fixture_meta();
        let identity = LaunchIdentity::offline("Steve").unwrap();
        let java = crate::java::JavaRuntime {
            executable: PathBuf::from("/usr/bin/java"),
            major_version: 17,
            home: None,
        };
        let plan = build_plan(&meta, &identity, &java, &opts(false)).unwrap();

        assert_eq!(plan.main_class, "net.minecraft.client.main.Main");
        assert!(plan.jvm_args.contains(&"-Xmx2048M".to_owned()));
        assert!(plan
            .jvm_args
            .contains(&"-Dlog4j.configurationFile=/data/log-configs/client-1.12.xml".to_owned()));
        assert!(plan.jvm_args.contains(&"-XX:+UseG1GC".to_owned()));
        // Platform-gated jvm arg: -XstartOnFirstThread only on osx.
        assert_eq!(
            plan.jvm_args.iter().any(|a| a == "-XstartOnFirstThread"),
            cfg!(target_os = "macos")
        );

        // Game args carry identity + paths.
        let username_at = plan.game_args.iter().position(|a| a == "Steve");
        assert!(username_at.is_some_and(|i| i > 0 && plan.game_args[i - 1] == "--username"));
        assert!(plan.game_args.contains(&"/inst/game".to_owned()));
        assert!(
            plan.game_args.contains(&"17".to_owned()),
            "asset index id passed"
        );

        // Offline token is the honest placeholder, never fabricated auth.
        let token_at = plan.game_args.iter().position(|a| a == "0");
        assert!(token_at.is_some_and(|i| plan.game_args[i - 1] == "--accessToken"));
    }

    #[test]
    fn dry_run_argv_redacts_every_secret_occurrence() {
        let meta = fixture_meta();
        let identity = LaunchIdentity::offline("Steve").unwrap();
        let java = crate::java::JavaRuntime {
            executable: PathBuf::from("/usr/bin/java"),
            major_version: 17,
            home: None,
        };
        let plan = build_plan(&meta, &identity, &java, &opts(false)).unwrap();

        // Offline placeholder token is "0"; use a realistic-looking secret.
        let secrets = vec!["s3cret-token-abc".to_owned(), "Steve".to_owned()];
        let argv = plan.argv_redacted(&secrets);
        assert!(!argv.iter().any(|a| a.contains("s3cret-token-abc")));
        assert!(argv.iter().any(|a| a.contains("[redacted]")));
        // The raw form still exists for the actual spawn (not the debug view).
        assert!(plan.argv().iter().any(|a| a == "--username"));
    }

    #[test]
    fn redact_secrets_is_total_and_noop_safe() {
        assert_eq!(redact_secrets("token=xyz end", &["xyz".to_owned()]), "token=[redacted] end");
        assert_eq!(redact_secrets("nothing here", &[]), "nothing here");
        assert_eq!(redact_secrets("empty secret", &[String::new()]), "empty secret");
    }

    #[test]
    fn memory_bounds_are_enforced() {
        let meta = fixture_meta();
        let identity = LaunchIdentity::offline("Steve").unwrap();
        let java = crate::java::JavaRuntime {
            executable: PathBuf::from("java"),
            major_version: 17,
            home: None,
        };
        let mut o = opts(true);
        o.memory_mb = Some(100);
        assert_eq!(
            build_plan(&meta, &identity, &java, &o).unwrap_err().code(),
            ErrorCode::InstanceInvalid
        );
        o.memory_mb = Some(999_999);
        assert!(build_plan(&meta, &identity, &java, &o).is_err());
    }

    #[test]
    fn legacy_metadata_builds_classic_plan() {
        let json = r#"{
            "id": "b1.7.3",
            "mainClass": "net.minecraft.client.Minecraft",
            "minecraftArguments": "${auth_player_name} ${auth_session} --gameDir ${game_directory}",
            "assets": "legacy",
            "assetIndex": { "id": "legacy", "url": "https://x/i.json", "sha1": "abc", "size": 1 },
            "downloads": { "client": { "url": "https://x/c.jar", "sha1": "def", "size": 1 } }
        }"#;
        let meta = VersionMetadata::parse(json).unwrap();
        let identity = LaunchIdentity::offline("Alex").unwrap();
        let java = crate::java::JavaRuntime {
            executable: PathBuf::from("java"),
            major_version: 8,
            home: None,
        };
        let plan = build_plan(&meta, &identity, &java, &opts(true)).unwrap();
        assert!(plan.jvm_args.windows(2).any(|w| w[0] == "-cp"));
        assert!(plan.game_args.contains(&"Alex".to_owned()));
        assert!(plan
            .game_args
            .iter()
            .any(|a| a.starts_with("token:0:") && a.ends_with(identity.uuid.as_str())));
    }
}
