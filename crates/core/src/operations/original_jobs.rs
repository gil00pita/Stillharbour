//! Organization job history never conveys approval or filesystem authority.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OriginalJobKind {
    Rename,
    Undo,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OriginalJobState {
    Queued,
    Running,
    CompletedVerified,
    NeedsRecovery,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginalOperationJob {
    pub id: String,
    pub plan_id: String,
    pub source_root_id: String,
    pub kind: OriginalJobKind,
    pub state: OriginalJobState,
    pub total_moves: u32,
    pub verified_moves: u32,
    pub started_at: String,
    pub updated_at: String,
    pub message: String,
    pub generated_fixture_only: bool,
}
