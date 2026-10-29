//! The Rust definitions are the IPC schema; export_bindings generates TypeScript.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FoundationRequest {
    Status {},
    PickDirectory {},
    GetPreferences {},
    SavePreferences {
        preferences: Box<crate::preferences::AppPreferences>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FoundationStatus {
    pub name: String,
    pub version: String,
    pub milestone: String,
    pub sqlite_version: String,
    pub catalog_id: String,
    pub catalog_schema: u32,
    pub original_mutations_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DirectorySelection {
    pub id: String,
    /// Display only. Native PathBuf and identity stay in the backend.
    pub display_path: String,
    pub identity_available: bool,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum FoundationResponse {
    Status(FoundationStatus),
    Directory(Option<DirectorySelection>),
    Preferences(crate::preferences::AppPreferences),
}

pub const FOUNDATION_COMMAND: &str = "foundation_request";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renderer_cannot_supply_native_paths_or_unknown_actions() {
        assert!(
            serde_json::from_str::<FoundationRequest>(
                r#"{"kind":"pick_directory","path":"/unapproved"}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<FoundationRequest>(r#"{"kind":"delete"}"#).is_err());
    }
}
