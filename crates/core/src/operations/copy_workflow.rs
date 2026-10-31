//! Review/control contracts. Native paths, plans and approval proofs stay in
//! the backend; response paths are display data only.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyInspection {
    pub review_id: String,
    pub states: Vec<CopyInspectionState>,
    pub all_published_bytes_verified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopyInspectionState {
    NotStarted,
    PreparedEmptyStage,
    RetainedPartialStage,
    StagedVerified,
    PublishedVerified,
    NeedsAttention,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyWorkflowCapabilities {
    pub available: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopyAction {
    PrepareNames,
    CopyFiles,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyReviewFile {
    pub id: String,
    pub source_path: String,
    pub target_path: String,
    pub bytes: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyActionReview {
    pub id: String,
    pub plan_id: String,
    pub manifest_id: String,
    pub digest: String,
    pub action: CopyAction,
    pub source_root_id: String,
    pub destination_display_path: String,
    pub output_folder: String,
    pub files: Vec<CopyReviewFile>,
    pub assessment: super::CopyAssessment,
    pub sources_retained: bool,
    pub independent_backup: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyActionApproval {
    pub id: String,
    pub review_id: String,
    pub action: CopyAction,
    pub expires_in_seconds: u32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopyJobState {
    Queued,
    Running,
    PauseRequested,
    CancelRequested,
    Prepared,
    CompletedVerified,
    CanceledWithPartialCopies,
    NeedsAttention,
    NeedsRecovery,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyWorkflowJob {
    pub id: String,
    pub plan_id: String,
    pub review_id: String,
    pub source_root_id: String,
    pub action: CopyAction,
    pub state: CopyJobState,
    pub total_files: u32,
    pub verified_files: u32,
    pub ready_review_id: Option<String>,
    pub message: String,
    pub error: Option<crate::domain::AppError>,
    pub started_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyApprovalInput {
    pub review_id: String,
    pub digest: String,
    pub action: CopyAction,
    pub fidelity_limits_acknowledged: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyStartInput {
    pub review_id: String,
    pub digest: String,
    pub approval_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyIoStatus {
    pub aggregate_bytes_per_second: String,
    pub byte_io_paused: bool,
    pub burst_bytes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyRecoveryInspection {
    pub job_id: String,
    pub source_root_id: String,
    pub destination_display_path: String,
    pub output_folder: String,
    pub files: Vec<CopyReviewFile>,
    pub inspection: CopyInspection,
    pub issues: Vec<CopyInspectionIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyInspectionIssue {
    pub file_id: String,
    pub error: crate::domain::AppError,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CopyRecordedPhase {
    NotStarted,
    IntentRecorded,
    StageRecorded,
    StageFlushed,
    StageVerified,
    PublicationIntent,
    CompletedRecorded,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyContinuationFile {
    pub file: CopyReviewFile,
    pub recorded_phase: CopyRecordedPhase,
    pub retained_stage_bytes: Option<String>,
    pub output_present: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyContinuationReview {
    pub id: String,
    pub prior_job_id: String,
    pub plan_id: String,
    pub manifest_id: String,
    pub digest: String,
    pub source_root_id: String,
    pub destination_display_path: String,
    pub output_folder: String,
    pub files: Vec<CopyContinuationFile>,
    pub assessment: super::CopyAssessment,
    pub recorded_complete: bool,
    pub sources_retained: bool,
    pub independent_backup: bool,
}
