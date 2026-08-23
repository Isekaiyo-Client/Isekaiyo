//! Isekaiyo launcher application shell.
//!
//! Responsibilities: run the startup sequence
//!   logging → platform paths → configuration → instance store → UI
//! and expose the ONLY surface the frontend may talk to: these typed commands.
//! All business logic lives in `ikk-core` / `ikk-minecraft`; this file is
//! composition glue — every heavy operation delegates downward.

use ikk_api_types::{
    AppConfig, CommandError, ConfigLoadInfo, Instance, InstanceListing, LoaderKindInput, SystemInfo,
};
use ikk_core::config::{ConfigStore, LoadSource};
use ikk_core::ids::InstanceId;
use ikk_core::store::InstanceStore;
use ikk_minecraft::download::{self, DownloadOptions};
use ikk_minecraft::java::{self, JavaRuntime};
use ikk_minecraft::loaders::{self, LoaderId, ResolvedLoader};
use ikk_minecraft::manifest::{ManifestCache, MANIFEST_FRESHNESS};
use ikk_minecraft::metadata::VersionMetadata;
use ikk_minecraft::natives::extract_natives;
use ikk_minecraft::planner::{build_plan, LaunchOptions};
use ikk_minecraft::process::{self, GameExit, ManagedProcess};
use ikk_minecraft::resolve::{plan_assets, plan_install, ArtifactKind};
use ikk_minecraft::state::{LaunchPhase, PhaseTracker};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use tauri::State;
use tracing::{error, info, warn};

/// Everything the application owns at runtime. The single mutex guards fast
/// state only — long network/file operations snapshot what they need, drop
/// the lock, and re-lock briefly for progress updates so the UI never stalls.
struct AppData {
    config_store: ConfigStore,
    instance_store: InstanceStore,
    /// Frozen snapshot of how configuration came up, for the UI to display.
    startup_info: ConfigLoadInfo,
    /// Root for all derived storage (cache/, profiles/, instances/<id>/game).
    data_dir: PathBuf,
    phase: PhaseTracker,
    /// Progress of the current install, polled by the UI.
    progress: InstallProgress,
    running: Option<ManagedProcess>,
    agent: ureq::Agent,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AppData {
    fn cancel_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.cancel.clone()
    }
}

#[derive(Default, Clone)]
pub struct InstallProgress {
    pub total_files: u32,
    pub done_files: u32,
    pub current_label: String,
}

