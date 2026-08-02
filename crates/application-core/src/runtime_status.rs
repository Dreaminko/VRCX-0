use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeOperationStatus {
    Pending,
    Scheduled,
    Running,
    Ready,
    Ok,
    Idle,
    Completed,
    Error,
    Unavailable,
    Checkpoint,
    Persisted,
    Ignored,
    Sent,
    Skipped,
    Observed,
    Stale,
    Partial,
}
