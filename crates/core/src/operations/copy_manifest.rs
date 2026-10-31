//! Pure reconciliation of explicitly selected companion decisions. Native
//! preflight and execution consume the private manifest, never display paths.
use crate::{
    domain::{AppError, ErrorCode, Result},
    organization::portable_relative_target,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyManifestInput {
    pub plan_id: String,
    pub bundle_confirmation_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CopyManifestIssueKind {
    SourceHasMultipleTargets,
    TargetHasMultipleSources,
    PossibleCaseCollision,
    FileDirectoryCollision,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyManifestIssue {
    pub kind: CopyManifestIssueKind,
    pub file_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyManifestFile {
    pub id: String,
    pub root_id: String,
    pub source_name: String,
    pub source_revision: String,
    pub target_relative_path: String,
    pub bytes: String,
    pub bundle_asset_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyManifest {
    pub id: String,
    pub plan_id: String,
    pub draft_digest: String,
    pub digest: String,
    pub created_at: String,
    pub bundle_confirmation_ids: Vec<String>,
    pub files: Vec<CopyManifestFile>,
    pub issues: Vec<CopyManifestIssue>,
    pub selected_bytes: String,
    pub execution_available: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManifestFile {
    pub view: CopyManifestFile,
    pub relative_native: Vec<u8>,
    pub identity: (u64, u64),
}

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}

/// Coalesce only the same native locator/revision/object and exact target.
/// Different revisions or source-to-target mappings never get silently chosen.
pub(crate) fn reconcile(
    mut inputs: Vec<ManifestFile>,
) -> Result<(Vec<ManifestFile>, Vec<CopyManifestIssue>, String)> {
    if inputs.is_empty() || inputs.len() > 15_600 {
        return Err(invalid(
            "A copy manifest must contain 1–15,600 reviewed file entries.",
        ));
    }
    inputs.sort_by(|a, b| {
        (
            &a.view.root_id,
            &a.relative_native,
            &a.view.target_relative_path,
        )
            .cmp(&(
                &b.view.root_id,
                &b.relative_native,
                &b.view.target_relative_path,
            ))
    });
    let mut files: Vec<ManifestFile> = Vec::new();
    let mut source_evidence = BTreeMap::new();
    for mut input in inputs {
        if !portable_relative_target(&input.view.target_relative_path)
            || input.relative_native.is_empty()
            || input.view.source_revision.len() != 64
        {
            return Err(invalid(
                "A reviewed file has invalid native source or target evidence.",
            ));
        }
        let bytes = input
            .view
            .bytes
            .parse::<u64>()
            .map_err(|_| invalid("A reviewed file has an invalid byte count."))?;
        let key = (input.view.root_id.clone(), input.relative_native.clone());
        let evidence = (input.identity, input.view.source_revision.clone(), bytes);
        if let Some(previous) = source_evidence.insert(key, evidence.clone())
            && previous != evidence
        {
            return Err(invalid(
                "Overlapping bundles disagree about a source revision. Review all affected companions again.",
            ));
        }
        input.view.bundle_asset_ids.sort();
        input.view.bundle_asset_ids.dedup();
        let id = serde_json::to_vec(&(
            &input.view.root_id,
            &input.relative_native,
            input.identity,
            &input.view.source_revision,
            &input.view.target_relative_path,
        ))
        .map_err(|e| invalid("Could not fingerprint a manifest file.").caused_by(e))?;
        input.view.id = blake3::hash(&id).to_hex().to_string();
        if let Some(previous) = files.last_mut()
            && previous.view.id == input.view.id
        {
            previous
                .view
                .bundle_asset_ids
                .extend(input.view.bundle_asset_ids);
            previous.view.bundle_asset_ids.sort();
            previous.view.bundle_asset_ids.dedup();
        } else {
            files.push(input);
        }
    }
    let mut sources: BTreeMap<_, Vec<String>> = BTreeMap::new();
    let mut targets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut folded: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut total = 0u128;
    for file in &files {
        sources
            .entry((&file.view.root_id, &file.relative_native))
            .or_default()
            .push(file.view.id.clone());
        targets
            .entry(file.view.target_relative_path.clone())
            .or_default()
            .push(file.view.id.clone());
        folded
            .entry(file.view.target_relative_path.to_ascii_lowercase())
            .or_default()
            .push(file.view.id.clone());
        total += u128::from(
            file.view
                .bytes
                .parse::<u64>()
                .map_err(|_| invalid("Invalid manifest bytes."))?,
        );
    }
    let mut issues = Vec::new();
    for ids in sources.values().filter(|ids| ids.len() > 1) {
        issues.push(CopyManifestIssue {
            kind: CopyManifestIssueKind::SourceHasMultipleTargets,
            file_ids: ids.clone(),
        });
    }
    for ids in targets.values().filter(|ids| ids.len() > 1) {
        issues.push(CopyManifestIssue {
            kind: CopyManifestIssueKind::TargetHasMultipleSources,
            file_ids: ids.clone(),
        });
    }
    for ids in folded.values().filter(|ids| ids.len() > 1) {
        // Report possible case equivalence even on a case-sensitive volume;
        // actual native semantics must independently validate future names.
        issues.push(CopyManifestIssue {
            kind: CopyManifestIssueKind::PossibleCaseCollision,
            file_ids: ids.clone(),
        });
    }
    let mut directory_conflicts = BTreeSet::new();
    for (path, ids) in &folded {
        let mut prefix = path.as_str();
        while let Some((parent, _)) = prefix.rsplit_once('/') {
            if let Some(ancestor) = folded.get(parent) {
                let mut conflict = ids.clone();
                conflict.extend(ancestor.iter().cloned());
                conflict.sort();
                conflict.dedup();
                directory_conflicts.insert(conflict);
            }
            prefix = parent;
        }
    }
    for ids in directory_conflicts {
        issues.push(CopyManifestIssue {
            kind: CopyManifestIssueKind::FileDirectoryCollision,
            file_ids: ids,
        });
    }
    Ok((files, issues, total.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(source: &str, target: &str, asset: &str) -> ManifestFile {
        ManifestFile {
            view: CopyManifestFile {
                id: String::new(),
                root_id: "root".into(),
                source_name: source.into(),
                source_revision: "a".repeat(64),
                target_relative_path: target.into(),
                bytes: "9007199254740993".into(),
                bundle_asset_ids: vec![asset.into()],
            },
            relative_native: source.as_bytes().to_vec(),
            identity: (1, u64::from(source.as_bytes()[0])),
        }
    }
    #[test]
    fn overlapping_identical_bundle_files_coalesce_without_losing_membership_or_precision() {
        let (files, issues, total) = reconcile(vec![
            file("sidecar", "2026/photo.xmp", "raw"),
            file("sidecar", "2026/photo.xmp", "jpeg"),
        ])
        .unwrap();
        assert_eq!(files.len(), 1);
        assert!(issues.is_empty());
        assert_eq!(total, "9007199254740993");
        assert_eq!(files[0].view.bundle_asset_ids, ["jpeg", "raw"]);
    }
    #[test]
    fn cross_bundle_conflicts_are_retained_for_review_and_never_resolved_by_order() {
        let inputs = vec![
            file("a", "Folder/photo.xmp", "raw"),
            file("a", "different/photo.xmp", "jpeg"),
            file("b", "Folder/photo.xmp", "other"),
            file("c", "folder/PHOTO.XMP/child", "fourth"),
        ];
        let (files, issues, total) = reconcile(inputs.clone()).unwrap();
        assert_eq!(files.len(), 4);
        assert_eq!(total, "36028797018963972");
        for kind in [
            CopyManifestIssueKind::SourceHasMultipleTargets,
            CopyManifestIssueKind::TargetHasMultipleSources,
            CopyManifestIssueKind::PossibleCaseCollision,
            CopyManifestIssueKind::FileDirectoryCollision,
        ] {
            assert!(
                issues
                    .iter()
                    .any(|issue| std::mem::discriminant(&issue.kind)
                        == std::mem::discriminant(&kind))
            );
        }
        let mut reversed = inputs;
        reversed.reverse();
        let other = reconcile(reversed).unwrap();
        assert_eq!(
            serde_json::to_string(&files).unwrap(),
            serde_json::to_string(&other.0).unwrap()
        );
    }
    #[test]
    fn overlapping_changed_sources_and_unsafe_targets_fail_instead_of_rebasing() {
        let mut changed = file("a", "photo.xmp", "raw");
        changed.view.source_revision = "b".repeat(64);
        assert!(reconcile(vec![file("a", "photo.xmp", "jpeg"), changed]).is_err());
        assert!(reconcile(vec![file("a", "../escape", "raw")]).is_err());
    }
}