fn lock(data: &State<'_, Mutex<AppData>>) -> Result<MutexGuard<'_, AppData>, CommandError> {
    data.lock()
        .map_err(|_| CommandError::internal("application state lock poisoned"))
}

fn loader_id_of(kind: &ikk_core::instance::LoaderKind) -> LoaderId {
    use ikk_core::instance::LoaderKind as K;
    match kind {
        K::Vanilla => LoaderId::Vanilla,
        K::Fabric => LoaderId::Fabric,
        K::Forge => LoaderId::Forge,
        K::NeoForge => LoaderId::NeoForge,
        K::Quilt => LoaderId::Quilt,
    }
}

// ---------------------------------------------------------------------------
// Configuration & instance commands (unchanged contracts)
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_system_info() -> SystemInfo {
    SystemInfo {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        target: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_owned(),
    }
}

#[tauri::command]
fn get_startup_info(data: State<'_, Mutex<AppData>>) -> Result<ConfigLoadInfo, CommandError> {
    Ok(lock(&data)?.startup_info.clone())
}

#[tauri::command]
fn get_config(data: State<'_, Mutex<AppData>>) -> Result<AppConfig, CommandError> {
    Ok(lock(&data)?.config_store.load().config)
}

#[tauri::command]
fn set_config(
    data: State<'_, Mutex<AppData>>,
    config: AppConfig,
) -> Result<AppConfig, CommandError> {
    let app = lock(&data)?;
    app.config_store.save(&config)?;
    info!("configuration updated");
    Ok(app.config_store.load().config)
}

#[tauri::command]
fn list_instances(data: State<'_, Mutex<AppData>>) -> Result<InstanceListing, CommandError> {
    Ok(lock(&data)?.instance_store.list())
}

#[tauri::command]
fn create_instance(
    data: State<'_, Mutex<AppData>>,
    name: String,
    minecraft_version: String,
    loader: Option<LoaderKindInput>,
) -> Result<Instance, CommandError> {
    let app = lock(&data)?;
    let instance = app.instance_store.create(name, minecraft_version, loader)?;
    info!(id = %instance.id, "instance created");
    Ok(instance)
}

#[tauri::command]
fn update_instance(
    data: State<'_, Mutex<AppData>>,
    instance: Instance,
) -> Result<Instance, CommandError> {
    let app = lock(&data)?;
    app.instance_store.update(&instance)?;
    info!(id = %instance.id, "instance updated");
    Ok(instance)
}

#[tauri::command]
fn delete_instance(data: State<'_, Mutex<AppData>>, id: String) -> Result<bool, CommandError> {
    let app = lock(&data)?;
    let deleted = app.instance_store.delete(&InstanceId::new(id))?;
    info!(%deleted, "instance delete requested");
    Ok(deleted)
}

// ---------------------------------------------------------------------------
// Version metadata service (Phase 3)
// ---------------------------------------------------------------------------

/// Cache-first version manifest: fresh cache answers instantly; stale or
/// missing cache falls back to the network; corrupt cache is discarded and
/// refetched rather than trusted.
#[tauri::command]
fn list_versions(
    data: State<'_, Mutex<AppData>>,
    force_refresh: bool,
) -> Result<ikk_api_types::VersionListDto, CommandError> {
    let (cache_path, agent) = {
        let app = lock(&data)?;
        (app.data_dir.join("cache"), app.agent.clone())
    };
    let cache = ManifestCache::new(cache_path);

    if !force_refresh {
        if let Ok(Some(cached)) = cache.load() {
            if cache.is_fresh(&cached, MANIFEST_FRESHNESS) {
                return Ok(ikk_api_types::VersionListDto {
                    source: "cache".into(),
                    entries: cached
                        .manifest
                        .versions
                        .iter()
                        .map(|e| ikk_api_types::VersionEntryDto {
                            id: e.id.clone(),
                            kind: e.kind.clone(),
                        })
                        .collect(),
                });
            }
        }
    }

    // Network refresh (or first run). On failure, fall back to ANY valid
    // cache and label it stale so the UI can say so honestly.
    match crate_fetch_manifest(&agent, &cache) {
        Ok(manifest) => Ok(ikk_api_types::VersionListDto {
            source: "network".into(),
            entries: manifest_to_dto(&manifest),
        }),
        Err(net_err) => {
            warn!(error = %net_err, "manifest refresh failed; trying cache fallback");
            match cache.load() {
                Ok(Some(cached)) => {
                    // Corrupt cache data is never trusted: clear it so the
                    // next attempt starts clean either way.
                    if cached.manifest.versions.is_empty() {
                        cache.clear();
                    }
                    Ok(ikk_api_types::VersionListDto {
                        source: "stale-cache".into(),
                        entries: manifest_to_dto(&cached.manifest),
                    })
                }
                _ => Err(CommandError::from(net_err)),
            }
        }
    }
}

fn manifest_to_dto(
    manifest: &ikk_minecraft::manifest::VersionManifest,
) -> Vec<ikk_api_types::VersionEntryDto> {
    manifest
        .versions
        .iter()
        .map(|e| ikk_api_types::VersionEntryDto {
            id: e.id.clone(),
            kind: e.kind.clone(),
        })
        .collect()
}

fn crate_fetch_manifest(
    agent: &ureq::Agent,
    cache: &ManifestCache,
) -> ikk_core::Result<ikk_minecraft::manifest::VersionManifest> {
    const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let text = ikk_minecraft::fetch_text(agent, MANIFEST_URL)?;
    let manifest = ikk_minecraft::manifest::VersionManifest::parse(&text)?;
    cache.save(&manifest)?;
    Ok(manifest)
}

/// Available loader versions for a Minecraft version, straight from the
/// loader's meta service (Fabric/Quilt). Forge reports not-implemented.
#[tauri::command]
fn list_loader_versions(
    data: State<'_, Mutex<AppData>>,
    kind: String,
    mc_version: String,
) -> Result<Vec<ikk_api_types::LoaderVersionDto>, CommandError> {
    let id = parse_loader_id(&kind)?;
    let provider = loaders::provider_for(id)?;
    let agent = lock(&data)?.agent.clone();
    let versions = provider.list_versions(&agent, &mc_version)?;
    Ok(versions
        .into_iter()
        .map(|v| ikk_api_types::LoaderVersionDto {
            version: v.version,
            stable: v.stable,
        })
        .collect())
}

fn parse_loader_id(kind: &str) -> Result<LoaderId, CommandError> {
    match kind {
        "vanilla" => Ok(LoaderId::Vanilla),
        "fabric" => Ok(LoaderId::Fabric),
        "quilt" => Ok(LoaderId::Quilt),
        "forge" => Ok(LoaderId::Forge),
        "neoforge" => Ok(LoaderId::NeoForge),
        other => Err(CommandError::internal(format!(
            "unknown loader kind {other:?}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Installation pipeline (Phase 3/5): metadata → resolve → download → verify
// ---------------------------------------------------------------------------

#[tauri::command]
fn install_instance(
    data: State<'_, Mutex<AppData>>,
    id: String,
) -> Result<ikk_api_types::InstallReportDto, CommandError> {
    transition(&data, LaunchPhase::Preparing)?;

    // --- snapshot everything the long phase needs, then release the lock ---
    let (instance, cache_root, agent) = {
        let app = lock(&data)?;
        (
            find_instance(&app, &id)?,
            app.data_dir.join("cache"),
            app.agent.clone(),
        )
    };

    transition(&data, LaunchPhase::ResolvingMetadata)?;
    let resolved = resolve_effective_metadata(&agent, &cache_root, &instance)?;

    // Persist the effective document for launch-time reuse.
    let profiles_dir = cache_root.join("profiles");
    std::fs::create_dir_all(&profiles_dir).map_err(io_err("cannot create profiles dir"))?;
    let profile_path = profiles_dir.join(format!("{id}.json"));
    std::fs::write(
        &profile_path,
        serde_json::to_vec_pretty(&resolved.effective_metadata).map_err(ser_err)?,
    )
    .map_err(io_err("cannot write profile"))?;

    // Stage 1 artifacts.
    transition(&data, LaunchPhase::Downloading)?;
    let mut plan = plan_install(&resolved.effective_metadata, &cache_root)?;

    let mut report = ikk_api_types::InstallReportDto::default();
    run_plan(&data, &plan.artifacts, &mut report)?;

    // Stage 2: asset index → asset objects.
    transition(&data, LaunchPhase::Verifying)?;
    if let Some(index_ref) = resolved.effective_metadata.asset_index() {
        let index_path = cache_root
            .join("assets")
            .join("indexes")
            .join(format!("{}.json", index_ref.id));
        let raw = std::fs::read_to_string(&index_path)
            .map_err(io_err("asset index missing after install"))?;
        let index = ikk_minecraft::assets::AssetIndex::parse(&raw)?;
        let assets = plan_assets(&index, &cache_root);
        report.total_files += assets.len() as u32;
        run_plan(&data, &assets, &mut report)?;
    }

    // Natives extraction (per-instance, zip-slip guarded inside the engine).
    let game_dir = game_dir_of(&lock(&data)?.data_dir, &id);
    let natives_dir = natives_dir_of(&game_dir);
    let native_jars: Vec<PathBuf> = plan
        .artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::NativeJar)
        .map(|a| a.dest.clone())
        .collect();
    if !native_jars.is_empty() {
        let refs: Vec<&std::path::Path> = native_jars.iter().map(|p| p.as_path()).collect();
        let count = extract_natives(
            &refs,
            &natives_dir,
            &["META-INF/".to_owned(), ".git/".to_owned()],
        )
        .map_err(CommandError::from)?;
        info!(count, "natives extracted");
    }

    transition(&data, LaunchPhase::Idle)?;
    info!(id = %id, downloaded = report.downloaded, skipped = report.skipped, "install finished");
    Ok(report)
}

/// Fetch (and overlay, for modded) the metadata an instance launches with.
fn resolve_effective_metadata(
    agent: &ureq::Agent,
    cache_root: &std::path::Path,
    instance: &Instance,
) -> Result<ResolvedLoader, CommandError> {
    let cache = ManifestCache::new(cache_root);
    let entry = cache
        .load()
        .ok()
        .flatten()
        .and_then(|c| {
            c.manifest
                .find(instance.minecraft_version.as_str())
                .cloned()
        })
        .ok_or_else(|| {
            CommandError::from(ikk_core::Error::new(
                ikk_core::ErrorCode::MetadataInvalid,
                format!(
                    "Minecraft {} is unknown — refresh the version list",
                    instance.minecraft_version
                ),
            ))
        })?;
    let vanilla_json = ikk_minecraft::fetch_text(agent, &entry.url)?;
    let provider = loaders::provider_for(loader_id_of(&instance.loader.kind))?;
    let loader_version = instance.loader.version.as_deref().unwrap_or("");
    provider.resolve(
        agent,
        instance.minecraft_version.as_str(),
        loader_version,
        &vanilla_json,
    )
}

fn run_plan(
    data: &State<'_, Mutex<AppData>>,
    artifacts: &[ikk_minecraft::resolve::PlannedArtifact],
    report: &mut ikk_api_types::InstallReportDto,
) -> Result<(), CommandError> {
    let (agent, opts) = {
        let app = lock(data)?;
        let opts = DownloadOptions {
            retries: 2,
            cancel: app.cancel_flag(),
        };
        (app.agent.clone(), opts)
    };

    {
        let mut app = lock(data)?;
        app.progress.total_files += artifacts.len() as u32;
    }

    for artifact in artifacts {
        {
            let mut app = lock(data)?;
            app.progress.current_label = artifact.label.clone();
        }
        let result = download::download_verified(
            &agent,
            &artifact.url,
            &artifact.dest,
            artifact.sha1.as_deref(),
            &opts,
            &mut |_| {},
        );
        let mut app = lock(data)?;
        app.progress.done_files += 1;
        match result {
            Ok(download::FileStatus::Skipped) => report.skipped += 1,
            Ok(_) => report.downloaded += 1,
            Err(e) => {
                error!(artifact = %artifact.label, error = %e, "download failed");
                report.failed.push(format!("{}: {}", artifact.label, e));
            }
        }
    }
    Ok(())
}

fn find_instance(app: &AppData, id: &str) -> Result<Instance, CommandError> {
    app.instance_store
        .list()
        .instances
        .into_iter()
        .find(|i| i.id.as_str() == id)
        .ok_or_else(|| {
            CommandError::from(ikk_core::Error::new(
                ikk_core::ErrorCode::InstanceNotFound,
                format!("no instance with id {id}"),
            ))
        })
}

fn game_dir_of(data_dir: &std::path::Path, instance_id: &str) -> PathBuf {
    data_dir.join("instances").join(instance_id).join("game")
}

fn natives_dir_of(game_dir: &std::path::Path) -> PathBuf {
    game_dir.join("natives")
}

// ---------------------------------------------------------------------------
// Launch pipeline (Phase 3): java → plan → spawn → track
// ---------------------------------------------------------------------------

/// Launch an instance with an offline profile identity. Microsoft accounts
/// arrive with the authentication milestone; nothing here fabricates tokens.
#[tauri::command]
fn launch_instance(
    data: State<'_, Mutex<AppData>>,
    id: String,
    username: String,
) -> Result<u32, CommandError> {
    transition(&data, LaunchPhase::Preparing)?;

    let (data_dir, agent) = {
        let app = lock(&data)?;
        (app.data_dir.clone(), app.agent.clone())
    };

    let instance = {
        let app = lock(&data)?;
        find_instance(&app, &id)?
    };

    // Effective metadata must exist from a prior install.
    transition(&data, LaunchPhase::ResolvingMetadata)?;
    let profile_path = data_dir
        .join("cache")
        .join("profiles")
        .join(format!("{id}.json"));
    let meta: VersionMetadata =
        serde_json::from_slice(&std::fs::read(&profile_path).map_err(|_| {
            CommandError::runtime_unavailable(
                "This instance is not installed yet — install it before launching.",
            )
        })?)
        .map_err(ser_err)?;

    // Java resolution: probe candidates, require the metadata's floor.
    transition(&data, LaunchPhase::ResolvingJava)?;
    let required_major = meta.required_java_major();
    let found = java::discover();
    let runtime: &JavaRuntime = java::select(&found, required_major).map_err(CommandError::from)?;
    info!(major = runtime.major_version, path = %runtime.executable.display(), "java selected");

    // Classpath: client jar + every applicable library from a fresh plan.
    transition(&data, LaunchPhase::BuildingPlan)?;
    let cache_root = data_dir.join("cache");
    let plan_files = plan_install(&meta, &cache_root)?;
    let classpath: Vec<PathBuf> = std::iter::once(
        cache_root
            .join("versions")
            .join(meta.id.clone())
            .join(format!("{}.jar", meta.id)),
    )
    .chain(
        plan_files
            .artifacts
            .iter()
            .filter(|a| a.kind == ArtifactKind::Library)
            .map(|a| a.dest.clone()),
    )
    .collect();

    let game_dir = game_dir_of(&data_dir, &id);
    std::fs::create_dir_all(&game_dir).map_err(io_err("cannot create game dir"))?;

    // Logging config artifact, when present.
    let logging_config = plan_files
        .artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::LoggingConfig)
        .map(|a| a.dest.clone());

    let identity =
        ikk_minecraft::account::LaunchIdentity::offline(username).map_err(CommandError::from)?;
    let options = LaunchOptions {
        game_dir: game_dir.clone(),
        assets_dir: cache_root.join("assets"),
        natives_dir: natives_dir_of(&game_dir),
        classpath,
        logging_config,
        memory_mb: None, // profile-level memory arrives with profiles UI
        jvm_extra: Vec::new(),
    };
    let plan = build_plan(&meta, &identity, runtime, &options).map_err(CommandError::from)?;

    // Spawn + track. Logs land next to the instance, argv is NOT logged
    // (it carries the access-token slot).
    transition(&data, LaunchPhase::Starting)?;
    let log_path = game_dir.join("logs").join("latest-launch.log");
    let managed = process::spawn(&plan, &game_dir, &log_path).map_err(CommandError::from)?;
    let pid = managed.pid();

    record_launch_history(&game_dir, pid)?;

    let mut app = lock(&data)?;
    app.running = Some(managed);
    app.phase
        .transition(LaunchPhase::Running)
        .map_err(CommandError::from)?;
    info!(pid, instance = %id, "minecraft started");
    Ok(pid)
}

/// Poll the tracked process; moves Running → Completed/Failed with the real
/// exit classification.
#[tauri::command]
fn launch_status(
    data: State<'_, Mutex<AppData>>,
) -> Result<ikk_api_types::LaunchStatusDto, CommandError> {
    let mut app = lock(&data)?;
    let phase = app.phase.phase();
    if phase == LaunchPhase::Running {
        if let Some(proc_handle) = app.running.as_mut() {
            if let Some(exit) = proc_handle.try_wait()? {
                let exit_dto = ikk_api_types::GameExitDto {
                    exit_code: exit.exit_code,
                    user_stopped: exit.user_stopped,
                    category: exit.category().to_owned(),
                };
                app.phase
                    .transition(if exit.succeeded() {
                        LaunchPhase::Completed
                    } else {
                        LaunchPhase::Failed
                    })
                    .map_err(CommandError::from)?;
                let log = proc_handle.log_path().display().to_string();
                return Ok(ikk_api_types::LaunchStatusDto {
                    phase: app.phase.phase().as_str().to_owned(),
                    pid: None,
                    exit: Some(exit_dto),
                    log_path: Some(log),
                });
            }
            return Ok(ikk_api_types::LaunchStatusDto {
                phase: phase.as_str().to_owned(),
                pid: Some(proc_handle.pid()),
                exit: None,
                log_path: Some(proc_handle.log_path().display().to_string()),
            });
        }
    }
    Ok(ikk_api_types::LaunchStatusDto {
        phase: phase.as_str().to_owned(),
        pid: None,
        exit: None,
        log_path: None,
    })
}

#[tauri::command]
fn stop_launch(data: State<'_, Mutex<AppData>>) -> Result<bool, CommandError> {
    let mut app = lock(&data)?;
    if let Some(handle) = app.running.as_mut() {
        handle.kill()?;
        info!("user requested minecraft stop");
        return Ok(true);
    }
    Ok(false)
}

/// Tail of the merged game output for the console view.
#[tauri::command]
fn read_launch_log(
    data: State<'_, Mutex<AppData>>,
    max_bytes: i64,
) -> Result<String, CommandError> {
    let app = lock(&data)?;
    let path = app.running.as_ref().map(|h| h.log_path().to_path_buf());
    let Some(path) = path else {
        return Ok(String::new());
    };
    drop(app);
    read_tail(&path, max_bytes.max(0) as u64)
}

fn read_tail(path: &std::path::Path, max_bytes: u64) -> Result<String, CommandError> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(path) else {
        return Ok(String::new());
    };
    let len = file.metadata().map_err(io_err("cannot stat log"))?.len();
    let start = len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))
        .map_err(io_err("cannot seek log"))?;
    let mut buf = Vec::with_capacity((len - start).min(1024 * 1024) as usize);
    file.read_to_end(&mut buf)
        .map_err(io_err("cannot read log"))?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Minimal append-only history: when we launched and how it ended.
/// No personal data beyond the chosen username length is recorded.
fn record_launch_history(game_dir: &std::path::Path, pid: u32) -> Result<(), CommandError> {
    #[derive(serde::Serialize)]
    struct HistoryEntry {
        launched_at_unix: u64,
        pid: u32,
    }
    let dir = game_dir.join("ikk");
    std::fs::create_dir_all(dir).map_err(io_err("cannot create ikk dir"))?;
    let entry = HistoryEntry {
        launched_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        pid,
    };
    let line = serde_json::to_string(&entry).map_err(ser_err)?;
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("launch-history.jsonl"))
            .map_err(io_err("cannot open launch history"))?;
        writeln!(file, "{line}").map_err(io_err("cannot write launch history"))?;
    }
    Ok(())
}

