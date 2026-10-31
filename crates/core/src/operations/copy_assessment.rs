//! Copy-only space estimate and explicit fidelity limits. No approval authority.
use super::CopyManifest;
use crate::{
    domain::{AppError, ErrorCode, Result},
    organization::portable_relative_target,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ts_rs::TS;

pub const CAPACITY_POLICY: &str = "retained-source-copy-estimate-v1";
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopyCapacityStatus {
    SufficientForEstimate,
    InsufficientForEstimate,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyCapacityEstimate {
    pub policy: String,
    pub status: CopyCapacityStatus,
    pub allocation_unit_bytes: String,
    pub logical_bytes: String,
    pub rounded_data_bytes: String,
    pub metadata_allowance_bytes: String,
    pub journal_allowance_bytes: String,
    pub headroom_bytes: String,
    pub estimated_peak_additional_bytes: String,
    pub observed_available_bytes: String,
    pub observed_total_bytes: String,
    pub output_directories: u32,
    pub space_reserved: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CopyFidelityAspect {
    FileBytes,
    ConfirmedCompanions,
    Timestamps,
    PermissionsAndOwnership,
    ExtendedAttributesAndResourceForks,
    AlternateDataStreams,
    SparseAndCompressionLayout,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CopyFidelityStatus {
    VerificationRequired,
    SeparateFilesSelected,
    NotPreservedOrVerified,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyFidelityItem {
    pub aspect: CopyFidelityAspect,
    pub status: CopyFidelityStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CopyAssessment {
    pub plan_id: String,
    pub manifest_id: String,
    pub manifest_digest: String,
    pub observed_at: String,
    pub capacity: CopyCapacityEstimate,
    pub fidelity: Vec<CopyFidelityItem>,
    pub sources_retained: bool,
    pub independent_backup_verified: bool,
    pub execution_available: bool,
}
pub(crate) fn assess(
    manifest: &CopyManifest,
    unit: u64,
    available: u64,
    total: u64,
) -> Result<CopyAssessment> {
    let invalid = || {
        AppError::new(
            ErrorCode::InvalidInput,
            "Invalid or unsupported copy capacity scope. Use a smaller fresh manifest.",
        )
    };
    if manifest.files.is_empty()
        || manifest.files.len() > 15_600
        || unit == 0
        || unit > 1024 * 1024 * 1024
        || available > total
    {
        return Err(invalid());
    }
    let mut directories = BTreeSet::from([String::new()]);
    let mut logical = 0u128;
    let mut rounded = 0u128;
    for file in &manifest.files {
        if !portable_relative_target(&file.target_relative_path) {
            return Err(invalid());
        }
        let bytes = file.bytes.parse::<u64>().map_err(|_| invalid())? as u128;
        logical += bytes;
        rounded += bytes.div_ceil(unit as u128) * unit as u128;
        let parts: Vec<_> = file.target_relative_path.split('/').collect();
        for count in 1..parts.len() {
            directories.insert(parts[..count].join("/"));
        }
        if directories.len() > 1024 {
            return Err(invalid());
        }
    }
    if logical.to_string() != manifest.selected_bytes {
        return Err(invalid());
    }
    // These are declared planning allowances, never a filesystem upper bound.
    // Staging and published names refer to the same output objects: no second
    // data copy is allocated by no-replace rename. Source storage stays intact.
    let metadata = (manifest.files.len() + directories.len()) as u128 * 65_536;
    let journal = 32u128 * 1024 * 1024;
    let headroom = rounded.div_ceil(10).max(64 * 1024 * 1024);
    let peak = rounded + metadata + journal + headroom;
    use CopyFidelityAspect::*;
    use CopyFidelityStatus::*;
    Ok(CopyAssessment {
        plan_id: manifest.plan_id.clone(),
        manifest_id: manifest.id.clone(),
        manifest_digest: manifest.digest.clone(),
        observed_at: String::new(),
        capacity: CopyCapacityEstimate {
            policy: CAPACITY_POLICY.into(),
            status: if peak <= available as u128 {
                CopyCapacityStatus::SufficientForEstimate
            } else {
                CopyCapacityStatus::InsufficientForEstimate
            },
            allocation_unit_bytes: unit.to_string(),
            logical_bytes: logical.to_string(),
            rounded_data_bytes: rounded.to_string(),
            metadata_allowance_bytes: metadata.to_string(),
            journal_allowance_bytes: journal.to_string(),
            headroom_bytes: headroom.to_string(),
            estimated_peak_additional_bytes: peak.to_string(),
            observed_available_bytes: available.to_string(),
            observed_total_bytes: total.to_string(),
            output_directories: directories.len() as u32,
            space_reserved: false,
        },
        fidelity: vec![
            CopyFidelityItem {
                aspect: FileBytes,
                status: VerificationRequired,
            },
            CopyFidelityItem {
                aspect: ConfirmedCompanions,
                status: SeparateFilesSelected,
            },
            CopyFidelityItem {
                aspect: Timestamps,
                status: NotPreservedOrVerified,
            },
            CopyFidelityItem {
                aspect: PermissionsAndOwnership,
                status: NotPreservedOrVerified,
            },
            CopyFidelityItem {
                aspect: ExtendedAttributesAndResourceForks,
                status: NotPreservedOrVerified,
            },
            CopyFidelityItem {
                aspect: AlternateDataStreams,
                status: NotPreservedOrVerified,
            },
            CopyFidelityItem {
                aspect: SparseAndCompressionLayout,
                status: NotPreservedOrVerified,
            },
        ],
        sources_retained: true,
        independent_backup_verified: false,
        execution_available: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::CopyManifestFile;
    fn manifest(sizes: &[u64]) -> CopyManifest {
        CopyManifest {
            id: "manifest".into(),
            plan_id: "plan".into(),
            draft_digest: "draft".into(),
            digest: "digest".into(),
            created_at: "now".into(),
            bundle_confirmation_ids: vec![],
            files: sizes
                .iter()
                .enumerate()
                .map(|(index, size)| CopyManifestFile {
                    id: index.to_string(),
                    root_id: "root".into(),
                    source_name: "source".into(),
                    source_revision: "revision".into(),
                    target_relative_path: format!("trip/{index}.bin"),
                    bytes: size.to_string(),
                    bundle_asset_ids: vec![],
                })
                .collect(),
            issues: vec![],
            selected_bytes: sizes
                .iter()
                .map(|size| *size as u128)
                .sum::<u128>()
                .to_string(),
            execution_available: false,
        }
    }
    #[test]
    fn copy_estimate_rounds_each_file_once_and_keeps_explicit_allowances_and_limits() {
        let result = assess(&manifest(&[0, 1, 4096, 4097]), 4096, u64::MAX, u64::MAX).unwrap();
        let cap = result.capacity;
        assert_eq!(cap.logical_bytes, "8194");
        assert_eq!(cap.rounded_data_bytes, "16384");
        assert_eq!(cap.metadata_allowance_bytes, (6u64 * 65536).to_string());
        let peak = 16384 + 6u64 * 65536 + 32 * 1024 * 1024 + 64 * 1024 * 1024;
        assert_eq!(cap.estimated_peak_additional_bytes, peak.to_string());
        assert_eq!(cap.status, CopyCapacityStatus::SufficientForEstimate);
        assert!(!cap.space_reserved);
        assert!(
            !result.execution_available
                && !result.independent_backup_verified
                && result.sources_retained
        );
        assert!(result.fidelity.iter().all(|item| !matches!(
            item.status,
            CopyFidelityStatus::VerificationRequired
        ) || matches!(
            item.aspect,
            CopyFidelityAspect::FileBytes
        )));
        assert_eq!(
            assess(&manifest(&[0, 1, 4096, 4097]), 4096, peak - 1, u64::MAX)
                .unwrap()
                .capacity
                .status,
            CopyCapacityStatus::InsufficientForEstimate
        );
        assert_eq!(
            assess(&manifest(&[0, 1, 4096, 4097]), 4096, peak, u64::MAX)
                .unwrap()
                .capacity
                .status,
            CopyCapacityStatus::SufficientForEstimate
        );
    }
    #[test]
    fn large_counts_remain_exact_and_bad_scope_never_becomes_a_space_success() {
        let scope = manifest(&[u64::MAX, u64::MAX]);
        let result = assess(&scope, 4096, u64::MAX, u64::MAX).unwrap();
        assert_eq!(
            result.capacity.logical_bytes,
            (2u128 * u64::MAX as u128).to_string()
        );
        assert_eq!(
            result.capacity.status,
            CopyCapacityStatus::InsufficientForEstimate
        );
        for unit in [0, u64::MAX] {
            assert!(assess(&scope, unit, 1, 1).is_err());
        }
        assert!(assess(&scope, 4096, 2, 1).is_err());
        let mut scope = manifest(&[1]);
        scope.selected_bytes = "2".into();
        assert!(assess(&scope, 4096, 1, 1).is_err());
        scope.selected_bytes = "1".into();
        scope.files[0].target_relative_path = "../outside".into();
        assert!(assess(&scope, 4096, 1, 1).is_err());
    }
}
