//! M4A review contracts. These observations never authorize writes or recovery.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompareMode {
    Quick,
    Verified,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyncProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub supersedes: Option<String>,
    pub id: String,
    pub version: u32,
    pub catalog_id: String,
    pub label: String,
    pub source_root_id: String,
    pub source_label: String,
    pub source_display_path: String,
    pub destination_display_path: String,
    pub created_at: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonCategory {
    SourceOnly,
    DestinationOnly,
    VerifiedEqual,
    AppearsUnchangedUnverified,
    DifferentContent,
    DestinationEditedSinceLastRun,
    Uncertain,
    Blocked,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonEntryKind {
    File,
    Directory,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonSide {
    Source,
    Destination,
    Comparison,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonSummary {
    pub destination_edited_since_last_run: u32,
    pub source_only: u32,
    pub destination_only: u32,
    pub verified_equal: u32,
    pub appears_unchanged_unverified: u32,
    pub different_content: u32,
    pub uncertain: u32,
    pub blocked: u32,
}
impl ComparisonSummary {
    pub(crate) fn collect(rows: &[ComparisonEntry]) -> Self {
        let mut result = Self::default();
        for row in rows {
            let count = match row.category {
                ComparisonCategory::SourceOnly => &mut result.source_only,
                ComparisonCategory::DestinationOnly => &mut result.destination_only,
                ComparisonCategory::VerifiedEqual => &mut result.verified_equal,
                ComparisonCategory::AppearsUnchangedUnverified => {
                    &mut result.appears_unchanged_unverified
                }
                ComparisonCategory::DifferentContent => &mut result.different_content,
                ComparisonCategory::DestinationEditedSinceLastRun => {
                    &mut result.destination_edited_since_last_run
                }
                ComparisonCategory::Uncertain => &mut result.uncertain,
                ComparisonCategory::Blocked => &mut result.blocked,
            };
            *count += 1;
        }
        result
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncProposal {
    CopyAfterReview,
    KeepDestination,
    LeaveUnchanged,
    ReviewDifference,
    Recompare,
    Blocked,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DestinationCapability {
    pub read_only_comparison: bool,
    pub exclusive_create: bool,
    pub no_replace_publication: bool,
    pub retained_replacement: bool,
    pub metadata_fidelity_verified: bool,
    pub independent_recovery_verified: bool,
    pub execution_available: bool,
}
impl Default for DestinationCapability {
    fn default() -> Self {
        Self {
            read_only_comparison: true,
            exclusive_create: false,
            no_replace_publication: false,
            retained_replacement: false,
            metadata_fidelity_verified: false,
            independent_recovery_verified: false,
            execution_available: false,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyncIssue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub side: Option<ComparisonSide>,
    pub display_path: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonEntry {
    pub companions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_kind: Option<ComparisonEntryKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub destination_kind: Option<ComparisonEntryKind>,
    pub id: String,
    /// Display only. Native relative paths never round-trip through the renderer.
    pub display_path: String,
    pub source_bytes: Option<String>,
    pub destination_bytes: Option<String>,
    pub source_modified: Option<String>,
    pub destination_modified: Option<String>,
    pub category: ComparisonCategory,
    pub proposal: SyncProposal,
    pub evidence: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonState {
    Comparing,
    NeedsReview,
    Incomplete,
    Canceled,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonRun {
    pub review_plan: Option<ImmutableSyncPlan>,
    pub baseline_id: Option<String>,
    pub history_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub summary: Option<ComparisonSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub profile_id: Option<String>,
    pub id: String,
    pub policy_version: u32,
    pub catalog_id: String,
    pub source_root_id: String,
    pub source_label: String,
    pub source_display_path: String,
    pub destination_display_path: String,
    pub dataset_display_path: String,
    pub mode: CompareMode,
    pub state: ComparisonState,
    pub started_at: String,
    pub bytes_read: String,
    pub entries: u32,
    pub issues: Vec<SyncIssue>,
    pub message: String,
    pub plan_digest: Option<String>,
    pub capabilities: DestinationCapability,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonPage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub total: Option<u32>,
    pub run_id: String,
    pub entries: Vec<ComparisonEntry>,
    pub next_offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonPreview {
    pub run_id: String,
    pub entry_id: String,
    pub side: ComparisonSide,
    pub png: Vec<u8>,
    pub message: String,
}

/// Immutable read-only plan descriptor. No transfer/approval command accepts it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImmutableSyncPlan {
    pub run_id: String,
    pub catalog_id: String,
    pub source_root_id: String,
    pub policy_version: u32,
    pub digest: String,
    pub entries: u32,
    pub execution_available: bool,
}

/// Pure classification: absence is meaningful only after complete inventories.
pub(crate) fn classify(
    source: bool,
    destination: bool,
    complete: bool,
    equality: Option<bool>,
    quick_match: bool,
) -> (ComparisonCategory, SyncProposal) {
    use ComparisonCategory as C;
    use SyncProposal as P;
    match (source, destination, complete, equality, quick_match) {
        (true, true, _, Some(true), _) => (C::VerifiedEqual, P::LeaveUnchanged),
        (true, true, _, Some(false), _) => (C::DifferentContent, P::ReviewDifference),
        (true, true, _, None, true) => (C::AppearsUnchangedUnverified, P::Recompare),
        (true, false, true, _, _) => (C::SourceOnly, P::CopyAfterReview),
        (false, true, true, _, _) => (C::DestinationOnly, P::KeepDestination),
        _ => (C::Uncertain, P::Recompare),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_inventory_never_proves_absence_or_equality() {
        assert_eq!(
            classify(true, false, false, None, false).0,
            ComparisonCategory::Uncertain
        );
        assert_eq!(
            classify(false, true, false, None, false).0,
            ComparisonCategory::Uncertain
        );
        assert_eq!(
            classify(true, true, true, None, true).0,
            ComparisonCategory::AppearsUnchangedUnverified
        );
        assert_eq!(
            classify(false, true, true, None, false).1,
            SyncProposal::KeepDestination
        );
        assert!(!DestinationCapability::default().execution_available);
    }
}