fn transition(data: &State<'_, Mutex<AppData>>, next: LaunchPhase) -> Result<(), CommandError> {
    let mut app = lock(data)?;
    app.phase.transition(next).map_err(CommandError::from)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Mod management (Phase 6): search → resolve → confirm → staged install.
// All HTTP lives in ikk_minecraft::mods; these commands are thin adapters
// that own only path resolution and state persistence.
// ---------------------------------------------------------------------------

use ikk_minecraft::mods::install::{
    self as mods_install, ModsData, ModsStore,
};
use ikk_minecraft::mods::modrinth::{parse_versions, USER_AGENT as MODRINTH_UA};
use ikk_minecraft::mods::resolver::{self, InstanceContext, VersionLookup};
use ikk_minecraft::mods::{ProjectRef, SourceKind};

fn instance_dirs(app: &AppData, id: &str) -> (PathBuf, PathBuf) {
    let game = game_dir_of(&app.data_dir, id);
    (game.join("ikk"), game.join("mods"))
}

fn load_mods_data(app: &AppData, id: &str) -> Result<ModsData, CommandError> {
    let (ikk_dir, _) = instance_dirs(app, id);
    match ModsStore::new(&ikk_dir).load() {
        Ok((data, _)) => Ok(data),
        Err(e) => Err(CommandError::from(e)),
    }
}

fn save_mods_data(app: &AppData, id: &str, data: &ModsData) -> Result<(), CommandError> {
    let (ikk_dir, _) = instance_dirs(app, id);
    ModsStore::new(&ikk_dir)
        .save(data)
        .map_err(CommandError::from)
}

fn parse_source(source: &str) -> Result<SourceKind, CommandError> {
    match source {
        "modrinth" => Ok(SourceKind::Modrinth),
        other => Err(CommandError::internal(format!(
            "unknown mod source {other:?}"
        ))),
    }
}

/// Network-backed version lookup with request dedup within one resolve run.
struct NetLookup<'a> {
    agent: &'a ureq::Agent,
    cache: std::collections::HashMap<String, Vec<ikk_minecraft::mods::ProjectVersion>>,
}

