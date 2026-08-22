//! Task/event types shared by services and the UI event stream.
//!
//! Every long-running operation (download, launch, install) emits these so the
//! UI renders real progress and failures carry a stable category.

use crate::error::ErrorCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskEvent {
    Started { task_id: String, label: String },
    Progress { task_id: String, done: u64, total: Option<u64> },
    Succeeded { task_id: String },
    Failed { task_id: String, error_code: String, message: String },
    Cancelled { task_id: String },
}

impl TaskEvent {
    pub fn failed(task_id: impl Into<String>, code: ErrorCode, message: impl Into<String>) -> Self {
        TaskEvent::Failed { task_id: task_id.into(), error_code: code.as_str().to_owned(), message: message.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_events_roundtrip_through_json() {
        let ev = TaskEvent::failed("dl-1", ErrorCode::ChecksumMismatch, "sha1 mismatch for client.jar");
        let json = serde_json::to_string(&ev).expect("serialization is total for owned data");
        let back: TaskEvent = serde_json::from_str(&json).expect("we just produced this shape");
        assert_eq!(back, ev);
        assert!(json.contains("download.checksum_mismatch"));
    }
}
