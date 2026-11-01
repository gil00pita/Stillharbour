use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ContentAlgorithm {
    Blake3,
    Sha256,
    Sha1,
    Md5,
    DirectBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    ExactContent,
    Properties,
    NormalizedPixels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RuleLogic {
    All,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Truth {
    Match,
    Different,
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamePolicy {
    pub lowercase: bool,
    pub ignore_whitespace: bool,
    pub ignored_characters: String,
    pub unicode_nfc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PropertyRules {
    pub logic: RuleLogic,
    pub filename: bool,
    pub extension: bool,
    pub size: bool,
    pub modified: bool,
    pub created: bool,
    pub readonly: bool,
    pub content: bool,
    pub tolerance_millis: u32,
    pub name: NamePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DuplicateOptions {
    pub mode: MatchMode,
    pub algorithm: ContentAlgorithm,
    pub rules: PropertyRules,
    pub exclude_hardlinked_objects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PropertyEvidence {
    pub result: Truth,
    pub unknown_criteria: u32,
    pub matched_criteria: u32,
    pub selected_criteria: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum HealthMode {
    Quick,
    Deep,
    CompareBaseline,
    EstablishBaseline,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InspectionTask {
    Duplicates { options: DuplicateOptions },
    Health { mode: HealthMode },
    Storage {},
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectionInput {
    pub root_ids: Vec<String>,
    pub scope: super::storage::InspectionScope,
    pub task: InspectionTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum InspectionState {
    Queued,
    Running,
    CancelRequested,
    Completed,
    Partial,
    Canceled,
    Interrupted,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InspectionJob {
    pub id: String,
    pub input: InspectionInput,
    pub state: InspectionState,
    pub started_at: String,
    pub updated_at: String,
    pub finished_at: Option<String>,
    pub files_observed: u32,
    pub files_checked: u32,
    pub bytes_read: String,
    pub issues: u32,
    pub limited: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum InventoryState {
    Present,
    Missing,
    Offline,
    Inaccessible,
    ChangedDuringRead,
    UnsupportedEntry,
    Excluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ContentState {
    NotChecked,
    QuickPass,
    DeepPass,
    Unsupported,
    DecoderWarning,
    SuspectedCorrupt,
    ConfirmedDecodeFailure,
    ChangedDuringRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ChecksumState {
    NotChecked,
    NoBaseline,
    MatchesBaseline,
    ChangedSinceBaseline,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InspectionFinding {
    pub id: String,
    pub asset_id: Option<String>,
    pub root_id: String,
    pub folder_id: String,
    pub name: String,
    pub display_path: String,
    pub bytes: String,
    pub reported_allocated_bytes: Option<String>,
    pub object_id: Option<String>,
    pub links: String,
    pub is_media: bool,
    pub inventory: InventoryState,
    pub content: ContentState,
    pub checksum: ChecksumState,
    pub comparison_name: Option<String>,
    pub filesystem_modified: Option<String>,
    pub filesystem_created: Option<String>,
    pub readonly: Option<bool>,
    pub baseline_id: Option<String>,
    pub decoder_policy: Option<String>,
    pub pixel_signature: Option<String>,
    pub pixel_digest: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
    pub coverage: String,
    pub error: Option<crate::domain::AppError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateEvidence {
    ExactBytes,
    PropertyCandidate,
    NormalizedPixels,
    HardLinkAliases,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub id: String,
    pub evidence: DuplicateEvidence,
    pub member_ids: Vec<String>,
    pub member_count: u32,
    pub members_truncated: bool,
    pub mutation_available: bool,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityBaseline {
    pub id: String,
    pub file_id: String,
    pub previous_id: Option<String>,
    pub content_revision: String,
    pub algorithm: String,
    pub policy: String,
    pub digest: String,
    pub bytes: String,
    pub established_at: String,
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FindingPage {
    pub job_id: String,
    pub rows: Vec<InspectionFinding>,
    pub next: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatePage {
    pub job_id: String,
    pub groups: Vec<DuplicateGroup>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum IssueEntryKind {
    File,
    Directory,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InspectionIssue {
    pub root_id: String,
    pub display_path: String,
    pub entry_kind: IssueEntryKind,
    pub inventory: InventoryState,
    pub error: crate::domain::AppError,
}

impl InspectionInput {
    pub(crate) fn validate(&self) -> crate::domain::Result<()> {
        if self.root_ids.is_empty() || self.root_ids.len() > 64 {
            return Err(super::invalid("Choose 1–64 source libraries."));
        }
        let mut seen = std::collections::BTreeSet::new();
        for id in &self.root_ids {
            crate::operations::validate_id(id)?;
            if !seen.insert(id) {
                return Err(super::invalid("Source libraries contain a repeated ID."));
            }
        }
        if let InspectionTask::Duplicates { options } = &self.task {
            if options.mode == MatchMode::Properties {
                options.rules.validate()?;
            }
            if options.rules.name.ignored_characters.chars().count() > 128
                || options.rules.tolerance_millis > 86_400_000
            {
                return Err(super::invalid(
                    "Comparison normalization or date tolerance exceeds its bound.",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummary {
    pub total: super::FolderSize,
    pub potential_duplicate_logical_bytes: Option<String>,
    pub reclaimable_physical_bytes: Option<String>,
    pub quarantine_freed_bytes: String,
    pub accounting_incomplete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InspectionVolume {
    pub id: String,
    pub root_ids: Vec<String>,
    pub total_bytes: Option<String>,
    pub available_bytes: Option<String>,
    pub observed_at: String,
    pub coverage: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InspectionReport {
    pub volumes: Vec<InspectionVolume>,
    pub job_id: String,
    pub coverage: super::InventoryCoverage,
    pub exact_or_candidate_groups: u32,
    pub alias_groups: u32,
    pub compared_pairs: String,
    pub unknown_pairs: String,
    pub storage: Option<StorageSummary>,
    pub id3_applicability: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FolderPage {
    pub job_id: String,
    pub parent: super::FolderSize,
    pub rows: Vec<super::FolderSize>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InspectionIssuePage {
    pub job_id: String,
    pub rows: Vec<InspectionIssue>,
    pub next: Option<String>,
}