impl<'a> NetLookup<'a> {
    fn new(agent: &'a ureq::Agent) -> Self {
        Self {
            agent,
            cache: std::collections::HashMap::new(),
        }
    }
}

impl VersionLookup for NetLookup<'_> {
    fn versions_of(
        &mut self,
        project_id: &str,
    ) -> ikk_core::Result<Vec<ikk_minecraft::mods::ProjectVersion>> {
        if let Some(v) = self.cache.get(project_id) {
            return Ok(v.clone());
        }
        let reference = ProjectRef::new(SourceKind::Modrinth, project_id);
        let url = format!(
            "https://api.modrinth.com/v2/project/{}/version",
            reference.project_id
        );
        let json = ikk_minecraft::fetch_text_with(self.agent, &url, MODRINTH_UA)?;
        let versions = parse_versions(&json)?;
        self.cache.insert(project_id.to_owned(), versions.clone());
        Ok(versions)
    }
}

fn instance_context_of(instance: &Instance) -> InstanceContext {
    InstanceContext::new(
        instance.minecraft_version.as_str().to_owned(),
        loader_id_of(&instance.loader.kind).as_str().to_owned(),
    )
}

/// Search a mod source, scoped to the instance's Minecraft version + loader.
#[tauri::command]
fn mods_search(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
    query: String,
    sort: Option<String>,
    page: u32,
) -> Result<Vec<ikk_api_types::ModProjectDto>, CommandError> {
    let (instance, agent) = {
        let app = lock(&data)?;
        (find_instance(&app, &instance_id)?, app.agent.clone())
    };
    let ctx = instance_context_of(&instance);
    let q = ikk_minecraft::mods::source::SearchQuery {
        text: query,
        game_versions: vec![ctx.game_version],
        loaders: vec![ctx.loader],
        categories: Vec::new(),
        sort,
        page: page.max(1),
    };
    let hits = <ikk_minecraft::mods::modrinth::ModrinthSource as ikk_minecraft::mods::source::ModSource>::search(
        &ikk_minecraft::mods::modrinth::ModrinthSource,
        &agent,
        &q,
    )?;
    Ok(hits
        .into_iter()
        .map(|h| ikk_api_types::ModProjectDto {
            source: h.project.reference.source.as_str().to_owned(),
            project_id: h.project.reference.project_id,
            title: h.project.title,
            description: h.project.description,
            authors: h.project.authors,
            icon_url: h.project.icon_url,
            downloads: h.project.downloads,
            categories: h.project.categories,
            game_versions: h.project.game_versions,
        })
        .collect())
}

