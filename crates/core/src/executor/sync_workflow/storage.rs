//! All durable sync state is serialized through the existing catalog actor.
use super::*;

impl SyncWorkflow {
    pub(super) fn create_job(&self, id: &str, binding: &Binding) -> Result<()> {
        let id = id.to_owned();
        let catalog_id = binding.scope.catalog_id.clone();
        let root = binding.scope.root_id.clone();
        let run = binding.scope.run_id.clone();
        let text = encode(binding)?;
        let timestamp = now();
        let action = action_name(binding.view.action).to_owned();
        self.catalog.call(move|db|{let count:u32=db.query_row("SELECT count(*) FROM backup_sync_jobs",[],|r|r.get(0))?;if count>=10_000{return Err(invalid("This catalog reached its retained sync-job bound. History is never silently erased."));}db.execute("INSERT INTO backup_sync_jobs(id,catalog_id,root_id,comparison_run,scope_json,created_at,updated_at,state,phase,action,verified_files,message) VALUES(?1,?2,?3,?4,?5,?6,?6,'queued','queued',?7,0,'Approved exact native action queued; source content and prior history are retained.')",params![id,catalog_id,root,run,text,timestamp,action])?;Ok(())})
    }
    pub(super) fn binding(&self, id: &str) -> Result<Binding> {
        crate::operations::validate_id(id)?;
        let id = id.to_owned();
        let expected = self.catalog.summary()?.id;
        self.catalog.call(move|db|{let (catalog,root,run,text):(String,String,String,Option<String>)=db.query_row("SELECT catalog_id,root_id,comparison_run,CASE WHEN length(CAST(scope_json AS BLOB))<=33554432 THEN scope_json END FROM backup_sync_jobs WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;let binding:Binding=decode(&text.ok_or_else(||failed("Oversized sync binding."))?)?;if catalog!=expected || binding.scope.catalog_id!=catalog || binding.scope.root_id!=root || binding.scope.run_id!=run || binding.view.run_id!=run || binding.view.source_root_id!=root{return Err(failed("Sync binding belongs to another catalog/root/review."));}Ok(binding)})
    }
    pub(super) fn update(
        &self,
        id: &str,
        state: SyncState,
        phase: &str,
        action: SyncAction,
        ready: Option<String>,
        message: &str,
    ) -> Result<()> {
        let id = id.to_owned();
        let state = state_name(state).to_owned();
        let phase = phase.to_owned();
        let action = action_name(action).to_owned();
        let message = message.to_owned();
        self.catalog.call(move|db|{if db.execute("UPDATE backup_sync_jobs SET state=?1,phase=?2,action=?3,ready_review_id=?4,message=?5,updated_at=?6 WHERE id=?7",params![state,phase,action,ready,message,now(),id])?!=1{return Err(failed("Sync job disappeared."));}Ok(())})
    }
    pub(super) fn job(&self, id: &str) -> Result<SyncJob> {
        let binding = self.binding(id)?;
        let key = id.to_owned();
        let (state,phase,action,created,updated,verified,ready,message):(String,String,String,String,String,u32,Option<String>,String)=self.catalog.call(move|db|Ok(db.query_row("SELECT state,phase,action,created_at,updated_at,verified_files,ready_review_id,message FROM backup_sync_jobs WHERE id=?",[key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))?))?;
        let state = serde_json::from_value(serde_json::Value::String(state))
            .map_err(|e| failed("Invalid sync job state.").caused_by(e))?;
        let action = serde_json::from_value(serde_json::Value::String(action))
            .map_err(|e| failed("Invalid sync job action.").caused_by(e))?;
        if verified > binding.view.copy_files {
            return Err(failed("Verified sync count exceeds its immutable scope."));
        }
        Ok(SyncJob {
            id: id.into(),
            run_id: binding.scope.run_id,
            source_root_id: binding.scope.root_id,
            action,
            state,
            phase,
            created_at: created,
            updated_at: updated,
            source_display_path: binding.view.source_display_path,
            dataset_display_path: binding.view.dataset_display_path,
            total_files: binding.view.copy_files,
            verified_files: verified,
            copy_bytes: binding.view.copy_bytes,
            uncopied_source_revisions: binding.view.uncopied_source_revisions,
            retained_destination_files: binding.view.retained_destination_files,
            ready_review_id: ready,
            bytes_per_second: PacedIo::shared().bytes_per_second()?.to_string(),
            message,
        })
    }
    pub(super) fn list(&self) -> Result<Vec<SyncJob>> {
        let ids = self.catalog.call(|db| {
            let mut query = db.prepare(
                "SELECT id FROM backup_sync_jobs ORDER BY created_at DESC,id DESC LIMIT 128",
            )?;
            Ok(query
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?)
        })?;
        ids.iter().map(|id| self.job(id)).collect()
    }
    pub(super) fn bind_plan(&self, id: &str, plan: &CopyPlan) -> Result<()> {
        let id = id.to_owned();
        let plan = plan.clone();
        let encoded = encode(&plan)?;
        if plan.digest()? != plan.digest {
            return Err(failed("Invalid immutable sync copy plan."));
        }
        self.catalog.call(move |db| {
            db.execute(
                "INSERT INTO backup_sync_plans(job_id,digest,plan_json) VALUES(?1,?2,?3)",
                params![id, plan.digest, encoded],
            )?;
            Ok(())
        })
    }
    pub(super) fn plan(&self, id: &str) -> Result<Option<CopyPlan>> {
        let id = id.to_owned();
        self.catalog.call(move|db|{let value:Option<(String,Option<String>)>=db.query_row("SELECT digest,CASE WHEN length(CAST(plan_json AS BLOB))<=33554432 THEN plan_json END FROM backup_sync_plans WHERE job_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;value.map(|(digest,text)|{let plan:CopyPlan=decode(&text.ok_or_else(||failed("Oversized sync plan."))?)?;if plan.digest!=digest || plan.digest()?!=digest{return Err(failed("Sync plan digest changed."));}Ok(plan)}).transpose()})
    }
    pub(super) fn event(
        &self,
        id: &str,
        item: Option<usize>,
        phase: &str,
        evidence: serde_json::Value,
    ) -> Result<()> {
        let id = id.to_owned();
        let item = item
            .map(i64::try_from)
            .transpose()
            .map_err(|_| failed("Sync item index overflow."))?;
        let phase = phase.to_owned();
        let text = encode(&evidence)?;
        if text.len() > 65536 {
            return Err(failed("Sync event exceeds its 64 KiB bound."));
        }
        self.catalog.call(move|db|{db.execute("INSERT INTO backup_sync_events(job_id,ordinal,item,phase,evidence) SELECT ?1,COALESCE(MAX(ordinal),0)+1,?2,?3,?4 FROM backup_sync_events WHERE job_id=?1",params![id,item,phase,text])?;Ok(())})
    }
    pub(super) fn events(
        &self,
        id: &str,
    ) -> Result<Vec<(Option<usize>, String, serde_json::Value)>> {
        let id = id.to_owned();
        self.catalog.call(move|db|{
            let mut query=db.prepare("SELECT ordinal,item,phase,CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM backup_sync_events WHERE job_id=? ORDER BY ordinal LIMIT 200001")?;
            let mut rows=query.query([id])?;let mut result=Vec::new();let mut total_bytes=0usize;
            while let Some(row)=rows.next()? {let ordinal:i64=row.get(0)?;if ordinal!=result.len() as i64+1 || result.len()>=200_000{return Err(failed("Sync event sequence is discontinuous or exceeds its bound."));}let item:Option<i64>=row.get(1)?;let item=item.map(usize::try_from).transpose().map_err(|_|failed("Invalid retained sync item."))?;let phase:String=row.get(2)?;let text:Option<String>=row.get(3)?;let text=text.ok_or_else(||failed("Oversized sync event."))?;total_bytes=total_bytes.saturating_add(text.len());if total_bytes>64*1024*1024{return Err(failed("Sync history exceeds its 64 MiB inspection bound."));}let evidence=decode(&text)?;result.push((item,phase,evidence));}Ok(result)
        })
    }
}

pub(super) struct SyncJournal {
    catalog: Arc<Catalog>,
    job: String,
    plan: CopyPlan,
    retained: bool,
}
impl SyncJournal {
    pub(super) fn new(catalog: Arc<Catalog>, job: String, plan: CopyPlan) -> Result<Self> {
        if plan.digest()? != plan.digest {
            return Err(failed("Invalid sync journal plan."));
        }
        let expected = encode(&plan)?;
        let id = job.clone();
        let digest = plan.digest.clone();
        catalog.call(move |db| {
            let retained: (String, String) = db.query_row(
                "SELECT digest,plan_json FROM backup_sync_plans WHERE job_id=?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if retained != (digest, expected) {
                return Err(failed("Sync journal anchor changed."));
            }
            Ok(())
        })?;
        Ok(Self {
            catalog,
            job,
            plan,
            retained: false,
        })
    }
    pub(super) fn new_retained(catalog: Arc<Catalog>, job: String, plan: CopyPlan) -> Result<Self> {
        let mut journal = Self::new(catalog, job, plan)?;
        journal.retained = true;
        Ok(journal)
    }
    pub(super) fn history(&self) -> Result<(Vec<resume::Cursor>, bool, String)> {
        let id = self.job.clone();
        let plan = self.plan.clone();
        let retained = self.retained;
        self.catalog.call(move|db|{let mut query=db.prepare("SELECT item,phase,CASE WHEN length(CAST(evidence AS BLOB))<=8192 THEN evidence END FROM backup_sync_events WHERE job_id=? ORDER BY ordinal LIMIT 200001")?;let mut rows=query.query([id])?;let mut events=Vec::new();let mut total_bytes=0usize;while let Some(row)=rows.next()?{let phase:String=row.get(1)?;if let Some(phase)=phase.strip_prefix("copy/"){let item:Option<i64>=row.get(0)?;let item=item.map(usize::try_from).transpose().map_err(|_|failed("Invalid copy event index."))?;let text:Option<String>=row.get(2)?;let text=text.ok_or_else(||failed("Oversized copy event."))?;total_bytes=total_bytes.saturating_add(text.len());if total_bytes>64*1024*1024{return Err(failed("Copy history exceeds its 64 MiB inspection bound."));}let evidence=decode(&text)?;events.push((events.len() as u64+1,item,phase.into(),evidence));}else if !(phase.starts_with("prepare/") && !retained || phase.starts_with("retained/") && retained){return Err(failed("Unknown sync journal phase."));}}super::super::catalog_journal::decode_copy_events(&plan,events)})
    }
}
impl EventJournal for SyncJournal {
    fn append(&self, item: Option<usize>, phase: &str, evidence: serde_json::Value) -> Result<()> {
        let id = self.job.clone();
        let phase = phase.to_owned();
        let text = encode(&evidence)?;
        let item = item
            .map(i64::try_from)
            .transpose()
            .map_err(|_| failed("Copy item overflow."))?;
        if text.len() > 8192 {
            return Err(failed("Copy event exceeds 8 KiB."));
        }
        self.catalog.call(move|db|{let tx=db.transaction()?;tx.execute("INSERT INTO backup_sync_events(job_id,ordinal,item,phase,evidence) SELECT ?1,COALESCE(MAX(ordinal),0)+1,?2,?3,?4 FROM backup_sync_events WHERE job_id=?1",params![id,item,format!("copy/{phase}"),text])?;if phase=="item_completed_verified" {tx.execute("UPDATE backup_sync_jobs SET verified_files=verified_files+1,phase='verifying',updated_at=? WHERE id=?",params![now(),id])?;}tx.commit()?;Ok(())})
    }
}
impl CopyJournal for SyncJournal {
    fn history_count(&self) -> Result<u64> {
        let id = self.job.clone();
        self.catalog.call(move |db| {
            let count: i64 = db.query_row(
                "SELECT count(*) FROM backup_sync_events WHERE job_id=? AND phase LIKE 'copy/%'",
                [id],
                |r| r.get(0),
            )?;
            u64::try_from(count).map_err(|_| failed("Invalid copy history count."))
        })
    }
    fn stage_identity(&self, item: usize) -> Result<Option<filesystem::FileIdentity>> {
        let (cursors, _, _) = self.history()?;
        Ok(cursors
            .get(item)
            .and_then(|c| c.object.as_ref())
            .map(|o| filesystem::FileIdentity {
                volume: o.volume,
                file: o.file,
                links: 1,
            }))
    }
}
