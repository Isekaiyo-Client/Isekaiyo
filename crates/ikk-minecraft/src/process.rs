//! Cross-platform process management for the game.
//!
//! - spawns with piped stdout/stderr, both merged into an append-mode log file
//!   on a reader thread (the UI never blocks; the pipes never deadlock)
//! - tracks exit status and distinguishes user-initiated stops from crashes
//! - argv is never written to logs (the access token lives there)

use ikk_core::error::{Error, ErrorCode, Result};
use std::io::Read;
use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

#[derive(Debug)]
pub struct ManagedProcess {
    child: Child,
    killed: Arc<AtomicBool>,
    log_path: PathBuf,
    reader: Option<JoinHandle<()>>,
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        // Never leave orphans behind if the launcher closes first.
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

impl ManagedProcess {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Non-blocking poll: `None` = still running, `Some(exit)` = finished.
    pub fn try_wait(&mut self) -> Result<Option<GameExit>> {
        match self.child.try_wait().map_err(|e| {
            Error::with_source(ErrorCode::IoFailure, "failed to poll game process", e)
        })? {
            None => Ok(None),
            Some(status) => {
                if let Some(handle) = self.reader.take() {
                    let _ = handle.join();
                }
                let code = status.code();
                let user_stopped = self.killed.load(Ordering::Relaxed);
                Ok(Some(GameExit {
                    exit_code: code,
                    user_stopped,
                }))
            }
        }
    }

    /// Block until exit.
    pub fn wait(mut self) -> Result<GameExit> {
        let status = self.child.wait().map_err(|e| {
            Error::with_source(ErrorCode::IoFailure, "failed to wait for game process", e)
        })?;
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
        Ok(GameExit {
            exit_code: status.code(),
            user_stopped: self.killed.load(Ordering::Relaxed),
        })
    }

    /// Request termination. Recorded as a user stop, not a crash.
    pub fn kill(&mut self) -> Result<()> {
        self.killed.store(true, Ordering::Relaxed);
        self.child.kill().map_err(|e| {
            Error::with_source(ErrorCode::IoFailure, "failed to stop game process", e)
        })?;
        Ok(())
    }
}

/// How a game run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameExit {
    /// Process exit code; `None` on signal-termination (Unix).
    pub exit_code: Option<i32>,
    /// True when Isekaiyo itself stopped the process.
    pub user_stopped: bool,
}

impl GameExit {
    /// Successful vanilla client exit is code 0.
    pub fn succeeded(&self) -> bool {
        !self.user_stopped && self.exit_code == Some(0)
    }

    pub fn category(&self) -> &'static str {
        if self.user_stopped {
            "user-stopped"
        } else if self.succeeded() {
            "completed"
        } else {
            "crashed"
        }
    }
}

/// Spawn the plan's command line. Output streams into `log_path`
/// (created/append). The working directory is the instance game dir when it
/// exists — Minecraft writes saves/screenshots relative to cwd.
pub fn spawn(
    plan: &crate::planner::LaunchPlan,
    game_dir: &Path,
    log_path: &Path,
) -> Result<ManagedProcess> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot create log dir {}", parent.display()),
                e,
            )
        })?;
    }

    let mut command = Command::new(&plan.java_executable);
    command
        .args(&plan.jvm_args)
        .arg(&plan.main_class)
        .args(&plan.game_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    if game_dir.exists() {
        command.current_dir(game_dir);
    }

    let mut child = command.spawn().map_err(|e| {
        Error::with_source(
            ErrorCode::IoFailure,
            format!(
                "failed to start {} — is the Java runtime usable?",
                plan.java_executable.display()
            ),
            e,
        )
    })?;

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot open {}", log_path.display()),
                e,
            )
        })?;

    // Merge stdout+stderr into one log via two reader threads writing into a
    // shared file handle clone.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let log_for_stderr = log_file
        .try_clone()
        .map_err(|e| Error::with_source(ErrorCode::IoFailure, "cannot share log file handle", e))?;

    let killed = Arc::new(AtomicBool::new(false));
    let reader = std::thread::spawn(move || {
        if let Some(out) = stdout {
            pump(out, &log_file);
        }
        if let Some(err) = stderr {
            pump(err, &log_for_stderr);
        }
    });

    Ok(ManagedProcess {
        child,
        killed,
        log_path: log_path.to_path_buf(),
        reader: Some(reader),
    })
}

fn pump<R: Read>(mut stream: R, mut log: &std::fs::File) {
    use std::io::Write;
    let mut buf = [0u8; 4096];
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
        let _ = log.write_all(&buf[..n]);
        let _ = log.flush();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn exit_categories_are_distinct() {
        assert_eq!(
            GameExit {
                exit_code: Some(0),
                user_stopped: false
            }
            .category(),
            "completed"
        );
        assert_eq!(
            GameExit {
                exit_code: Some(1),
                user_stopped: false
            }
            .category(),
            "crashed"
        );
        assert_eq!(
            GameExit {
                exit_code: Some(1),
                user_stopped: true
            }
            .category(),
            "user-stopped"
        );
        assert_eq!(
            GameExit {
                exit_code: None,
                user_stopped: true
            }
            .category(),
            "user-stopped"
        );
    }

    #[cfg(unix)]
    #[test]
    fn spawns_and_tracks_a_real_process() {
        let dir = std::env::temp_dir().join(format!("ikk-proc-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("game.log");

        // The planner emits `<java> <jvm…> <main_class> <game…>`; for this
        // shell stand-in the "main class" slot carries `-c`.
        let plan = crate::planner::LaunchPlan {
            java_executable: PathBuf::from("sh"),
            jvm_args: vec![],
            main_class: "-c".to_owned(),
            game_args: vec!["echo hello-from-game; sleep 5".to_owned()],
        };
        let mut proc = spawn(&plan, &dir, &log).unwrap();
        assert!(proc.pid() > 0);

        // Give the shell a beat to write output, then confirm capture.
        std::thread::sleep(std::time::Duration::from_millis(300));
        proc.kill().unwrap();
        let exit = proc.wait().unwrap();
        assert!(exit.user_stopped);

        let logged = std::fs::read_to_string(&log).unwrap();
        assert!(
            logged.contains("hello-from-game"),
            "stdout must reach the log"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