/// Versions of one project that support THIS instance (compatibility is
/// resolved server-of-truth side, then filtered here).
#[tauri::command]
fn mods_compatible_versions(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
    source: String,
    project_id: String,
) -> Result<Vec<ikk_api_types::ModVersionDto>, CommandError> {
    let (instance, agent) = {
        let app = lock(&data)?;
        (find_instance(&app, &instance_id)?, app.agent.clone())
    };
    parse_source(&source)?;
    let ctx = instance_context_of(&instance);
    let url = format!(
        "https://api.modrinth.com/v2/project/{project_id}/version"
    );
    let json = ikk_minecraft::fetch_text_with(&agent, &url, MODRINTH_UA)?;
    let versions = parse_versions(&json)?;
    Ok(versions
        .iter()
        .filter(|v| v.supports(&ctx.game_version, &ctx.loader))
        .map(|v| {
            let file = v.primary_file();
            ikk_api_types::ModVersionDto {
                version_id: v.version_id.clone(),
                version_number: v.version_number.clone(),
                release_type: v.release_type.clone(),
                filename: file.map(|f| f.filename.clone()).unwrap_or_default(),
                size_bytes: file.map(|f| f.size_bytes).unwrap_or(0),
                hash_verified_source: file.and_then(|f| f.sha1.as_ref()).is_some(),
            }
        })
        .collect())
}

