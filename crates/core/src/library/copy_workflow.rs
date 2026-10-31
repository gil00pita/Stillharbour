//! ID-only application workflow boundary. The desktop passes destination paths
//! only from its private native picker session, never renderer path strings.
use super::Library;
#[cfg(not(any(test, feature = "copy-workflow")))]
use crate::domain::{AppError, ErrorCode};
use crate::{domain::Result, filesystem::FileIdentity, operations::*};
use std::{path::PathBuf, sync::Arc};

#[cfg(not(any(test, feature = "copy-workflow")))]
fn unavailable() -> AppError {
    AppError::new(
        ErrorCode::Unsupported,
        "Verified organization copying is not enabled in this build; native workflow gates are still being completed.",
    )
}
impl Library {
    pub fn organization_import_comparison_start(
        &self,
        _input: ImportComparisonInput,
    ) -> Result<ImportComparisonJob> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.start_import_comparison(self, _input)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_import_comparison_job(&self, _id: String) -> Result<ImportComparisonJob> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.import_comparison_job(_id)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_import_comparison_cancel(
        &self,
        _id: String,
    ) -> Result<ImportComparisonJob> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.cancel_import_comparison(_id)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_import_comparison_discard(&self, _id: String) -> Result<()> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.discard_import_comparison(_id)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }

    pub fn organization_copy_preparation_reopen(
        &self,
        _job: String,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyPreparationReopened> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .reopen_preparation(self, _job, _selection, _path, _identity)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }

    pub fn organization_copy_effect_match(
        &self,
        _plan: String,
        _manifest: String,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyEffectMatch> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .effect_match(self, _plan, _manifest, _selection, _path, _identity)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }

    pub fn organization_copy_continuation_review(
        &self,
        _job: String,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyContinuationReview> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .create_continuation_review(self, _job, _selection, _path, _identity)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_continuation_approve(
        &self,
        _input: CopyApprovalInput,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyActionApproval> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .approve_continuation(self, _input, _selection, _path, _identity)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_continuation_start(
        self: &Arc<Self>,
        _input: CopyStartInput,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyWorkflowJob> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.start_continuation(
                self.clone(),
                _input,
                _selection,
                _path,
                _identity,
            )
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_continuation_discard(&self, _review: String) -> Result<()> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.discard_continuation(&_review)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_recovery_inspect(
        &self,
        _job_id: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyRecoveryInspection> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .inspect_retained(_job_id, _path, _identity.0, _identity.1)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_inspect(
        &self,
        _id: String,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyInspection> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .inspect(_id, _selection, _path, _identity.0, _identity.1)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_capabilities(&self) -> CopyWorkflowCapabilities {
        let available = cfg!(feature = "copy-workflow") && cfg!(any(windows, target_os = "macos"));
        CopyWorkflowCapabilities{available,reason:if available {"Source-retaining, byte-verified copies only. Original-file mutations remain disabled."} else {"Verified organization copying is not enabled in this build; native workflow gates are still being completed."}.into()}
    }
    pub fn organization_copy_review_create(
        &self,
        _plan: String,
        _manifest: String,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyActionReview> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .create_review(self, _plan, _manifest, _selection, _path, _identity)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_review(
        &self,
        _id: String,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyActionReview> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .get_review(self, _id, _selection, _path, _identity.0, _identity.1)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_approve(
        &self,
        _input: CopyApprovalInput,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyActionApproval> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow
                .approve(self, _input, _selection, _path, _identity.0, _identity.1)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_start(
        self: &Arc<Self>,
        _input: CopyStartInput,
        _selection: String,
        _path: PathBuf,
        _identity: (FileIdentity, String),
    ) -> Result<CopyWorkflowJob> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.start(
                self.clone(),
                _input,
                _selection,
                _path,
                _identity.0,
                _identity.1,
            )
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_job(&self, id: String) -> Result<CopyWorkflowJob> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.job(id)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            super::operation_jobs::load(&self.catalog, id)
        }
    }
    pub fn organization_copy_cancel(&self, _id: String) -> Result<CopyWorkflowJob> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.cancel(&_id)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_io(&self) -> Result<CopyIoStatus> {
        let io = crate::jobs::paced_io::PacedIo::shared();
        Ok(CopyIoStatus {
            aggregate_bytes_per_second: io.bytes_per_second()?.to_string(),
            byte_io_paused: io.is_paused()?,
            burst_bytes: crate::jobs::paced_io::ISSUED_BURST_BYTES as u32,
        })
    }
    pub fn organization_copy_set_io(
        &self,
        _rate: Option<String>,
        _paused: Option<bool>,
    ) -> Result<CopyIoStatus> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.set_io(_rate, _paused)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
    pub fn organization_copy_discard(&self, _id: String) -> Result<()> {
        #[cfg(any(test, feature = "copy-workflow"))]
        {
            self.copy_workflow.discard(&_id)
        }
        #[cfg(not(any(test, feature = "copy-workflow")))]
        {
            Err(unavailable())
        }
    }
}
