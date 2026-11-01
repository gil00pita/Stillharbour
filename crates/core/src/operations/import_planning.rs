//! Explicit whole-bundle decisions. Retained plans never restore approvals.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportBundleChoice {
    pub file_id: String,
    pub existing_group_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportPlanInput {
    pub comparison_job_id: String,
    pub choices: Vec<ImportBundleChoice>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportSkipFile {
    pub file_id: String,
    pub keeper_asset_id: String,
    pub keeper_root_id: String,
    pub keeper_relative_path: String,
    pub digest: String,
    pub same_native_object: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
    pub id: String,
    pub plan_id: String,
    pub manifest_id: String,
    pub manifest_digest: String,
    pub digest: String,
    pub created_at: String,
    pub source_root_id: String,
    pub scope_root_ids: Vec<String>,
    pub choices: Vec<ImportBundleChoice>,
    pub copy_file_ids: Vec<String>,
    pub skip_files: Vec<ImportSkipFile>,
    pub sources_retained: bool,
    pub execution_available: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportCopyReceipt {
    pub job_id: String,
    pub plan_id: String,
    pub manifest_id: String,
    pub import_plan_id: String,
    pub import_digest: String,
    pub copy_review_id: String,
    pub copied_file_ids: Vec<String>,
    pub skipped_files: Vec<ImportSkipFile>,
    pub keeper_bytes_read: String,
    pub verified_at: String,
    pub sources_retained: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutputIndex {
    pub job_id: String,
    pub root: crate::library::model::LibraryRoot,
    pub scan: Option<crate::library::model::ScanJob>,
    pub already_registered: bool,
    pub original_mutations_enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportSkipApprovalInput {
    pub plan_id: String,
    pub import_plan_id: String,
    pub digest: String,
    pub sources_retained_acknowledged: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportSkipApproval {
    pub id: String,
    pub import_plan_id: String,
    pub digest: String,
    pub expires_in_seconds: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportSkipStartInput {
    pub plan_id: String,
    pub import_plan_id: String,
    pub digest: String,
    pub approval_id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportSessionState {
    Queued,
    Running,
    CancelRequested,
    Completed,
    Canceled,
    Failed,
    NeedsRecovery,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportSession {
    pub id: String,
    pub plan_id: String,
    pub import_plan_id: String,
    pub digest: String,
    pub state: ImportSessionState,
    pub total_files: u32,
    pub verified_files: u32,
    pub bytes_read: String,
    pub sources_retained: bool,
    pub output_created: bool,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub error: Option<crate::domain::AppError>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlanSummary {
    pub id: String,
    pub manifest_id: String,
    pub digest: String,
    pub created_at: String,
    pub copy_files: u32,
    pub skip_files: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportHistory {
    pub plans: Vec<ImportPlanSummary>,
    pub sessions: Vec<ImportSession>,
}
