use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetadataExportKind {
    Catalog,
    Xmp,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MetadataExportFile {
    pub name: String,
    pub bytes: String,
    pub digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MetadataExportReview {
    pub id: String,
    pub digest: String,
    pub revision: String,
    pub export_kind: MetadataExportKind,
    pub destination: String,
    pub files: Vec<MetadataExportFile>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MetadataExportSaved {
    pub id: String,
    pub files: u32,
    pub destination: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MetadataExportRecord {
    pub id: String,
    pub last_phase: String,
    pub files_verified: u32,
    pub updated_at: String,
}