/// Resolve an install request into a confirmation payload WITHOUT touching
/// disk. The UI shows this first (§30/§31).
#[tauri::command]
fn mods_install_plan(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
    source: String,
    project_id: String,
) -> Result<ikk_api_types::ModInstallPlanDto, CommandError> {
    let (instance, installed, agent) = {
        let app = lock(&data)?;
        (
            find_instance(&app, &instance_id)?,
            load_mods_data(&app, &instance_id)?.installed,
            app.agent.clone(),
        )
    };
    parse_source(&source)?;
    let ctx = instance_context_of(&instance);
    let mut lookup = NetLookup::new(&agent);
    let plan = resolver::resolve(&project_id, &ctx, &mut lookup, &installed)?;
    Ok(plan_to_dto(&plan))
}

fn plan_to_dto(
    plan: &ikk_minecraft::mods::resolver::InstallPlan,
) -> ikk_api_types::ModInstallPlanDto {
    ikk_api_types::ModInstallPlanDto {
        to_install: plan
            .to_install
            .iter()
            .map(|v| ikk_api_types::ModProjectDto {
                source: v.project.source.as_str().to_owned(),
                project_id: v.project.project_id.clone(),
                title: v.version_number.clone(), // display name arrives with details fetch
                description: String::new(),
                authors: Vec::new(),
                icon_url: None,
                downloads: 0,
                categories: Vec::new(),
                game_versions: v.game_versions.clone(),
            })
            .collect(),
        already_installed: plan
            .already_installed
            .iter()
            .map(|p| p.project_id.clone())
            .collect(),
        unsatisfiable: plan.unsatisfiable.clone(),
        conflicts: plan.conflicts.iter().map(|c| c.project_title.clone()).collect(),
    }
}

/// The real installation: staged download → verify → commit files → record.
/// Metadata is written only when every file succeeded (atomic at the set level).
#[tauri::command]
fn mods_install(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
    source: String,
    project_id: String,
) -> Result<ikk_api_types::ModInstallReportDto, CommandError> {
    let (instance, mut mods_data, agent, cancel) = {
        let app = lock(&data)?;
        (
            find_instance(&app, &instance_id)?,
            load_mods_data(&app, &instance_id)?,
            app.agent.clone(),
            app.cancel_flag(),
        )
    };
    parse_source(&source)?;
    let (_, mods_dir) = instance_dirs(&lock(&data)?, &instance_id);

    let ctx = instance_context_of(&instance);
    let mut lookup = NetLookup::new(&agent);
    let plan = resolver::resolve(&project_id, &ctx, &mut lookup, &mods_data.installed)?;
    if !plan.unsatisfiable.is_empty() || !plan.conflicts.is_empty() {
        return Err(CommandError::from(ikk_core::Error::new(
            ikk_core::ErrorCode::InstanceInvalid,
            format!(
                "cannot install {}: {}",
                project_id,
                if !plan.unsatisfiable.is_empty() {
                    format!("unsatisfiable: {}", plan.unsatisfiable.join("; "))
                } else {
                    format!("conflicts with: {}", plan.summary)
                }
            ),
        )));
    }

    let opts = DownloadOptions {
        retries: 2,
        cancel,
    };
    let (outcome, rows) = mods_install::install_plan(&agent, &plan.to_install, &mods_dir, &opts)
        .map_err(CommandError::from)?;

    let report = ikk_api_types::ModInstallReportDto {
        downloaded: outcome.downloaded.clone(),
        skipped: outcome.skipped.clone(),
        unverified: outcome.unverified.clone(),
        failed: outcome.failed.clone(),
    };

    if outcome.ok() {
        for row in rows {
            mods_data.installed.retain(|m| m.project != row.project);
            mods_data.installed.push(row);
        }
        save_mods_data(&data, &instance_id, &mods_data)?;
        info!(instance = %instance_id, count = report.downloaded.len() + report.skipped.len(), "mods installed");
    }
    Ok(report)
}

