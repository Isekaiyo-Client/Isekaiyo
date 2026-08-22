//! The launch state machine. One enum, explicit transitions, no booleans.

use ikk_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchPhase {
    /// Nothing happening.
    Idle,
    Preparing,
    ResolvingMetadata,
    Downloading,
    Verifying,
    ResolvingJava,
    BuildingPlan,
    Starting,
    Running,
    Stopping,
    Completed,
    Failed,
    Cancelled,
}

impl LaunchPhase {
    /// Whether another phase may follow this one. Terminal phases require an
    /// explicit reset to `Idle` — no accidental re-entry into `Running` from
    /// `Completed`.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            LaunchPhase::Completed | LaunchPhase::Failed | LaunchPhase::Cancelled
        )
    }

    /// Allowed successor states. Anything else is a programming error and the
    /// caller should treat it as a bug (the launcher logs and refuses).
    pub fn can_transition_to(self, next: LaunchPhase) -> bool {
        use LaunchPhase::*;
        match self {
            Idle => matches!(next, Preparing),
            Preparing => matches!(next, ResolvingMetadata | Failed | Cancelled),
            ResolvingMetadata => matches!(next, Downloading | ResolvingJava | Failed | Cancelled),
            Downloading => matches!(next, Verifying | Failed | Cancelled),
            Verifying => matches!(next, ResolvingJava | Failed | Cancelled),
            ResolvingJava => matches!(next, BuildingPlan | Failed | Cancelled),
            BuildingPlan => matches!(next, Starting | Failed | Cancelled),
            Starting => matches!(next, Running | Failed | Cancelled),
            Running => matches!(next, Stopping | Completed | Failed),
            Stopping => matches!(next, Completed | Failed | Cancelled),
            Completed | Failed | Cancelled => matches!(next, Idle),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LaunchPhase::Idle => "idle",
            LaunchPhase::Preparing => "preparing",
            LaunchPhase::ResolvingMetadata => "resolving-metadata",
            LaunchPhase::Downloading => "downloading",
            LaunchPhase::Verifying => "verifying",
            LaunchPhase::ResolvingJava => "resolving-java",
            LaunchPhase::BuildingPlan => "building-plan",
            LaunchPhase::Starting => "starting",
            LaunchPhase::Running => "running",
            LaunchPhase::Stopping => "stopping",
            LaunchPhase::Completed => "completed",
            LaunchPhase::Failed => "failed",
            LaunchPhase::Cancelled => "cancelled",
        }
    }
}

/// A tracked phase with validated transitions. Cheap to clone; lives behind
/// a mutex in the application layer.
#[derive(Debug, Clone)]
pub struct PhaseTracker {
    phase: LaunchPhase,
}

impl PhaseTracker {
    pub fn new() -> Self {
        Self {
            phase: LaunchPhase::Idle,
        }
    }

    pub fn phase(&self) -> LaunchPhase {
        self.phase
    }

    /// Transition, or return the current phase unchanged with an error naming
    /// the illegal move.
    pub fn transition(&mut self, next: LaunchPhase) -> Result<()> {
        if self.phase == next {
            return Ok(()); // idempotent repeats are fine (e.g. progress ticks)
        }
        if self.phase.can_transition_to(next) {
            self.phase = next;
            Ok(())
        } else {
            Err(ikk_core::Error::new(
                ikk_core::ErrorCode::Internal,
                format!(
                    "illegal launch state transition {} → {}",
                    self.phase.as_str(),
                    next.as_str()
                ),
            ))
        }
    }

    pub fn reset(&mut self) {
        self.phase = LaunchPhase::Idle;
    }
}

impl Default for PhaseTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn happy_path_transitions_cleanly() {
        let mut t = PhaseTracker::new();
        for step in [
            LaunchPhase::Preparing,
            LaunchPhase::ResolvingMetadata,
            LaunchPhase::Downloading,
            LaunchPhase::Verifying,
            LaunchPhase::ResolvingJava,
            LaunchPhase::BuildingPlan,
            LaunchPhase::Starting,
            LaunchPhase::Running,
            LaunchPhase::Completed,
        ] {
            t.transition(step).unwrap();
        }
        assert!(t.phase().is_terminal());
        t.transition(LaunchPhase::Idle).unwrap();
        assert_eq!(t.phase(), LaunchPhase::Idle);
    }

    #[test]
    fn failure_can_come_from_any_active_phase() {
        // Valid prefixes of the pipeline, ending at various active phases.
        let paths: Vec<Vec<LaunchPhase>> = vec![
            vec![LaunchPhase::Preparing],
            vec![
                LaunchPhase::Preparing,
                LaunchPhase::ResolvingMetadata,
                LaunchPhase::Downloading,
            ],
            vec![
                LaunchPhase::Preparing,
                LaunchPhase::ResolvingMetadata,
                LaunchPhase::Downloading,
                LaunchPhase::Verifying,
                LaunchPhase::ResolvingJava,
                LaunchPhase::BuildingPlan,
                LaunchPhase::Starting,
            ],
            vec![
                LaunchPhase::Preparing,
                LaunchPhase::ResolvingMetadata,
                LaunchPhase::Downloading,
                LaunchPhase::Verifying,
                LaunchPhase::ResolvingJava,
                LaunchPhase::BuildingPlan,
                LaunchPhase::Starting,
                LaunchPhase::Running,
            ],
        ];
        for path in paths {
            let mut t = PhaseTracker::new();
            for step in path {
                t.transition(step).unwrap();
            }
            t.transition(LaunchPhase::Failed).unwrap();
            assert_eq!(t.phase(), LaunchPhase::Failed);
        }
    }

    #[test]
    fn illegal_jumps_are_rejected() {
        let mut t = PhaseTracker::new();
        // Idle → Running skips everything.
        assert!(t.transition(LaunchPhase::Running).is_err());
        t.transition(LaunchPhase::Preparing).unwrap();
        // Preparing → Completed skips the pipeline.
        assert!(t.transition(LaunchPhase::Completed).is_err());
        assert_eq!(t.phase(), LaunchPhase::Preparing);
    }

    #[test]
    fn terminal_states_require_reset() {
        let mut t = PhaseTracker::new();
        t.transition(LaunchPhase::Preparing).unwrap();
        t.transition(LaunchPhase::Failed).unwrap();
        // Failed → Running directly is illegal…
        assert!(t.transition(LaunchPhase::Downloading).is_err());
        // …but reset then restart works.
        t.transition(LaunchPhase::Idle).unwrap();
        t.transition(LaunchPhase::Preparing).unwrap();
        assert_eq!(t.phase(), LaunchPhase::Preparing);
    }
}
