//! Explicit read-only catalog discovery of an already completed import output.
//! This grants no source mutation, digest baseline or independent backup claim.
use super::*;
use crate::library::{
    model::{RootMode, ScanFilter},
    store,
};

impl CopyWorkflow {
    pub(crate) fn index_import_output(
        &self,
        library: Arc<Library>,
        job_id: String,
        path: PathBuf,
        identity: (FileIdentity, String),
    ) -> Result<ImportOutputIndex> {
        library.ensure_work_admission()?;
        let _permit = self.gate.try_start()?;
        let retained = self.retained_copy(job_id.clone(), &path, &identity.0, &identity.1)?;
        if retained.job.state != CopyJobState::CompletedVerified
            || retained.sources.import_selection.is_none()
            || self.import_copy_receipt(job_id.clone())?.is_none()
        {
            return Err(failed(
                "A completed copy import and its exact verification receipt are required before output indexing.",
            ));
        }
        let scope = retained.sources.open_scope(&self.catalog, Access::Read)?;
        let inventory = retained
            .journal
            .recovery_inventory(scope.directory(1).leaf())?;
        let output =
            super::super::native::read_directory(scope.directory(1).leaf(), &retained.namespace)?;
        retained.journal.validate_completed_members(&output)?;
        inventory.validate()?;
        let output_path = path.join(&retained.namespace).canonicalize()?;
        let chain = crate::filesystem::DirectoryChain::open(&output_path)?;
        let output_identity = crate::filesystem::identity(chain.leaf())?;
        let volume = native::volume_token(chain.leaf())?;
        if !retained.journal.plan().destination.matches(chain.leaf())?
            || volume != retained.sources.destination_volume
        {
            return Err(failed(
                "The completed output folder changed native identity or volume.",
            ));
        }
        let roots = store::roots(&self.catalog)?;
        let owner = retained.sources.owner_id.clone();
        let managed_id = self.catalog.call(move |db| {
            Ok(crate::library::operation_store::load(db, &owner)?
                .view
                .destination_root_id)
        })?;
        if let Some(id) = managed_id {
            let root = roots
                .iter()
                .find(|root| root.view.id == id)
                .ok_or_else(|| failed("The selected managed destination no longer exists."))?;
            if root.view.mode != RootMode::Managed
                || root.identity.volume != retained.sources.destination.volume
                || root.identity.file != retained.sources.destination.file
                || root.volume != volume
            {
                return Err(failed(
                    "The indexed destination grant changed. Retain the verified output.",
                ));
            }
            // Existing discovery owns its scan independently of this dialog.
            // Join it without claiming cancellation ownership; otherwise request
            // an ordinary scan using this root's existing filters/grant.
            let active = store::jobs(&self.catalog)?.into_iter().find(|job| {
                job.root_id == id
                    && !matches!(job.kind, Some(crate::library::model::JobKind::Thumbnails))
                    && matches!(
                        job.state.as_str(),
                        "queued" | "running" | "cancel_requested"
                    )
            });
            let (scan, started) = match active {
                Some(scan) => (scan, false),
                None => (library.scan(id, root.view.filter.clone())?, true),
            };
            return Ok(ImportOutputIndex {
                scan_started: Some(started),
                job_id,
                root: root.view.clone(),
                scan: Some(scan),
                already_registered: true,
                original_mutations_enabled: false,
            });
        }
        if let Some(root) = roots.into_iter().find(|root| {
            root.identity.volume == output_identity.volume
                && root.identity.file == output_identity.file
        }) {
            if root.path != output_path || root.volume != volume {
                return Err(failed(
                    "The indexed output identity has a different retained root mapping.",
                ));
            }
            let scan = store::jobs(&self.catalog)?.into_iter().find(|scan| {
                scan.root_id == root.view.id
                    && !matches!(&scan.kind, Some(crate::library::model::JobKind::Thumbnails))
            });
            return Ok(ImportOutputIndex {
                scan_started: Some(false),
                job_id,
                root: root.view,
                scan,
                already_registered: true,
                original_mutations_enabled: false,
            });
        }
        library.ensure_work_admission()?;
        // Current native object/version checks retain the previous full-byte
        // verification's identity. The following ordinary discovery scan has
        // its own progress/cancellation and never claims a fresh content hash.
        retained.journal.validate_completed_members(&output)?;
        inventory.validate()?;
        let scan = library.register_verified(
            output_path,
            (output_identity, volume),
            "Imported output".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )?;
        let root = store::root(&self.catalog, scan.root_id.clone())?.view;
        Ok(ImportOutputIndex {
            scan_started: Some(true),
            job_id,
            root,
            scan: Some(scan),
            already_registered: false,
            original_mutations_enabled: false,
        })
    }
}
