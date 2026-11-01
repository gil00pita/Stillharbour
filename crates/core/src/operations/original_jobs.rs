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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginalHistoryMember {
    pub id: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginalOperationHistory {
    pub job: OriginalOperationJob,
    pub plan_digest: String,
    pub reverse_of: Option<String>,
    pub members: Vec<OriginalHistoryMember>,
    pub history_only: bool,
    pub current_locations_checked: bool,
    pub execution_available: bool,
}
