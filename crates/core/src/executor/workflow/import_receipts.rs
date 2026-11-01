//! Durable verification facts precede job completion. A fact alone is never a
//! completed import: readers also require that exact job's committed success.
use super::*;
use rusqlite::params;

const PHASE: &str = "import_copy_verified_v1";
const MAX: usize = 1024 * 1024;

fn validate(
    view: &ImportCopyReceipt,
    job: &CopyWorkflowJob,
    plan: &import_planning::Stored,
) -> Result<()> {
    let minimum = 4 * plan
        .source
        .files
        .iter()
        .filter(|file| {
            plan.view
                .skip_files
                .iter()
                .any(|skip| skip.file_id == file.id)
        })
        .map(|file| file.bytes as u128)
        .sum::<u128>();
    if view.job_id != job.id
        || view.plan_id != job.plan_id
        || view.copy_review_id != job.review_id
        || view.import_plan_id != plan.view.id
        || view.import_digest != plan.view.digest
        || view.manifest_id != plan.view.manifest_id
        || view.plan_id != plan.view.plan_id
        || view.copied_file_ids != plan.view.copy_file_ids
        || serde_json::to_vec(&view.skipped_files).ok()
            != serde_json::to_vec(&plan.view.skip_files).ok()
        || !view.sources_retained
        || view.verified_at.is_empty()
        || view.verified_at.len() > 128
        || view.keeper_bytes_read.len() > 39
        || view
            .keeper_bytes_read
            .parse::<u128>()
            .is_ok_and(|bytes| bytes < minimum)
        || view.keeper_bytes_read.parse::<u128>().is_err()
        || job.action != CopyAction::CopyFiles
        || job.total_files as usize != view.copied_file_ids.len()
        || job.verified_files != job.total_files
        || job.source_root_id != plan.view.source_root_id
    {
        return Err(failed(
            "Import copy verification evidence differs from its retained job and decisions.",
        ));
    }
    Ok(())
}
impl CopyWorkflow {
    pub(super) fn record_import_copy(
        &self,
        source: &Snapshot,
        job_id: &str,
        keeper_bytes: u128,
    ) -> Result<()> {
        let Some(selection) = source.import_selection.as_ref() else {
            return Ok(());
        };
        let job = operation_jobs::load(&self.catalog, job_id.into())?;
        let source = source.clone();
        let selection = selection.clone();
        self.catalog.call(move|db| {
            let tx=db.transaction()?;
            source.validate(&tx)?;
            let plan=import_planning::load(&tx,&source.owner_id,&selection.id)?;
            let view=ImportCopyReceipt{job_id:job.id.clone(),plan_id:source.owner_id,manifest_id:source.manifest_id,import_plan_id:selection.id,import_digest:selection.digest,copy_review_id:job.review_id.clone(),copied_file_ids:plan.view.copy_file_ids.clone(),skipped_files:plan.view.skip_files.clone(),keeper_bytes_read:keeper_bytes.to_string(),verified_at:crate::library::store::now(),sources_retained:true};
            validate(&view,&job,&plan)?;
            if !matches!(job.state,CopyJobState::Running|CopyJobState::PauseRequested|CopyJobState::CancelRequested) {return Err(failed("Only a running verified copy can record import evidence."));}
            let text=serde_json::to_string(&view).map_err(|error|failed("Could not encode import copy evidence.").caused_by(error))?;
            if text.len()>MAX {return Err(failed("Import copy evidence exceeds its bound."));}
            let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3)",params![view.plan_id,view.job_id,PHASE],|row|row.get(0))?;
            if exists {return Err(failed("This job already has retained import copy evidence."));}
            tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![view.plan_id,view.job_id,PHASE,text,view.verified_at])?;
            tx.commit()?;Ok(())
        })
    }
    pub(crate) fn import_copy_receipt(&self, job_id: String) -> Result<Option<ImportCopyReceipt>> {
        let job = operation_jobs::load(&self.catalog, job_id)?;
        if job.state != CopyJobState::CompletedVerified {
            return Ok(None);
        }
        self.catalog.call(move|db| {
            let rows=db.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=?4 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 LIMIT 2")?.query_map(params![job.plan_id,job.id,PHASE,MAX as i64],|row|row.get::<_,Option<String>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            if rows.is_empty(){return Ok(None);}
            if rows.len()!=1{return Err(failed("Import copy evidence is ambiguous."));}
            let text=rows[0].as_ref().ok_or_else(||failed("Import copy evidence exceeds its bound."))?;
            let view:ImportCopyReceipt=serde_json::from_str(text).map_err(|error|failed("Import copy evidence is invalid.").caused_by(error))?;
            let plan=import_planning::load(db,&job.plan_id,&view.import_plan_id)?;
            validate(&view,&job,&plan)?;
            Ok(Some(view))
        })
    }
}
