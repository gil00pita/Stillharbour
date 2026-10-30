use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Default photo-file discovery preset, not a decoder-support promise.
pub const DEFAULT_PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "jfif", "heic", "heif", "hif", "png", "tif", "tiff", "webp", "avif", "gif",
    "bmp", "dng", "cr3", "cr2", "crw", "nef", "nrw", "arw", "sr2", "srf", "raf", "orf", "rw2",
    "raw", "pef", "srw", "x3f", "3fr", "fff", "iiq", "rwl",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RootMode {
    Referenced,
    Managed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanFilter {
    pub version: u32,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
    pub excluded_folders: Vec<String>,
    pub extensions: Vec<String>,
    pub include_hidden_files: bool,
    pub include_hidden_folders: bool,
    pub min_bytes: Option<String>,
    pub max_bytes: Option<String>,
    pub from_date: Option<String>,
    pub through_date: Option<String>,
}

impl Default for ScanFilter {
    fn default() -> Self {
        Self {
            version: 1,
            include_globs: vec![],
            exclude_globs: vec![],
            excluded_folders: vec![],
            extensions: vec![],
            include_hidden_files: false,
            include_hidden_folders: false,
            min_bytes: None,
            max_bytes: None,
            from_date: None,
            through_date: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRoot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub preparing_previews: Option<bool>,
    pub id: String,
    pub label: String,
    pub display_path: String,
    pub mode: RootMode,
    pub is_base: bool,
    pub availability: String,
    pub filter: ScanFilter,
    pub last_scan: Option<String>,
    pub file_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    #[default]
    Scan,
    Thumbnails,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScanJob {
    #[serde(default)]
    #[ts(optional)]
    pub kind: Option<JobKind>,
    pub id: String,
    pub root_id: String,
    pub state: String,
    pub discovered: u32,
    pub indexed: u32,
    pub skipped: u32,
    pub errors: u32,
    pub message: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RootMonitor {
    pub root_id: String,
    pub mode: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LibraryChangeKind {
    Added,
    Moved,
    Updated,
    Removed,
    Reappeared,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryChange {
    pub version: u32,
    pub sequence: String,
    pub root_id: String,
    pub root_label: String,
    pub asset_id: Option<String>,
    pub entry_kind: String,
    pub kind: LibraryChangeKind,
    pub previous_native: Option<String>,
    pub native_path: Option<String>,
    pub previous_path: Option<String>,
    pub path: Option<String>,
    pub previous_revision: Option<String>,
    pub revision: Option<String>,
    pub observed_at: String,
    pub scan_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryChangePage {
    pub changes: Vec<LibraryChange>,
    pub next_before: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub monitoring: Option<Vec<RootMonitor>>,
    pub roots: Vec<LibraryRoot>,
    pub jobs: Vec<ScanJob>,
    pub revision: String,
    pub preview_generation: String,
    pub metadata_available: bool,
    pub preview_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JobDetail {
    pub job: ScanJob,
    pub issues: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntry {
    pub path: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub defer_pending_previews: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub directory_id: Option<String>,
    pub root_id: Option<String>,
    pub folder: String,
    pub name: String,
    pub from_date: String,
    pub through_date: String,
    pub min_bytes: String,
    pub max_bytes: String,
    pub extension: String,
    pub favorites_only: bool,
    pub tag: String,
    pub sort: String,
    pub include_missing: bool,
}

impl Default for AssetQuery {
    fn default() -> Self {
        Self {
            defer_pending_previews: None,
            directory_id: None,
            root_id: None,
            folder: String::new(),
            name: String::new(),
            from_date: String::new(),
            through_date: String::new(),
            min_bytes: String::new(),
            max_bytes: String::new(),
            extension: String::new(),
            favorites_only: false,
            tag: String::new(),
            sort: "capture".into(),
            include_missing: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetSummary {
    pub id: String,
    pub root_id: String,
    pub name: String,
    pub relative_path: String,
    pub folder: String,
    pub extension: String,
    pub detected_format: Option<String>,
    pub bytes: String,
    pub date: String,
    pub date_source: String,
    pub raw_date: Option<String>,
    pub offset: Option<String>,
    pub modified_at: Option<String>,
    pub born_at: Option<String>,
    pub added_at: String,
    pub revision: String,
    pub presence: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub camera: Option<String>,
    pub metadata_state: String,
    pub metadata_error: Option<String>,
    pub preview_state: String,
    pub preview_error: Option<String>,
    pub favorite: bool,
    pub rating: u8,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetDetail {
    pub asset: AssetSummary,
    pub root: LibraryRoot,
    pub raw_metadata: Option<String>,
    pub companions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetNeighbors {
    pub previous: Option<AssetSummary>,
    pub next: Option<AssetSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetPage {
    pub assets: Vec<AssetSummary>,
    pub next_cursor: Option<String>,
    pub total: u32,
    pub revision: String,
}

/// Wire version 1 removes repeated object keys without dropping table/viewer fields.
/// Keep field order stable; changing it requires a new format version and decoder.
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct PackedAsset(
    pub String,         // id
    pub String,         // root_id
    pub String,         // name
    pub String,         // relative_path
    pub String,         // folder
    pub String,         // extension
    pub Option<String>, // detected_format
    pub String,         // bytes
    pub String,         // date
    pub String,         // date_source
    pub Option<String>, // raw_date
    pub Option<String>, // offset
    pub Option<String>, // modified_at
    pub Option<String>, // born_at
    pub String,         // added_at
    pub String,         // revision
    pub String,         // presence
    pub Option<u32>,    // width
    pub Option<u32>,    // height
    pub Option<String>, // camera
    pub String,         // metadata_state
    pub Option<String>, // metadata_error
    pub String,         // preview_state
    pub Option<String>, // preview_error
    pub bool,           // favorite
    pub u8,             // rating
    pub Vec<String>,    // tags
);
impl From<AssetSummary> for PackedAsset {
    fn from(asset: AssetSummary) -> Self {
        Self(
            asset.id,
            asset.root_id,
            asset.name,
            asset.relative_path,
            asset.folder,
            asset.extension,
            asset.detected_format,
            asset.bytes,
            asset.date,
            asset.date_source,
            asset.raw_date,
            asset.offset,
            asset.modified_at,
            asset.born_at,
            asset.added_at,
            asset.revision,
            asset.presence,
            asset.width,
            asset.height,
            asset.camera,
            asset.metadata_state,
            asset.metadata_error,
            asset.preview_state,
            asset.preview_error,
            asset.favorite,
            asset.rating,
            asset.tags,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackedAssetPage {
    pub format: u8,
    pub rows: Vec<PackedAsset>,
    pub next_cursor: Option<String>,
    pub total: u32,
    pub revision: String,
}
impl From<AssetPage> for PackedAssetPage {
    fn from(page: AssetPage) -> Self {
        Self {
            format: 1,
            rows: page.assets.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
            total: page.total,
            revision: page.revision,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DateBucket {
    pub year: String,
    pub count: u32,
    pub months: Vec<MonthBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonthBucket {
    pub period: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SelectionToken {
    pub id: String,
    pub count: u32,
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationPatch {
    pub favorite: Option<bool>,
    pub rating: Option<u8>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssetSelection {
    Ids { ids: Vec<String> },
    Token { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThumbnailKey {
    pub asset_id: String,
    pub revision: String,
    pub edge: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fallback_edge: Option<u32>,
    pub asset_id: String,
    pub revision: String,
    pub edge: u32,
    pub state: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetSeekPosition {
    Before,
    At,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDirectory {
    pub id: String,
    pub name: String,
    pub state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DirectoryEntry {
    Folder {
        directory: CatalogDirectory,
    },
    File {
        asset: Box<AssetSummary>,
        media: bool,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryPage {
    pub directory: CatalogDirectory,
    pub ancestors: Vec<CatalogDirectory>,
    pub entries: Vec<DirectoryEntry>,
    pub total: u32,
    pub previous_cursor: Option<String>,
    pub next_cursor: Option<String>,
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaintenanceTarget {
    Library { root_id: String },
    PreviewCache,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MaintenancePreview {
    pub id: String,
    pub target: MaintenanceTarget,
    pub root_label: Option<String>,
    pub root_display_path: Option<String>,
    pub asset_count: u32,
    pub cached_previews: u32,
    pub cache_bytes: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceResult {
    pub removed_root_id: Option<String>,
    pub cleared_entries: u32,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LibraryRequest {
    Changes {
        before: Option<String>,
        limit: u32,
    },
    BrowseDirectory {
        root_id: String,
        directory_id: Option<String>,
        name: String,
        cursor: Option<String>,
        limit: u32,
    },
    PrepareMaintenance {
        target: MaintenanceTarget,
    },
    ApplyMaintenance {
        plan_id: String,
    },
    SeekAsset {
        query: AssetQuery,
        asset_id: String,
        position: AssetSeekPosition,
        limit: u32,
    },
    Status {},
    SaveAndScan {
        selection_id: String,
        label: String,
        mode: RootMode,
        filter: ScanFilter,
    },
    Scan {
        root_id: String,
        filter: ScanFilter,
    },
    GenerateThumbnails {
        root_id: String,
    },
    Cancel {
        job_id: String,
    },
    JobDetails {
        job_id: String,
    },
    Folders {
        root_id: String,
    },
    Tags {},
    Relink {
        root_id: String,
        selection_id: String,
    },
    UpdateMode {
        root_id: String,
        mode: RootMode,
    },
    Query {
        query: AssetQuery,
        cursor: Option<String>,
        limit: u32,
    },
    Detail {
        asset_id: String,
    },
    Neighbors {
        query: AssetQuery,
        asset_id: String,
    },
    Seek {
        query: AssetQuery,
        year: String,
        limit: u32,
    },
    SeekDate {
        query: AssetQuery,
        period: String,
        limit: u32,
    },
    DateBuckets {
        query: AssetQuery,
    },
    FreezeSelection {
        query: AssetQuery,
    },
    Annotate {
        selection: AssetSelection,
        patch: AnnotationPatch,
    },
    AddTags {
        selection: AssetSelection,
        tags: Vec<String>,
    },
    Thumbnails {
        keys: Vec<ThumbnailKey>,
        generation: String,
        #[serde(default)]
        #[ts(optional)]
        cache_only: Option<bool>,
    },
    FilterPreview {
        filter: ScanFilter,
        paths: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum LibraryResponse {
    Changes(LibraryChangePage),
    Directory(DirectoryPage),
    MaintenancePreview(MaintenancePreview),
    MaintenanceDone(MaintenanceResult),
    Status(LibraryStatus),
    Job(ScanJob),
    Page(PackedAssetPage),
    Detail(Box<AssetDetail>),
    Neighbors(Box<AssetNeighbors>),
    Buckets(Vec<DateBucket>),
    Selection(SelectionToken),
    Updated(u32),
    Thumbnails(Vec<ThumbnailResult>),
    FilterPreview(Vec<String>),
    JobDetails(JobDetail),
    Folders(Vec<FolderEntry>),
    Tags(Vec<String>),
    Done,
}

pub const LIBRARY_COMMAND: &str = "library_request";

#[cfg(test)]
mod page_transport_tests {
    use super::*;
    #[test]
    fn packed_page_matches_shared_wire_v1_fixture_without_losing_fields() {
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/catalog-page-v1.json"
        ))
        .unwrap();
        let page: AssetPage = serde_json::from_value(golden["page"].clone()).unwrap();
        assert_eq!(
            serde_json::to_value(PackedAssetPage::from(page)).unwrap(),
            golden["packed"]
        );
    }
}

#[cfg(test)]
mod cache_request_tests {
    use super::*;
    #[test]
    fn legacy_thumbnail_requests_default_to_work_and_cache_mode_is_typed() {
        let legacy = serde_json::json!({"kind":"thumbnails", "keys":[], "generation":"1"});
        assert!(matches!(
            serde_json::from_value::<LibraryRequest>(legacy.clone()).unwrap(),
            LibraryRequest::Thumbnails {
                cache_only: None,
                ..
            }
        ));
        let mut cached = legacy;
        cached["cache_only"] = serde_json::json!(true);
        assert!(matches!(
            serde_json::from_value::<LibraryRequest>(cached.clone()).unwrap(),
            LibraryRequest::Thumbnails {
                cache_only: Some(true),
                ..
            }
        ));
        cached["cache_only"] = serde_json::json!("true");
        assert!(serde_json::from_value::<LibraryRequest>(cached).is_err());
    }
}
