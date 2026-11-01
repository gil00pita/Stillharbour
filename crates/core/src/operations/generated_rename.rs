//! Typed instrumentation contract. It conveys IDs, never filesystem paths or
//! authority over existing libraries. Runtime is excluded from normal builds.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneratedRenameRequest {
    Create {},
    CreateFromTemplate {
        template: crate::organization::OrganizationTemplate,
    },
    Inspect {},
    ReopenRetained {},
    ReviewReverse {},
    Approve {
        plan_id: String,
        acknowledged: bool,
    },
    Start {
        plan_id: String,
        ticket: String,
    },
}
#[derive(Clone, Serialize, Deserialize, TS, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedRenameState {
    Review,
    NeedsContinuation,
    NeedsCatalogCommit,
    Completed,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedRenameMember {
    pub id: String,
    pub before: String,
    pub after: String,
    pub observed: String,
    pub catalog_asset_id: String,
    pub catalog_path: String,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedRenameReview {
    pub plan_id: String,
    pub digest: String,
    pub reverse: bool,
    pub state: GeneratedRenameState,
    pub completed_steps: u32,
    pub total_steps: u32,
    pub pending_intent: bool,
    pub catalog_committed: bool,
    pub jobs: Vec<super::original_jobs::OriginalOperationJob>,
    pub members: Vec<GeneratedRenameMember>,
    pub ticket: Option<String>,
    pub production_authorization: bool,
    pub independent_recovery_proven: bool,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneratedWorkerRequest {
    Status {},
    Stop { job_id: String },
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedWorkerStatus {
    pub scope_created: bool,
    pub busy: bool,
    pub stop_requested: bool,
    pub completed_moves: u32,
    pub total_moves: u32,
    pub job: Option<super::original_jobs::OriginalOperationJob>,
    pub error: Option<crate::domain::AppError>,
}
