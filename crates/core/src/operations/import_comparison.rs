//! Read-only import comparison reports are observations, never skip/removal grants.
use crate::domain::AppError;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportComparisonInput {
    pub plan_id: String,
    pub manifest_id: String,
    pub scope_root_ids: Vec<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportComparisonState {
    Queued,
    Running,
    CancelRequested,
    Completed,
    CompletedWithIssues,
    Canceled,
    Failed,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportComparisonStage {
    Admitting,
    Sampling,
    Hashing,
    Comparing,
    Finished,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportFileComparison {
    NoIndexedMatch,
    ExactBytesCompared,
    SameNativeObject,
    NeedsAttention,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportMatch {
    pub asset_id: String,
    pub root_id: String,
    pub relative_path: String,
    pub display_truncated: bool,
    pub digest: String,
    pub same_native_object: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportComparisonFile {
    pub file_id: String,
    pub source_path: String,
    pub bytes: String,
    pub state: ImportFileComparison,
    pub matches: Vec<ImportMatch>,
    pub further_candidates_uncompared: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportComparisonIssue {
    pub root_id: String,
    pub asset_id: Option<String>,
    pub error: AppError,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportComparisonReport {
    pub policy: String,
    pub sample_policy: String,
    pub algorithm: String,
    pub observed_at: String,
    pub indexed_present_files_only: bool,
    pub all_scope_available: bool,
    pub excluded_or_missing_files: u32,
    pub bundle_review_required: bool,
    pub mutation_authorized: bool,
    pub files: Vec<ImportComparisonFile>,
    pub issues: Vec<ImportComparisonIssue>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportComparisonJob {
    pub id: String,
    pub plan_id: String,
    pub manifest_id: String,
    pub source_root_id: String,
    pub scope_root_ids: Vec<String>,
    pub state: ImportComparisonState,
    pub stage: ImportComparisonStage,
    pub candidate_files: u32,
    pub processed_candidates: u32,
    pub compared_source_files: u32,
    pub total_source_files: u32,
    pub bytes_read: String,
    pub report: Option<ImportComparisonReport>,
    pub error: Option<AppError>,
}
