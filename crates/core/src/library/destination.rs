//! Read-only destination identity/overlap observations. No destination writes,
//! name reservation, capacity promise, persistence or execution authority.
use super::{Library, filters::invalid, native, store};
use crate::filesystem::DirectoryChain;
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::FileIdentity,
    organization::{OrganizationDestinationReview, OrganizationTemplate},
};
use std::path::{Path, PathBuf};
mod targets;
fn same(left: &FileIdentity, right: &FileIdentity) -> bool {
    left.volume == right.volume && left.file == right.file
}
fn directory_chain(path: &Path) -> Result<DirectoryChain> {
    if path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .to_ascii_lowercase()
            .ends_with(".photoslibrary")
    }) {
        return Err(invalid(
            "Choose an ordinary destination outside Apple Photos library packages.",
        ));
    }
    DirectoryChain::open(path)
}

impl Library {
    pub fn organization_destination(
        &self,
        asset_ids: Vec<String>,
        template: OrganizationTemplate,
        preview_fingerprint: String,
        path: PathBuf,
        expected: (FileIdentity, String),
    ) -> Result<OrganizationDestinationReview> {
        let _admission = self.destination_gate.try_lock().map_err(|_| {
            AppError::new(
                ErrorCode::Busy,
                "A destination review is already running. Try again after it finishes.",
            )
        })?;
        let conflict = || {
            AppError::new(
                ErrorCode::Conflict,
                "The preview or folder identity changed. Build a fresh preview and choose the destination again.",
            )
        };
        // Rebuild from backend-owned snapshots; never trust renderer rows/paths.
        let preview = self.organization_preview(asset_ids.clone(), template.clone())?;
        if preview.fingerprint != preview_fingerprint {
            return Err(conflict());
        }
        // Picker paths may contain Windows short-name/case aliases. Resolve
        // spelling, then independently open every resolved component and bind
        // the final identity/volume to the picker observation. Canonicalization
        // alone is never authorization, and this grants no execution authority.
        let selected_path = path;
        // Apply the existing no-recall/reparse/placeholder guard before asking
        // the OS to resolve spelling, and retain that selected object as well.
        let selected = native::open_root(&selected_path, Some(&expected.0))?;
        if native::volume_token(&selected)? != expected.1 {
            return Err(conflict());
        }
        let path = selected_path.canonicalize()?;
        let destination = directory_chain(&path)?;
        let volume = native::volume_token(destination.leaf())?;
        if !same(destination.identity(), &expected.0) || volume != expected.1 {
            return Err(conflict());
        }
        let managed = directory_chain(&self.options.data_directory)?;
        if destination.overlaps(&managed) {
            return Err(invalid(
                "Choose a destination separate from application catalog and cache folders.",
            ));
        }
        let roots = store::roots(&self.catalog)?;
        if roots.len() > 32 {
            return Err(invalid("Too many library roots for destination review."));
        }
        let mut evidence = Vec::new();
        let mut held = Vec::new();
        for root in roots {
            // Offline/inaccessible/replaced roots cannot prove nonoverlap.
            let chain = directory_chain(&root.path).map_err(|error| AppError::new(error.code.clone(),
                "Every indexed root must be accessible for destination overlap review. Reconnect or relink unavailable roots.").caused_by(error))?;
            if !same(chain.identity(), &root.identity)
                || native::volume_token(chain.leaf())? != root.volume
            {
                return Err(conflict());
            }
            if destination.overlaps(&chain) {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Choose a destination outside every indexed library, with no ancestor or descendant overlap.",
                ));
            }
            evidence.push((
                root.view.id,
                native::encode_path(&root.path),
                root.volume,
                root.identity.volume,
                root.identity.file,
            ));
            held.push(chain);
        }
        let (targets, target_evidence) = targets::review(destination.leaf(), &preview.rows)?;
        let space = crate::filesystem::volume_probe(&path)?;
        // A read-only review is an observation, not a reservation. Check the
        // selected path again and reject catalog changes during native work.
        let current_selected = native::open_root(&selected_path, Some(&expected.0))?;
        if native::volume_token(&current_selected)? != volume {
            return Err(conflict());
        }
        let current = directory_chain(&selected_path.canonicalize()?)?;
        if !same(current.identity(), destination.identity())
            || native::volume_token(current.leaf())? != volume
        {
            return Err(conflict());
        }
        let current_preview = self.organization_preview(asset_ids, template)?;
        if current_preview.fingerprint != preview_fingerprint {
            return Err(conflict());
        }
        let bytes = serde_json::to_vec(&(
            "organization-destination-v2-target-observations",
            &preview_fingerprint,
            native::encode_path(&path),
            &volume,
            destination.identity().volume,
            destination.identity().file,
            evidence,
            target_evidence,
        ))
        .map_err(|e| invalid("Could not fingerprint destination observations.").caused_by(e))?;
        Ok(OrganizationDestinationReview {
            preview_fingerprint,
            catalog_revision: preview.catalog_revision,
            destination_path: path.to_string_lossy().into_owned(),
            fingerprint: blake3::hash(&bytes).to_hex().to_string(),
            execution_available: false,
            persisted: false,
            targets,
            available_bytes: space.available_bytes.to_string(),
            total_bytes: space.total_bytes.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_directory_chains_detect_overlap_and_leave_siblings_separate() {
        let fixture = tempfile::tempdir().unwrap();
        std::fs::create_dir(fixture.path().join("source")).unwrap();
        std::fs::create_dir(fixture.path().join("destination")).unwrap();
        // Canonical fixture path avoids the macOS /var temporary-directory alias.
        let base = fixture.path().canonicalize().unwrap();
        let parent = directory_chain(&base).unwrap();
        let source = directory_chain(&base.join("source")).unwrap();
        let destination = directory_chain(&base.join("destination")).unwrap();
        assert!(parent.overlaps(&source));
        assert!(source.overlaps(&parent));
        assert!(source.overlaps(&source));
        assert!(!source.overlaps(&destination));
        assert!(directory_chain(Path::new("relative")).is_err());
        assert!(directory_chain(&base.join("missing")).is_err());
        // Verbatim Windows PathBuf::join normalizes away '..'. Preserve the
        // raw spelling here to exercise rejection rather than that normalization.
        let mut traversal = base.as_os_str().to_os_string();
        traversal.push(format!(
            "{0}source{0}..{0}destination",
            std::path::MAIN_SEPARATOR
        ));
        assert!(directory_chain(Path::new(&traversal)).is_err());
    }

    #[test]
    fn destination_ipc_rejects_renderer_paths_and_execution_authority() {
        let valid = serde_json::json!({"kind":"organization_destination", "selection_id":"opaque",
            "asset_ids":[], "template":{"pattern":"{original_stem}.{ext}","event":"","missingDate":"undated"},
            "preview_fingerprint":"fingerprint"});
        assert!(
            serde_json::from_value::<super::super::model::LibraryRequest>(valid.clone()).is_ok()
        );
        for field in ["path", "destination_path", "approved", "execute", "rows"] {
            let mut hostile = valid.clone();
            hostile[field] = serde_json::json!("unapproved");
            assert!(
                serde_json::from_value::<super::super::model::LibraryRequest>(hostile).is_err()
            );
        }
    }

    #[test]
    fn directory_chain_refuses_a_link_in_an_ancestor() {
        let fixture = tempfile::tempdir().unwrap();
        let base = fixture.path().canonicalize().unwrap();
        let target = base.join("target");
        std::fs::create_dir_all(target.join("nested")).unwrap();
        std::fs::write(target.join("preserved"), b"source bytes").unwrap();
        let link = base.join("alias");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(windows)]
        assert!(std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:STILLHARBOUR_TEST_JUNCTION -Target $env:STILLHARBOUR_TEST_TARGET | Out-Null"])
            .env("STILLHARBOUR_TEST_JUNCTION", &link).env("STILLHARBOUR_TEST_TARGET", &target).status().unwrap().success());
        assert!(directory_chain(&link.join("nested")).is_err());
        assert_eq!(
            std::fs::read(target.join("preserved")).unwrap(),
            b"source bytes"
        );
    }
}
