//! Scope-consistent logical and reported allocation totals. Never disk savings.
use super::invalid;
use crate::domain::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

const MAX_OBJECT_MEMBERSHIPS: usize = 250_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum InspectionScope {
    IndexedMediaCompanions,
    AllEligibleFiles,
}
pub(crate) struct FileSize {
    pub id: String,
    pub root_id: String,
    pub root_label: String,
    /// Opaque backend folder IDs and display labels; never execution paths.
    pub folders: Vec<(String, String)>,
    pub object: Option<String>,
    pub logical: u64,
    pub allocated: Option<u64>,
    pub in_media_scope: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InventoryCoverage {
    pub excluded_entries: u32,
    pub inaccessible_entries: u32,
    pub incomplete_directories: u32,
    pub enumeration_complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FolderSize {
    pub id: String,
    pub parent_id: Option<String>,
    pub root_id: Option<String>,
    pub name: String,
    pub files: u32,
    pub direct_files: u32,
    pub direct_logical_bytes: String,
    pub logical_bytes: String,
    pub unique_object_logical_bytes: Option<String>,
    pub reported_allocated_bytes: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StorageReport {
    pub scope: InspectionScope,
    pub coverage: InventoryCoverage,
    pub folders: Vec<FolderSize>,
    pub largest_file_ids: Vec<String>,
    pub reclaimable_physical_bytes: Option<String>,
    pub quarantine_freed_bytes: String,
    pub accounting_incomplete: bool,
}
struct Node {
    row: FolderSize,
    logical: u128,
    direct_logical: u128,
    unique: u128,
    allocated: u128,
    objects: BTreeMap<String, (u64, Option<u64>)>,
    unique_unknown: bool,
    allocated_unknown: bool,
}
fn node(id: String, parent_id: Option<String>, root_id: Option<String>, name: String) -> Node {
    Node {
        row: FolderSize {
            id,
            parent_id,
            root_id,
            name,
            files: 0,
            direct_files: 0,
            direct_logical_bytes: "0".into(),
            logical_bytes: "0".into(),
            unique_object_logical_bytes: None,
            reported_allocated_bytes: None,
        },
        logical: 0,
        direct_logical: 0,
        unique: 0,
        allocated: 0,
        objects: BTreeMap::new(),
        unique_unknown: false,
        allocated_unknown: false,
    }
}
pub(crate) fn aggregate(
    scope: InspectionScope,
    files: &[FileSize],
    coverage: InventoryCoverage,
) -> Result<StorageReport> {
    aggregate_bounded(scope, files, coverage, 50_000)
}
fn aggregate_bounded(
    scope: InspectionScope,
    files: &[FileSize],
    coverage: InventoryCoverage,
    node_limit: usize,
) -> Result<StorageReport> {
    let mut nodes = BTreeMap::from([(
        "scope".into(),
        node("scope".into(), None, None, "Selected scope".into()),
    )]);
    let mut membership_count = 0;
    let mut accounting_incomplete = false;
    let mut eligible = Vec::new();
    for file in files
        .iter()
        .filter(|file| scope == InspectionScope::AllEligibleFiles || file.in_media_scope)
    {
        if file.folders.len() > 64 {
            return Err(invalid("Folder accounting exceeds its 64-level boundary."));
        }
        eligible.push(file);
        let mut ancestors = vec!["scope".to_owned()];
        let mut parent = "scope".to_owned();
        for (id, name) in std::iter::once((&file.root_id, &file.root_label))
            .chain(file.folders.iter().map(|(id, name)| (id, name)))
        {
            if ancestors.contains(id) {
                return Err(invalid(
                    "Folder accounting contains a repeated ancestor ID.",
                ));
            }
            if !nodes.contains_key(id) && nodes.len() >= node_limit {
                return Err(crate::domain::AppError::new(
                    crate::domain::ErrorCode::Unsupported,
                    "Folder accounting reached its 50,000-node bound; retained file observations remain available, but no complete tree is published.",
                ));
            }
            let entry = nodes.entry(id.clone()).or_insert_with(|| {
                node(
                    id.clone(),
                    Some(parent.clone()),
                    Some(file.root_id.clone()),
                    name.clone(),
                )
            });
            if entry.row.parent_id.as_ref() != Some(&parent)
                || entry.row.root_id.as_ref() != Some(&file.root_id)
                || entry.row.name != *name
            {
                return Err(invalid("A folder ID has conflicting retained scope."));
            }
            parent = id.clone();
            ancestors.push(id.clone());
        }
        let immediate = ancestors.last().cloned();
        for id in ancestors {
            let entry = nodes
                .get_mut(&id)
                .ok_or_else(|| invalid("Folder accounting scope is incomplete."))?;
            entry.row.files += 1;
            entry.logical += u128::from(file.logical);
            if immediate.as_ref() == Some(&id) {
                entry.row.direct_files += 1;
                entry.direct_logical += u128::from(file.logical);
            }
            let Some(object) = &file.object else {
                entry.unique_unknown = true;
                entry.allocated_unknown = true;
                continue;
            };
            if entry.unique_unknown {
                continue;
            }
            if let Some(previous) = entry.objects.get(object) {
                if *previous != (file.logical, file.allocated) {
                    entry.unique_unknown = true;
                    entry.allocated_unknown = true;
                    accounting_incomplete = true;
                }
            } else {
                if membership_count >= MAX_OBJECT_MEMBERSHIPS {
                    accounting_incomplete = true;
                    entry.unique_unknown = true;
                    entry.allocated_unknown = true;
                    membership_count -= entry.objects.len();
                    entry.objects.clear();
                    continue;
                }
                entry
                    .objects
                    .insert(object.clone(), (file.logical, file.allocated));
                membership_count += 1;
                entry.unique += u128::from(file.logical);
                match file.allocated {
                    Some(bytes) => entry.allocated += u128::from(bytes),
                    None => entry.allocated_unknown = true,
                }
            }
        }
    }
    eligible.sort_by(|a, b| b.logical.cmp(&a.logical).then_with(|| a.id.cmp(&b.id)));
    let folders = nodes
        .into_values()
        .map(|mut entry| {
            entry.row.logical_bytes = entry.logical.to_string();
            entry.row.direct_logical_bytes = entry.direct_logical.to_string();
            entry.row.unique_object_logical_bytes =
                (!entry.unique_unknown).then(|| entry.unique.to_string());
            entry.row.reported_allocated_bytes =
                (!entry.allocated_unknown).then(|| entry.allocated.to_string());
            entry.row
        })
        .collect();
    Ok(StorageReport {
        scope,
        coverage,
        folders,
        largest_file_ids: eligible.iter().map(|file| file.id.clone()).collect(),
        reclaimable_physical_bytes: None,
        quarantine_freed_bytes: "0".into(),
        accounting_incomplete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(id: &str, bytes: u64, media: bool) -> FileSize {
        FileSize {
            id: id.into(),
            root_id: "root".into(),
            root_label: "Fixture".into(),
            folders: vec![("folder".into(), "Known".into())],
            object: Some(id.into()),
            logical: bytes,
            allocated: Some(512),
            in_media_scope: media,
        }
    }
    #[test]
    fn folder_bound_refuses_a_complete_report_instead_of_growing_without_limit() {
        let error = aggregate_bounded(
            InspectionScope::AllEligibleFiles,
            &[file("generated", 100, false)],
            InventoryCoverage::default(),
            2,
        )
        .unwrap_err();
        assert_eq!(error.code, crate::domain::ErrorCode::Unsupported);
    }
    #[test]
    fn inconsistent_alias_lengths_or_allocations_mark_object_accounting_unknown() {
        let mut a = file("a", 100, false);
        let mut alias = file("alias", 200, false);
        alias.object = a.object.clone();
        a.allocated = Some(8);
        alias.allocated = Some(16);
        let report = aggregate(
            InspectionScope::AllEligibleFiles,
            &[a, alias],
            InventoryCoverage::default(),
        )
        .unwrap();
        assert!(report.accounting_incomplete);
        assert!(
            report
                .folders
                .iter()
                .all(|folder| folder.logical_bytes == "300"
                    && folder.unique_object_logical_bytes.is_none()
                    && folder.reported_allocated_bytes.is_none())
        );
    }
    #[test]
    fn known_tree_totals_largest_order_and_media_all_scope_agree() {
        let files = vec![
            file("a", 100, false),
            file("b", 200, false),
            file("c", 300, false),
            file("photo", 400, true),
        ];
        let coverage = InventoryCoverage {
            enumeration_complete: true,
            ..InventoryCoverage::default()
        };
        let all = aggregate(InspectionScope::AllEligibleFiles, &files, coverage.clone()).unwrap();
        let media = aggregate(InspectionScope::IndexedMediaCompanions, &files, coverage).unwrap();
        assert_eq!(all.largest_file_ids, ["photo", "c", "b", "a"]);
        assert!(
            all.folders
                .iter()
                .all(|folder| folder.logical_bytes == "1000" && folder.files == 4)
        );
        assert!(
            media
                .folders
                .iter()
                .all(|folder| folder.logical_bytes == "400" && folder.files == 1)
        );
        assert_eq!(all.quarantine_freed_bytes, "0");
        assert!(all.reclaimable_physical_bytes.is_none());
    }
    #[test]
    fn hardlinks_sparse_allocation_and_unknown_physical_savings_remain_distinct() {
        let mut files = vec![
            file("a", 300, false),
            file("alias", 300, false),
            file("independent", 300, false),
        ];
        files[0].allocated = Some(8);
        files[1].allocated = Some(8);
        files[1].object = files[0].object.clone();
        let result = aggregate(
            InspectionScope::AllEligibleFiles,
            &files,
            InventoryCoverage::default(),
        )
        .unwrap();
        let root = result.folders.iter().find(|row| row.id == "scope").unwrap();
        assert_eq!(root.logical_bytes, "900");
        assert_eq!(root.unique_object_logical_bytes.as_deref(), Some("600"));
        assert_eq!(root.reported_allocated_bytes.as_deref(), Some("520"));
        assert!(!result.coverage.enumeration_complete);
        assert!(result.reclaimable_physical_bytes.is_none());
        files[2].allocated = None;
        let result = aggregate(
            InspectionScope::AllEligibleFiles,
            &files,
            InventoryCoverage::default(),
        )
        .unwrap();
        assert!(
            result
                .folders
                .iter()
                .all(|row| row.reported_allocated_bytes.is_none())
        );
    }
}
