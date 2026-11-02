//! Explicit snapshot admission for unique OLD readable-copy destination data.
//! Only a fresh native comparison can produce this scope; profile source roots,
//! display paths, saved catalogs and renderer claims never authorize it.
use super::Library;
use crate::{backup::snapshots::SnapshotReview, domain::Result};

impl Library {
    pub(crate) fn backup_old_destination_review(
        &self,
        run_id: String,
        entry_id: String,
        profile_id: String,
    ) -> Result<SnapshotReview> {
        crate::operations::validate_id(&profile_id)?;
        let files = self
            .sync_workflow
            .recovery_snapshot_scope(self, run_id, entry_id)?;
        // This creates an immutable explicit backup preview. It does not issue
        // an approval, run a helper, or authorize retained replacement.
        self.snapshots
            .review_recovery_backup(self, &profile_id, files)
    }
}
