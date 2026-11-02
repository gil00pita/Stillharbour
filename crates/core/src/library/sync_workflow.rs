//! Application-facing ID-only readable-copy boundary.
use super::Library;
#[cfg(not(any(test, feature = "backup-workflow", feature = "fixture-probes")))]
use crate::domain::{AppError, ErrorCode};
use crate::{backup::sync::*, domain::Result};
use std::sync::Arc;

impl Library {
    pub fn backup_shutdown_ready(&self) -> Result<bool> {
        if !self.snapshots.idle()? {
            return Ok(false);
        }
        #[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
        if !self.sync_workflow.idle()? {
            return Ok(false);
        }
        Ok(true)
    }
    #[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
    pub(crate) fn sync_recovery_proof(
        &self,
        files: &[crate::backup::sync::RecoveryFile],
        evidence_id: &str,
    ) -> Result<crate::backup::sync::RecoveryProof> {
        self.snapshots.verify_recovery(files, evidence_id)
    }
    pub fn backup_sync(self: &Arc<Self>, request: SyncRequest) -> Result<SyncReply> {
        #[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
        {
            self.sync_workflow.handle(self.clone(), request)
        }
        #[cfg(not(any(test, feature = "backup-workflow", feature = "fixture-probes")))]
        {
            match request {
                SyncRequest::Capabilities {} => Ok(SyncReply::Capabilities(SyncCapabilities {
                    execution_available: false, retained_replacement_available: false,
                    reason: "Readable-copy execution is disabled in this build until native application and recovery gates pass.".into(),
                    io_scope: "The shared application-issued byte budget covers comparison, copying and verification; metadata, helpers, OS caches and physical traffic are outside that cap.".into(),
                    bytes_per_second: crate::jobs::paced_io::PacedIo::shared().bytes_per_second()?.to_string(),
                })),
                _ => Err(AppError::new(ErrorCode::MutationUnavailable,"Readable-copy actions are disabled in this build; native workflow gates are still being completed.")),
            }
        }
    }
}
