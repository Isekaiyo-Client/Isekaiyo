//! ikk-minecraft — the Isekaiyo Minecraft engine.
//!
//! Everything needed to take an instance from "user picked a version" to
//! "Minecraft process running", with no UI and no Tauri:
//!
//! - [`manifest`] / [`metadata`] — official Mojang metadata parsing (offline-testable)
//! - [`rules`] — platform rule evaluation
//! - [`resolve`] — deterministic install plans (client, libraries, natives, assets, logging)
//! - [`download`] — verified streaming downloads with skip/retry/atomic-replace/cancel
//! - [`java`] — Java discovery, version parsing, compatibility
//! - [`account`] — account states; never fabricates authentication tokens
//! - [`planner`] — LaunchPlan construction (JVM + game args); the UI never builds args
//! - [`process`] — cross-platform spawn/output-capture/exit tracking
//! - [`state`] — the launch state machine

pub mod account;
pub mod assets;
pub mod download;
pub mod java;
pub mod loaders;
pub mod manifest;
pub mod metadata;
pub mod natives;
pub mod planner;
pub mod process;
pub mod resolve;
pub mod rules;
pub mod state;

#[cfg(test)]
pub(crate) mod test_fixtures;

use ikk_core::Result;

/// Whether a library entry applies on the given platform context.
pub(crate) fn resolve_lib_rules(lib: &metadata::Library, ctx: &rules::EvalContext) -> bool {
    match &lib.rules {
        Some(rule_list) => rules::evaluate(rule_list, ctx),
        None => true,
    }
}

/// Shared HTTP agent: TLS via rustls, sane timeouts. One agent per process so
/// connection pooling works across artifacts.
pub fn http_agent() -> Result<ureq::Agent> {
    Ok(ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(30))
        .build())
}

/// Fetch a URL as UTF-8 text (manifests, metadata). Network errors map to the
/// stable `network.timeout` category; non-200s to `metadata.invalid` with the
/// status included.
pub fn fetch_text(agent: &ureq::Agent, url: &str) -> Result<String> {
    match agent.get(url).call() {
        Ok(response) => response.into_string().map_err(|e| {
            ikk_core::Error::with_source(
                ikk_core::ErrorCode::MetadataInvalid,
                format!("failed reading body of {url}"),
                e,
            )
        }),
        Err(ureq::Error::Status(status, _)) => Err(ikk_core::Error::new(
            ikk_core::ErrorCode::MetadataInvalid,
            format!("HTTP {status} fetching {url}"),
        )),
        Err(e) => Err(ikk_core::Error::with_source(
            ikk_core::ErrorCode::NetworkTimeout,
            format!("network error fetching {url}"),
            e,
        )),
    }
}
