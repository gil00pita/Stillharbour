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
            job_id,
            root,
            scan: Some(scan),
            already_registered: false,
            original_mutations_enabled: false,
        })
    }
}
