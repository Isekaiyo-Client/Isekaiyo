//! Platform rule evaluation for libraries and arguments.
//!
//! Mojang semantics: with no rules present the item applies; with rules
//! present the item applies only if the LAST matching rule allows it.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: Action,
    #[serde(default)]
    pub os: Option<RuleOs>,
    #[serde(default)]
    pub features: Option<std::collections::BTreeMap<String, bool>>,
}

/// This module's own view of an `os` rule object; metadata re-exports it as
/// [`RuleOs`] so documents parse in one place.
impl Rule {
    fn matches(&self, ctx: &EvalContext) -> bool {
        if let Some(os) = &self.os {
            if let Some(name) = &os.name {
                if !name.eq_ignore_ascii_case(ctx.os_name) {
                    return false;
                }
            }
            if let Some(arch) = &os.arch {
                if !arch.eq_ignore_ascii_case(ctx.arch) {
                    return false;
                }
            }
        }
        if let Some(features) = &self.features {
            for (feature, expected) in features {
                let actual = (ctx.feature)(*feature);
                if actual != *expected {
                    return false;
                }
            }
        }
        true
    }
}

/// The platform context a rule set is evaluated against.
pub struct EvalContext<'a> {
    /// Mojang OS name: "windows" | "linux" | "osx".
    pub os_name: &'a str,
    /// "x86_64" | "arm64" style (Mojang uses x86 in old docs; we normalize).
    pub arch: &'a str,
    pub feature: &'a dyn Fn(&str) -> bool,
}

/// Current platform context with no optional features enabled.
pub fn current_context() -> EvalContext<'static> {
    EvalContext {
        os_name: os_name(),
        arch: std::env::consts::ARCH,
        feature: &|_name| false,
    }
}

/// Map Rust's OS constant to Mojang's naming.
pub fn os_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx",
        other => other, // "linux", "freebsd", …
    }
}

/// Evaluate a rule list. `None`/empty → applies everywhere.
pub fn evaluate(rules: &[Rule], ctx: &EvalContext) -> bool {
    if rules.is_empty() {
        return true;
    }
    // Last matching rule wins (observed Mojang launcher behavior).
    let mut allowed = false;
    for rule in rules {
        if rule.matches(ctx) {
            allowed = rule.action == Action::Allow;
        }
    }
    allowed
}

// Re-exported so `metadata` can deserialize `os` objects without a cycle.
pub use os_rule::RuleOs;

mod os_rule {
    //! Separate module so the re-export reads cleanly.

    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct RuleOs {
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub arch: Option<String>,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn linux_ctx() -> EvalContext<'static> {
        EvalContext {
            os_name: "linux",
            arch: "x86_64",
            feature: &|_| false,
        }
    }

    fn windows_ctx() -> EvalContext<'static> {
        EvalContext {
            os_name: "windows",
            arch: "x86_64",
            feature: &|_| false,
        }
    }

    fn rule(action: Action, os: Option<&str>) -> Rule {
        Rule {
            action,
            os: os.map(|name| RuleOs {
                name: Some(name.to_owned()),
                arch: None,
            }),
            features: None,
        }
    }

    use Action::{Allow, Disallow};

    #[test]
    fn no_rules_means_apply() {
        assert!(evaluate(&[], &linux_ctx()));
    }

    #[test]
    fn allow_windows_excludes_linux() {
        let rules = vec![rule(Allow, Some("windows"))];
        assert!(!evaluate(&rules, &linux_ctx()));
        assert!(evaluate(&rules, &windows_ctx()));
    }

    #[test]
    fn disallow_linux_excludes_only_linux() {
        let rules = vec![rule(Disallow, Some("linux"))];
        assert!(!evaluate(&rules, &linux_ctx()));
        assert!(evaluate(&rules, &windows_ctx()));
    }

    #[test]
    fn last_matching_rule_wins() {
        let rules = vec![rule(Allow, None), rule(Disallow, Some("linux"))];
        assert!(!evaluate(&rules, &linux_ctx()));

        let flipped = vec![rule(Disallow, Some("linux")), rule(Allow, None)];
        assert!(evaluate(&flipped, &linux_ctx()));
    }

    #[test]
    fn features_gate_rules() {
        let rules = vec![Rule {
            action: Allow,
            os: None,
            features: Some(
                [("is_demo_user".to_owned(), true)].into_iter().collect(),
            ),
        }];
        let demo = EvalContext {
            os_name: "linux",
            arch: "x86_64",
            feature: &|name| name == "is_demo_user",
        };
        assert!(!evaluate(&rules, &linux_ctx()), "feature off");
        assert!(evaluate(&rules, &demo), "feature on");
    }

    #[test]
    fn current_os_maps_to_mojang_names() {
        let name = os_name();
        assert!(["windows", "linux", "osx"].contains(&name), "{name}");
    }
}
