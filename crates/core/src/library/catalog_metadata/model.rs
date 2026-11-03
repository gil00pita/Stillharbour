use super::super::model::{AssetQuery, AssetSelection};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetNotes {
    pub asset_id: String,
    pub version: u32,
    pub caption: String,
    pub rejected: bool,
    pub observed_keywords: Vec<String>,
    pub keywords_available: bool,
    pub keywords_truncated: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollectionKind {
    Album,
    Search,
}
impl CollectionKind {
    pub(super) fn text(self) -> &'static str {
        match self {
            Self::Album => "album",
            Self::Search => "search",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCollection {
    pub id: String,
    pub version: u32,
    pub kind: CollectionKind,
    pub name: String,
    pub query: Option<AssetQuery>,
    pub archived: bool,
    pub member_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CollectionMembership {
    pub collection: CatalogCollection,
    pub changed: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TagCategory {
    pub id: String,
    pub version: u32,
    pub label: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TagEditReview {
    pub id: String,
    pub digest: String,
    pub revision: String,
    pub from: String,
    pub to: Option<String>,
    pub affected_count: u32,
    pub merges: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TagHistoryEntry {
    pub id: String,
    pub affected_count: u32,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CatalogMetadataRequest {
    ExportHistory {},
    TagEditPreview {
        from: String,
        to: Option<String>,
    },
    TagEditApply {
        id: String,
        digest: String,
    },
    TagHistory {},
    TagUndo {
        id: String,
        expected_revision: String,
    },
    ImportKeywords {
        asset_id: String,
        expected_revision: String,
        restore_removed: bool,
    },
    Notes {
        asset_id: String,
    },
    SaveNotes {
        asset_id: String,
        expected_version: u32,
        caption: String,
        rejected: bool,
    },
    Collections {},
    CreateCollection {
        collection_kind: CollectionKind,
        name: String,
        query: Option<AssetQuery>,
    },
    UpdateCollection {
        id: String,
        expected_version: u32,
        name: String,
        query: Option<AssetQuery>,
    },
    ArchiveCollection {
        id: String,
        expected_version: u32,
        archived: bool,
    },
    CollectionMembers {
        id: String,
        expected_version: u32,
        expected_count: u32,
        selection: AssetSelection,
        remove: bool,
    },
    TagCategories {},
    SaveTagCategory {
        id: Option<String>,
        expected_version: Option<u32>,
        label: String,
        parent_id: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum CatalogMetadataReply {
    ExportHistory(Vec<super::super::metadata_export::model::MetadataExportRecord>),
    TagEditReview(TagEditReview),
    TagHistory(Vec<TagHistoryEntry>),
    TagHistoryEntry(TagHistoryEntry),
    Notes(AssetNotes),
    Collections(Vec<CatalogCollection>),
    Collection(CatalogCollection),
    Membership(CollectionMembership),
    TagCategories(Vec<TagCategory>),
    TagCategory(TagCategory),
}