/// Inventory = persisted metadata reconciled against the actual directory.
#[tauri::command]
fn mods_inventory(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
) -> Result<ikk_api_types::ModInventoryDto, CommandError> {
    let (mods_data, (_, mods_dir)) = {
        let app = lock(&data)?;
        let dirs = instance_dirs(&app, &instance_id);
        (load_mods_data(&app, &instance_id)?, dirs)
    };
    let inv = ikk_minecraft::mods::install::reconcile(&mods_data, &mods_dir);
    Ok(ikk_api_types::ModInventoryDto {
        mods: inv
            .mods
            .iter()
            .map(|e| ikk_api_types::InstalledModDto {
                source: e
                    .project
                    .as_ref()
                    .map(|p| p.source.as_str().to_owned())
                    .unwrap_or_else(|| "local".to_owned()),
                project_id: e.project.as_ref().map(|p| p.project_id.clone()),
                title: e.title.clone(),
                filename: e.filename.clone(),
                version_number: e.version_number.clone(),
                enabled: e.enabled,
                state: match e.state {
                    ikk_minecraft::mods::ManagedState::Managed => "managed".to_owned(),
                    ikk_minecraft::mods::ManagedState::External => "external".to_owned(),
                    ikk_minecraft::mods::ManagedState::Missing => "missing".to_owned(),
                },
                warning: e.warning.clone(),
            })
            .collect(),
    })
}

/// Enable or disable ONE tracked mod (reversible file rename). External jars
/// are rejected — their toggling belongs to the user's file manager.
#[tauri::command]
fn mods_set_enabled(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
    project_id: String,
    enabled: bool,
) -> Result<(), CommandError> {
    let mut mods_data = {
        let app = lock(&data)?;
        load_mods_data(&app, &instance_id)?
    };
    let (_, mods_dir) = instance_dirs(&lock(&data)?, &instance_id);
    let reference = ProjectRef::new(SourceKind::Modrinth, &project_id);
    let Some(row) = mods_data
        .installed
        .iter_mut()
        .find(|m| m.project == reference)
    else {
        return Err(CommandError::from(ikk_core::Error::new(
            ikk_core::ErrorCode::InstanceNotFound,
            format!("no tracked mod {project_id}"),
        )));
    };
    row.enabled = enabled;
    ikk_minecraft::mods::install::apply_enabled_state(&mut mods_data, &mods_dir)
        .map_err(CommandError::from)?;
    save_mods_data(&data, &instance_id, &mods_data)
}

/// Remove a managed mod. Refuses while other tracked mods still require it
/// (reverse-dependency analysis); pass `force` after showing the user why.
#[tauri::command]
fn mods_remove(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
    project_id: String,
    force: bool,
) -> Result<(), CommandError> {
    let mut mods_data = {
        let app = lock(&data)?;
        load_mods_data(&app, &instance_id)?
    };
    let (_, mods_dir) = instance_dirs(&lock(&data)?, &instance_id);
    let reference = ProjectRef::new(SourceKind::Modrinth, &project_id);
    if !force {
        let blockers = ikk_minecraft::mods::resolver::reverse_dependencies(
            &reference,
            &mods_data.installed,
            &[reference.clone()],
        );
        if !blockers.is_empty() {
            let names: Vec<String> = blockers.iter().map(|p| p.project_id.clone()).collect();
            return Err(CommandError::from(ikk_core::Error::new(
                ikk_core::ErrorCode::InstanceInvalid,
                format!(
                    "{project_id} is still required by: {}. Remove those first, or confirm forced removal.",
                    names.join(", ")
                ),
            )));
        }
    }
    let removed = ikk_minecraft::mods::install::remove_managed(&mut mods_data, &mods_dir, &reference)
        .map_err(CommandError::from)?;
    if removed {
        save_mods_data(&data, &instance_id, &mods_data)?;
        info!(instance = %instance_id, %project_id, "mod removed");
    }
    Ok(())
}

/// Update detection across every managed mod (§24).
#[tauri::command]
fn mods_updates(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
) -> Result<Vec<ikk_api_types::ModUpdateDto>, CommandError> {
    let (instance, mods_data, agent) = {
        let app = lock(&data)?;
        (
            find_instance(&app, &instance_id)?,
            load_mods_data(&app, &instance_id)?,
            app.agent.clone(),
        )
    };
    let ctx = instance_context_of(&instance);
    let mut updates = Vec::new();
    for m in &mods_data.installed {
        let url = format!(
            "https://api.modrinth.com/v2/project/{}/version",
            m.project.project_id
        );
        let state = match ikk_minecraft::fetch_text_with(&agent, &url, MODRINTH_UA)
            .and_then(|j| parse_versions(&j))
        {
            Ok(available) => {
                let s = ikk_minecraft::mods::resolver::update_state(m, &available, &ctx);
                let best = ikk_minecraft::mods::resolver::select_compatible(&available, &ctx);
                (s, best.map(|b| b.version_number.clone()))
            }
            Err(_) => (
                ikk_minecraft::mods::resolver::UpdateState::Unknown,
                None,
            ),
        };
        use ikk_minecraft::mods::resolver::UpdateState as US;
        updates.push(ikk_api_types::ModUpdateDto {
            project_id: m.project.project_id.clone(),
            installed_version: m.version_number.clone(),
            available_version: state.1,
            state: match state.0 {
                US::Current => "current",
                US::UpdateAvailable => "update-available",
                US::Incompatible => "incompatible",
                US::Unknown => "unknown",
            }
            .to_owned(),
        });
    }
    Ok(updates)
}

