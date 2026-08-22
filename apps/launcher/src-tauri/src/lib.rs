//! Isekaiyo launcher application shell.
//!
//! Milestone 1 scope (docs/roadmap.md): prove
//! frontend → Tauri → Rust → ikk-core end to end, with clean startup/shutdown,
//! structured startup logging, and one real command (`get_system_info`).
//! No launcher features live here yet — by design (spec §22/§23).

use ikk_api_types::SystemInfo;
use tracing::info;

#[tauri::command]
fn get_system_info() -> SystemInfo {
    SystemInfo {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        target: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        profile: if cfg!(debug_assertions) { "debug" } else { "release" }.to_owned(),
    }
}

pub fn run() {
    // Structured startup logging; RUST_LOG controls verbosity (.env.example).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!(version = env!("CARGO_PKG_VERSION"), "Isekaiyo launcher starting");

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_system_info])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("fatal: Tauri runtime failed to start: {e}");
            std::process::exit(1);
        });

    info!("Isekaiyo launcher stopped cleanly");
}
