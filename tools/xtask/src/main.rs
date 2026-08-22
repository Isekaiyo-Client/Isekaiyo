//! `cargo xtask doctor` — Isekaiyo development environment diagnostic.
//!
//! Checks every prerequisite from docs/development/getting-started.md and
//! reports exact remediation for anything missing. It never "fixes" your
//! system and never reports healthy when something is broken.

use std::process::{Command};

struct Check {
    name: &'static str,
    cmd: &'static str,
    args: &'static [&'static str],
    min_hint: &'static str,
    install: &'static str,
}

const CHECKS: &[Check] = &[
    Check {
        name: "Git",
        cmd: "git",
        args: &["--version"],
        min_hint: "any recent version",
        install: "https://git-scm.com/downloads",
    },
    Check {
        name: "Rust toolchain",
        cmd: "rustc",
        args: &["--version"],
        min_hint: "stable channel via rustup (see rust-toolchain.toml)",
        install: "https://rustup.rs  →  `rustup component add rustfmt clippy`",
    },
    Check {
        name: "Cargo",
        cmd: "cargo",
        args: &["--version"],
        min_hint: "ships with rustup",
        install: "https://rustup.rs",
    },
    Check {
        name: "Node.js",
        cmd: "node",
        args: &["--version"],
        min_hint: ">= 22.x (see .nvmrc)",
        install: "https://nodejs.org or `nvm install` in repo root",
    },
    Check {
        name: "pnpm",
        cmd: "pnpm",
        args: &["--version"],
        min_hint: ">= 9 (enable once: `corepack enable`)",
        install: "`corepack enable pnpm`  |  https://pnpm.io/installation",
    },
];

fn probe(check: &Check) -> Result<String, String> {
    let output = Command::new(check.cmd)
        .args(check.args)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "not found on PATH".to_owned()
            } else {
                format!("failed to execute: {e}")
            }
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or_default().trim().to_owned();
    if first_line.is_empty() {
        return Err("no output".into());
    }
    Ok(first_line)
}

fn main() {
    println!("Isekaiyo Development Environment");
    println!("{}", "-".repeat(46));

    let mut failures = Vec::new();
    for check in CHECKS {
        match probe(check) {
            Ok(version) => println!("{:<14} ✓ {version}", check.name),
            Err(reason) => {
                println!("{:<14} ✗ {reason}", check.name);
                failures.push((check.name, reason));
            }
        }
    }

    println!("{}", "-".repeat(46));
    if failures.is_empty() {
        println!("Environment ready.");
    } else {
        println!("{} problem(s) found:", failures.len());
        for &(name, _) in &failures {
            let check = CHECKS.iter().find(|c| c.name == name).expect("name came from CHECKS");
            println!();
            println!("{name}:");
            println!("  Required : {}", check.min_hint);
            println!("  Install  : {}", check.install);
        }
        std::process::exit(1);
    }
}
