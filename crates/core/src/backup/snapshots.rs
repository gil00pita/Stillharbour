//! ID-only snapshot application contract. Secret material never crosses this API.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SnapshotDestinationInput {
    Local {
        selection_id: String,
    },
    Sftp {
        host: String,
        port: u16,
        user: String,
        remote_path: String,
        host_key: String,
    },
    Ftps {
        host: String,
        port: u16,
        user: String,
        remote_path: String,
        ca_certificate_pem: Option<String>,
    },
    Ftp {
        host: String,
        port: u16,
        user: String,
        remote_path: String,
        insecure_acknowledged: bool,
    },
    Smb {
        host: String,
        port: u16,
        user: String,
        domain: String,
        share: String,
        remote_path: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SnapshotRequest {
    Capabilities,
    Profiles,
    ProfileCreate {
        label: String,
        root_ids: Vec<String>,
        repository_credential_ref: String,
        transport_credential_ref: Option<String>,
        destination: SnapshotDestinationInput,
    },
    ProfileOpen {
        profile_id: String,
        selection_id: Option<String>,
    },
    BackupReview {
        profile_id: String,
    },
    BackupApprove {
        review_id: String,
        digest: String,
    },
    BackupStart {
        review_id: String,
        approval_id: String,
    },
    Jobs {
        profile_id: Option<String>,
    },
    Job {
        job_id: String,
    },
    Cancel {
        job_id: String,
    },
    Reconcile {
        profile_id: String,
        job_id: String,
        snapshot_id: Option<String>,
    },
    Snapshots {
        profile_id: String,
    },
    Browse {
        profile_id: String,
        snapshot_id: String,
        offset: u32,
    },
    RestoreReview {
        profile_id: String,
        snapshot_id: String,
        selection_id: String,
    },
    RestoreApprove {
        review_id: String,
        digest: String,
    },
    RestoreStart {
        review_id: String,
        approval_id: String,
    },
    Verify {
        profile_id: String,
        snapshot_id: String,
    },
    ScheduleGet {
        profile_id: String,
    },
    ScheduleDisable {
        profile_id: String,
    },
    ScheduleSave {
        profile_id: String,
        review_id: String,
        digest: String,
        interval_minutes: u32,
        timezone: String,
        enabled: bool,
        catch_up: bool,
    },
}
impl SnapshotRequest {
    pub fn selection_id(&self) -> Option<&str> {
        match self {
            Self::ProfileCreate {
                destination: SnapshotDestinationInput::Local { selection_id },
                ..
            }
            | Self::RestoreReview { selection_id, .. } => Some(selection_id),
            Self::ProfileOpen { selection_id, .. } => selection_id.as_deref(),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotCapabilities {
    pub helpers_available: bool,
    pub credentials_available: bool,
    pub backup_available: bool,
    pub restore_available: bool,
    pub schedule_available: bool,
    pub protocols: Vec<String>,
    pub unavailable_reason: Option<String>,
    pub helper_limits: String,
    pub schedule_limits: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotProfile {
    pub id: String,
    pub label: String,
    pub catalog_id: String,
    pub root_ids: Vec<String>,
    pub protocol: String,
    pub destination_display: String,
    pub repository_id: Option<String>,
    pub created_at: String,
    pub host_fingerprint: Option<String>,
    pub insecure_transport: bool,
    pub credentials_present: bool,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotReview {
    pub items: Vec<SnapshotReviewItem>,
    pub id: String,
    pub profile_id: String,
    pub action: String,
    pub digest: String,
    pub files: u32,
    pub bytes: String,
    pub root_ids: Vec<String>,
    pub catalog_included: bool,
    pub source_display_paths: Vec<String>,
    pub source_retained: bool,
    pub target_display: String,
    pub issues: Vec<String>,
    pub execution_available: bool,
    pub unavailable_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotReviewItem {
    pub display_path: String,
    pub bytes: String,
    pub full_digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotApproval {
    pub id: String,
    pub review_id: String,
    pub digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotJob {
    pub id: String,
    pub profile_id: String,
    pub action: String,
    pub state: String,
    pub phase: String,
    pub files: u32,
    pub bytes: String,
    pub snapshot_id: Option<String>,
    pub message: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub included: bool,
    pub structural_checked: bool,
    pub data_read_verified: bool,
    pub restored_compared: bool,
    pub source_retained: bool,
    pub independent_recovery_verified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRecord {
    pub id: String,
    pub profile_id: String,
    pub time: String,
    pub files: u32,
    pub bytes: String,
    pub root_ids: Vec<String>,
    pub included: bool,
    pub structural_checked: bool,
    pub data_read_verified: bool,
    pub restored_compared: bool,
    pub catalog_included: bool,
    pub source_retained: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEntry {
    pub id: String,
    pub display_path: String,
    pub kind: String,
    pub bytes: String,
    pub digest: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotBrowse {
    pub profile_id: String,
    pub snapshot_id: String,
    pub entries: Vec<SnapshotEntry>,
    pub next_offset: Option<u32>,
    pub total: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSchedule {
    pub profile_id: String,
    pub interval_minutes: u32,
    pub timezone: String,
    pub dst_policy: String,
    pub enabled: bool,
    pub catch_up: bool,
    pub next_due: String,
    pub last_run: Option<String>,
    pub fully_quit_supported: bool,
    pub approved_digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum SnapshotReply {
    Capabilities(SnapshotCapabilities),
    Profiles(Vec<SnapshotProfile>),
    Profile(SnapshotProfile),
    Review(SnapshotReview),
    Approval(SnapshotApproval),
    Job(SnapshotJob),
    Jobs(Vec<SnapshotJob>),
    Snapshots(Vec<SnapshotRecord>),
    Browse(SnapshotBrowse),
    Schedule(Option<SnapshotSchedule>),
}
