//! Typed readable-copy application contract. Native paths and approvals remain backend-only.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyncRequest {
    Capabilities {},
    ReviewAppend {
        run_id: String,
        keep_destination: Vec<String>,
    },
    ReviewReplacement {
        run_id: String,
        entry_id: String,
        recovery_evidence_id: String,
    },
    ReviewRecoveryBackup {
        run_id: String,
        entry_id: String,
        profile_id: String,
    },
    Versions {
        job_id: String,
    },
    ReviewRestoreRetained {
        job_id: String,
    },
    Review {
        review_id: String,
    },
    Approve {
        review_id: String,
        acknowledge_uncopied: bool,
        acknowledge_metadata_limits: bool,
    },
    Start {
        approval_id: String,
    },
    List {},
    Job {
        job_id: String,
    },
    Pause {
        job_id: String,
    },
    Resume {
        job_id: String,
    },
    Cancel {
        job_id: String,
    },
    RecoveryReview {
        job_id: String,
    },
    SetLimit {
        bytes_per_second: String,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncAction {
    PrepareNames,
    CopyFiles,
    ResumeFiles,
    ReplaceRetaining,
    RestoreRetained,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Queued,
    Running,
    Paused,
    CancelRequested,
    NeedsReview,
    NeedsRecovery,
    CompletedVerified,
    CompletedPartial,
    Canceled,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncCapabilities {
    pub execution_available: bool,
    pub retained_replacement_available: bool,
    pub reason: String,
    pub io_scope: String,
    pub bytes_per_second: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncReview {
    pub id: String,
    pub job_id: Option<String>,
    pub run_id: String,
    pub action: SyncAction,
    pub digest: String,
    pub source_root_id: String,
    pub source_display_path: String,
    pub dataset_display_path: String,
    pub copy_files: u32,
    pub copy_bytes: String,
    pub retained_destination_files: u32,
    pub uncopied_source_revisions: u32,
    pub available_bytes: Option<String>,
    pub required_bytes: String,
    pub metadata_warning: String,
    pub execution_available: bool,
    pub reason: String,
    pub approval_lifetime_seconds: u32,
    pub items: Vec<SyncPlanItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncPlanItem {
    pub entry_id: String,
    pub display_path: String,
    pub action: SyncItemAction,
    pub source_bytes: Option<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncItemAction {
    CopyNew,
    KeepDestination,
    LeaveUnchanged,
    ReplaceRetaining,
    RestoreToNewOutput,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncRetainedVersion {
    pub id: String,
    pub job_id: String,
    pub display_path: String,
    pub bytes: String,
    pub full_digest: String,
    pub retained: bool,
    pub restore_available: bool,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncApproval {
    pub id: String,
    pub review_id: String,
    pub action: SyncAction,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncJob {
    pub id: String,
    pub run_id: String,
    pub source_root_id: String,
    pub action: SyncAction,
    pub state: SyncState,
    pub phase: String,
    pub created_at: String,
    pub updated_at: String,
    pub source_display_path: String,
    pub dataset_display_path: String,
    pub total_files: u32,
    pub verified_files: u32,
    pub copy_bytes: String,
    pub uncopied_source_revisions: u32,
    pub retained_destination_files: u32,
    pub ready_review_id: Option<String>,
    pub bytes_per_second: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SyncReply {
    Capabilities(SyncCapabilities),
    Review(SyncReview),
    Approval(SyncApproval),
    Job(SyncJob),
    Jobs(Vec<SyncJob>),
    Versions(Vec<SyncRetainedVersion>),
    RecoveryBackup(super::snapshots::SnapshotReview),
}

/// Backend-only exact-version provider contract. Never accepted from IPC or a
/// restored catalog as an execution approval.
#[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
#[derive(Clone, Debug)]
pub(crate) struct RecoveryFile {
    pub catalog_id: String,
    pub root_id: String,
    pub native_root: Vec<u8>,
    pub root_identity: crate::filesystem::FileIdentity,
    pub root_volume: String,
    pub relative_path: Vec<u8>,
    pub identity: crate::filesystem::FileIdentity,
    pub revision: String,
    pub bytes: u64,
    pub full_digest: String,
    pub essential_metadata_digest: Option<String>,
}
#[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
#[derive(Clone, Debug)]
pub(crate) struct RecoveryProof {
    pub evidence_id: String,
    pub scope_digest: String,
    pub independent_failure_domain: bool,
    pub essential_metadata_verified: bool,
}