// -- mod profiles ------------------------------------------------------------

fn profiles_to_dto(mods_data: &ModsData) -> Vec<ikk_api_types::ModProfileDto> {
    mods_data
        .profiles
        .iter()
        .map(|p| ikk_api_types::ModProfileDto {
            id: p.id.clone(),
            name: p.name.clone(),
            enabled_count: p.enabled_projects.len() as u32,
            active: mods_data.active_profile.as_deref() == Some(p.id.as_str()),
        })
        .collect()
}

#[tauri::command]
fn mods_list_profiles(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
) -> Result<Vec<ikk_api_types::ModProfileDto>, CommandError> {
    let app = lock(&data)?;
    Ok(profiles_to_dto(&load_mods_data(&app, &instance_id)?))
}

#[tauri::command]
fn mods_create_profile(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
    name: String,
) -> Result<Vec<ikk_api_types::ModProfileDto>, CommandError> {
    let mut mods_data = {
        let app = lock(&data)?;
        load_mods_data(&app, &instance_id)?
    };
    let id = format!("profile-{}", mods_data.profiles.len() + 1);
    let profile = ikk_minecraft::mods::install::create_profile_from_current(&mods_data, id, name)
        .map_err(CommandError::from)?;
    mods_data.profiles.push(profile);
    {
        let app = lock(&data)?;
        save_mods_data(&app, &instance_id, &mods_data)?;
    }
    Ok(profiles_to_dto(&mods_data))
}

/// Switch the active profile (`null` resets to all-enabled). Files are
/// renamed on disk; no downloads occur.
#[tauri::command]
fn mods_switch_profile(
    data: State<'_, Mutex<AppData>>,
    instance_id: String,
    profile_id: Option<String>,
) -> Result<(), CommandError> {
    let mut mods_data = {
        let app = lock(&data)?;
        load_mods_data(&app, &instance_id)?
    };
    let (_, mods_dir) = instance_dirs(&lock(&data)?, &instance_id);
    ikk_minecraft::mods::install::switch_profile(&mut mods_data, &mods_dir, profile_id.as_deref())
        .map_err(CommandError::from)?;
    save_mods_data(&data, &instance_id, &mods_data)
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

fn io_err(context: &'static str) -> impl Fn(std::io::Error) -> CommandError {
    move |e| {
        CommandError::from(ikk_core::Error::with_source(
            ikk_core::ErrorCode::IoFailure,
            context,
            e,
        ))
    }
}

fn ser_err(e: serde_json::Error) -> CommandError {
    CommandError::from(ikk_core::Error::with_source(
        ikk_core::ErrorCode::Internal,
        "serialization failure",
        e,
    ))
}

// ---------------------------------------------------------------------------
// Startup sequence
// ---------------------------------------------------------------------------

pub fn run() {
    // 1. Logging first so every later step is observable.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    info!(version = env!("CARGO_PKG_VERSION"), "Isekaiyo starting");

    // 2. Platform paths (IKK_DATA_DIR overrides; see ikk-core::platform).
    let data_dir = match ikk_core::platform::data_dir() {
        Some(dir) => dir,
        None => {
            error!(
                "fatal: could not resolve a platform data directory (set IKK_DATA_DIR to override)"
            );
            std::process::exit(1);
        }
    };
    info!(dir = %data_dir.display(), "platform data directory resolved");

    // 3. Configuration — never crashes on missing/corrupt files.
    let config_store = ConfigStore::new(&data_dir);
    let loaded = config_store.load();
    if loaded.source == LoadSource::RecoveredCorrupt {
        if let Some(backup) = &loaded.corrupt_backup {
            warn!(
                backup = %backup.display(),
                "configuration file was malformed; it was preserved and defaults are in effect"
            );
        }
    }
    let startup_info = ConfigLoadInfo {
        source: loaded.source.into(),
        corrupt_backup_path: loaded.corrupt_backup.map(|p| p.display().to_string()),
    };

    // 4. Core services.
    let instance_store = InstanceStore::new(data_dir.join("instances"));
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(30))
        .build();
    info!("core services initialized");

    // 5. UI.
    tauri::Builder::default()
        .manage(Mutex::new(AppData {
            config_store,
            instance_store,
            startup_info,
            data_dir,
            phase: PhaseTracker::new(),
            progress: InstallProgress::default(),
            running: None,
            agent,
            cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }))
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_startup_info,
            get_config,
            set_config,
            list_instances,
            create_instance,
            update_instance,
            delete_instance,
            list_versions,
            list_loader_versions,
            install_instance,
            launch_instance,
            launch_status,
            stop_launch,
            read_launch_log,
            mods_search,
            mods_compatible_versions,
            mods_install_plan,
            mods_install,
            mods_inventory,
            mods_set_enabled,
            mods_remove,
            mods_updates,
            mods_list_profiles,
            mods_create_profile,
            mods_switch_profile
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("fatal: Tauri runtime failed to start: {e}");
            std::process::exit(1);
        });

    info!("Isekaiyo stopped cleanly");
}
