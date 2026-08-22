//! `cargo xtask` — Isekaiyo developer tooling.
//!
//! Subcommands:
//! - `doctor` — validate the development environment
//! - `arch`   — enforce workspace dependency-direction rules
//!   (docs/architecture/dependency-rules.md)
//!
//! Both fail loudly; neither silently "fixes" anything.

use std::process::Command;

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
    let cmd = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "doctor".to_owned());
    match cmd.as_str() {
        "doctor" => doctor(),
        "arch" => arch::run(),
        other => {
            eprintln!("unknown xtask command: {other}");
            eprintln!("usage: cargo xtask [doctor|arch]");
            std::process::exit(2);
        }
    }
}

mod arch {
    //! Dependency-direction enforcement without external crates:
    //! scan each member manifest for path dependencies and compare the actual
    //! graph against the allowlist in docs/architecture/dependency-rules.md.

    use std::fs;
    use std::path::Path;

    /// allowed[dependent] = set of crates it may depend on.
    const ALLOWED_EDGES: &[(&str, &[&str])] = &[
        // Core depends on nothing inside the workspace.
        ("ikk-core", &[]),
        // The Minecraft engine speaks to core's error taxonomy only.
        ("ikk-minecraft", &["ikk-core"]),
        // DTOs may reference core types only.
        ("ikk-api-types", &["ikk-core"]),
        // The application shell composes libraries; libraries never know it.
        (
            "ikk-launcher",
            &["ikk-core", "ikk-api-types", "ikk-minecraft"],
        ),
        // The task runner is standalone by design.
        ("xtask", &[]),
    ];

    fn path_deps(manifest: &str) -> Vec<String> {
        manifest
            .lines()
            .filter_map(|line| line.trim().strip_prefix("ikk-"))
            .filter_map(|rest| rest.split_whitespace().next())
            .map(|name| format!("ikk-{name}"))
            .collect()
    }

    pub fn run() {
        let members = [
            ("ikk-core", "crates/ikk-core"),
            ("ikk-minecraft", "crates/ikk-minecraft"),
            ("ikk-api-types", "crates/ikk-api-types"),
            ("ikk-launcher", "apps/launcher/src-tauri"),
            ("xtask", "tools/xtask"),
        ];

        let mut violations = Vec::new();
        for (name, dir) in members {
            let manifest_path = Path::new(dir).join("Cargo.toml");
            let text = fs::read_to_string(&manifest_path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", manifest_path.display()));
            for dep in path_deps(&text) {
                let ok = ALLOWED_EDGES
                    .iter()
                    .find(|(from, _)| *from == name)
                    .is_some_and(|(_, to)| to.contains(&dep.as_str()));
                if !ok {
                    violations.push(format!("{name} -> {dep} is not an allowed edge"));
                }
            }
        }

        if violations.is_empty() {
            println!("Dependency direction OK ({} edges checked).", members.len());
        } else {
            eprintln!("Architecture violations:");
            for v in &violations {
                eprintln!("  ✗ {v}");
            }
            eprintln!("See docs/architecture/dependency-rules.md; changes require an ADR.");
            std::process::exit(1);
        }
    }
}

fn doctor() {
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
        for check in CHECKS {
            if failures.iter().any(|&(name, _)| name == check.name) {
                println!();
                println!("{}:", check.name);
                println!("  Required : {}", check.min_hint);
                println!("  Install  : {}", check.install);
            }
        }
        std::process::exit(1);
    }
}
