//! Isekaiyo launcher application shell — Milestone 001 vertical slice.
//!
//! Responsibilities (spec §7): run the startup sequence
//!   logging → platform paths → configuration → instance store → UI
//! and expose the ONLY surface the frontend may talk to: these typed commands.
//! All business logic lives in `ikk-core`; this file is composition glue.

use ikk_api_types::{
    AppConfig, CommandError, ConfigLoadInfo, Instance, InstanceListing, LoaderKindInput, SystemInfo,
};
use ikk_core::config::{ConfigStore, LoadSource};
use ikk_core::ids::InstanceId;
use ikk_core::store::InstanceStore;
use std::sync::{Mutex, MutexGuard};
use tauri::State;
use tracing::{error, info, warn};

/// Everything the application owns at runtime. Guarded by one mutex: all
/// operations here are fast file IO on tiny JSON files; contention is a
/// non-issue at this scale and correctness beats lock granularity.
struct AppData {
    config_store: ConfigStore,
    instance_store: InstanceStore,
    /// Frozen snapshot of how configuration came up, for the UI to display.
    startup_info: ConfigLoadInfo,
}

fn lock(data: &State<'_, Mutex<AppData>>) -> Result<MutexGuard<'_, AppData>, CommandError> {
    data.lock().map_err(|_| CommandError::internal("application state lock poisoned"))
}

// ---------------------------------------------------------------------------
// Commands (the entire UI↔core boundary)
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_system_info() -> SystemInfo {
    SystemInfo {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        target: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        profile: if cfg!(debug_assertions) { "debug" } else { "release" }.to_owned(),
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
fn set_config(data: State<'_, Mutex<AppData>>, config: AppConfig) -> Result<AppConfig, CommandError> {
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
fn update_instance(data: State<'_, Mutex<AppData>>, instance: Instance) -> Result<Instance, CommandError> {
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
            error!("fatal: could not resolve a platform data directory (set IKK_DATA_DIR to override)");
            std::process::exit(1);
        }
    };
    info!(dir = %data_dir.display(), "platform data directory resolved");

    // 3. Configuration — never crashes on missing/corrupt files (spec §9).
    let config_store = ConfigStore::new(&data_dir);
    let loaded = config_store.load();
    if loaded.source == LoadSource::RecoveredCorrupt {
        if let Some(backup) = &loaded.corrupt_backup {
            warn!(
                backup = %backup.display(),
                "configuration file was malformed; it was preserved and defaults are in effect"
            );
        }
    } else if loaded.source == LoadSource::Defaults {
        info!("no configuration file yet; using defaults");
    } else {
        info!("configuration loaded");
    }
    let startup_info = ConfigLoadInfo {
        source: loaded.source.into(),
        corrupt_backup_path: loaded.corrupt_backup.map(|p| p.display().to_string()),
    };

    // 4. Core services (Milestone 001 scope: instances).
    let instance_store = InstanceStore::new(data_dir.join("instances"));
    info!("core services initialized");

    // 5. UI.
    tauri::Builder::default()
        .manage(Mutex::new(AppData { config_store, instance_store, startup_info }))
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_startup_info,
            get_config,
            set_config,
            list_instances,
            create_instance,
            update_instance,
            delete_instance
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("fatal: Tauri runtime failed to start: {e}");
            std::process::exit(1);
        });

    info!("Isekaiyo stopped cleanly");
}
