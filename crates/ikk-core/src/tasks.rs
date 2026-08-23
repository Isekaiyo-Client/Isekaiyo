//! Backend task abstraction (Phase 8 §69/§70): every long-running operation
//! (install, repair, launch preparation) runs as a Task with explicit state,
//! progress, and cancellation. The UI polls [`TaskManager::snapshot`] rather
//! than being event-flooded; a future event bridge can sit on the same type.
//!
//! Pure Rust + tests, no Tauri.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// One progress step of a task: `current/total` plus a human label
/// ("Downloading libraries 23 / 148").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: String,
    pub label: String,
    pub state: TaskState,
    pub current: u64,
    pub total: u64,
    /// Free-form status text; never contains secrets.
    pub message: String,
    /// Stable error category string when `state == Failed`.
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl TaskSnapshot {
    pub fn percent(&self) -> u8 {
        if self.total == 0 {
            return match self.state {
                TaskState::Completed => 100,
                _ => 0,
            };
        }
        ((self.current.min(self.total) * 100) / self.total).min(100) as u8
    }
}

/// Per-task live data owned by the manager.
struct TaskInner {
    snapshot: TaskSnapshot,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct TaskManager {
    tasks: Mutex<HashMap<String, Arc<Mutex<TaskInner>>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin tracking a new task. Returns a handle for cheap updates.
    pub fn start(
        &self,
        id: impl Into<String>,
        label: impl Into<String>,
        total: u64,
        cancel: Arc<AtomicBool>,
    ) -> TaskHandle {
        let id = id.into();
        let inner = Arc::new(Mutex::new(TaskInner {
            snapshot: TaskSnapshot {
                id: id.clone(),
                label: label.into(),
                state: TaskState::Running,
                current: 0,
                total,
                message: String::new(),
                error_code: None,
                error_message: None,
            },
            cancel: cancel.clone(),
        }));
        self.tasks.lock().unwrap_or_else(|p| p.into_inner()).insert(id, inner.clone());
        TaskHandle { inner }
    }

    /// Snapshot one task, or all tasks when `id` is None (oldest dropped:
    /// finished tasks are pruned beyond a small ring to bound memory).
    pub fn snapshot(&self, id: Option<&str>) -> Vec<TaskSnapshot> {
        let tasks = self.tasks.lock().unwrap_or_else(|p| p.into_inner());
        // Opportunistic pruning of terminal tasks (keep last 16).
        let mut terminal_ids: Vec<String> = tasks
            .iter()
            .filter(|(_, t)| {
                matches!(
                    t.lock().unwrap_or_else(|p| p.into_inner()).snapshot.state,
                    TaskState::Completed | TaskState::Failed | TaskState::Cancelled
                )
            })
            .map(|(k, _)| k.clone())
            .collect();
        if terminal_ids.len() > 16 {
            terminal_ids.sort();
            let drop_count = terminal_ids.len() - 16;
            for stale in terminal_ids.into_iter().take(drop_count) {
                tasks.remove(&stale);
            }
        }
        let mut out: Vec<TaskSnapshot> = tasks
            .values()
            .map(|t| t.lock().unwrap_or_else(|p| p.into_inner()).snapshot.clone())
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        match id {
            Some(wanted) => out.into_iter().filter(|s| s.id == wanted).collect(),
            None => out,
        }
    }

    /// True while no task is running for this instance prefix.
    pub fn any_running_with_prefix(&self, prefix: &str) -> bool {
        self.snapshot(None)
            .iter()
            .any(|s| s.id.starts_with(prefix) && s.state == TaskState::Running)
    }
}

/// Cheap update surface handed to running operations.
pub struct TaskHandle {
    inner: Arc<Mutex<TaskInner>>,
}

impl TaskHandle {
    pub fn progress(&self, current: u64, message: impl Into<String>) {
        let mut t = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        t.snapshot.current = current;
        t.snapshot.message = message.into();
    }

    pub fn complete(&self, message: impl Into<String>) {
        let mut t = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        t.snapshot.state = TaskState::Completed;
        t.snapshot.current = t.snapshot.total;
        t.snapshot.message = message.into();
    }

    pub fn fail(&self, code: &str, message: impl Into<String>) {
        let mut t = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        t.snapshot.state = TaskState::Failed;
        t.snapshot.error_code = Some(code.to_owned());
        t.snapshot.error_message = Some(message.into());
    }

    pub fn cancel(&self) {
        let mut t = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        t.cancel.store(true, Ordering::Relaxed);
        t.snapshot.state = TaskState::Cancelled;
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .cancel
            .load(Ordering::Relaxed)
    }

    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner()).cancel.clone()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn lifecycle_progress_and_terminal_states() {
        let tm = TaskManager::new();
        let handle = tm.start("install-1", "Install", 10, Arc::new(AtomicBool::new(false)));
        handle.progress(3, "Downloading libraries 3 / 10");
        let snap = tm.snapshot(Some("install-1"));
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].percent(), 30);
        assert_eq!(snap[0].message, "Downloading libraries 3 / 10");

        handle.complete("done");
        assert_eq!(tm.snapshot(Some("install-1"))[0].state, TaskState::Completed);
        assert_eq!(tm.snapshot(Some("install-1"))[0].percent(), 100);
    }

    #[test]
    fn failure_carries_stable_error_category() {
        let tm = TaskManager::new();
        let handle = tm.start("t", "T", 5, Arc::new(AtomicBool::new(false)));
        handle.fail("download.checksum_mismatch", "bad sha1");
        let snap = &tm.snapshot(Some("t"))[0];
        assert_eq!(snap.state, TaskState::Failed);
        assert_eq!(snap.error_code.as_deref(), Some("download.checksum_mismatch"));
    }

    #[test]
    fn cancellation_flows_through_the_flag() {
        let tm = TaskManager::new();
        let handle = tm.start("t", "T", 5, Arc::new(AtomicBool::new(false)));
        assert!(!handle.is_cancelled());
        handle.cancel();
        assert!(handle.is_cancelled());
        assert_eq!(tm.snapshot(Some("t"))[0].state, TaskState::Cancelled);
    }

    #[test]
    fn prefix_query_detects_running_work_per_instance() {
        let tm = TaskManager::new();
        let h = tm.start("install-inst-1", "I", 4, Arc::new(AtomicBool::new(false)));
        assert!(tm.any_running_with_prefix("install-inst-1"));
        h.complete("");
        assert!(!tm.any_running_with_prefix("install-inst-1"));
    }
}
