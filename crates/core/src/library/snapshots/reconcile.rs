//! Explicit cold-process inspection of a previously approved backup. This never
//! replays its upload, adopts a partial restore, or unlocks/repairs a repository.
use super::*;

impl SnapshotWorkflow {
    pub(super) fn reconcile(
        self: &Arc<Self>,
        library: Arc<Library>,
        profile_id: &str,
        original_id: &str,
        requested: Option<String>,
    ) -> Result<SnapshotJob> {
        self.require_execution()?;
        id(original_id)?;
        if let Some(key) = &requested {
            snapshot_id(key)?;
        }
        let profile = self.profile(profile_id)?;
        self.authorized(&profile)?;
        let key = original_id.to_string();
        let text=self.catalog.call(move|db|Ok(db.query_row("SELECT CASE WHEN length(CAST(record AS BLOB))<=33554432 THEN record END FROM backup_snapshot_jobs WHERE id=?",[key],|row|row.get::<_,String>(0))?))?;
        let original: JobRecord = decode(text)?;
        if original.view.id != original_id
            || !self.lineage(&profile)?.contains(&original.view.profile_id)
            || original.profile.namespace_id != profile.namespace_id
        {
            return Err(conflict(
                "The interrupted job belongs to another immutable repository lineage.",
            ));
        }
        if original.view.action != "backup" {
            return Err(conflict(
                "Partial restores remain retained. Verify the recorded snapshot and create a fresh protected-output restore review; no restore is resumed.",
            ));
        }
        if original.view.state == "completed_verified" {
            return Err(conflict(
                "This backup already has a durable completion. Use concrete snapshot verification instead.",
            ));
        }
        validate_manifest(&original.manifest, &profile)?;
        if original.manifest.catalog_digest.is_none() || original.manifest.catalog_bytes == 0 {
            return Err(conflict(
                "The job did not persist its complete staged manifest before upload. Its copies remain retained; create a fresh review instead of replaying its approval.",
            ));
        }
        let binding = self
            .repository(&profile)?
            .ok_or_else(|| conflict("The interrupted job has no durable repository binding."))?;
        if binding.repository_id.is_none()
            || binding.repository_id != original.manifest.repository_id
        {
            return Err(conflict(
                "Repository initialization or identity is uncertain. No upload or adoption is authorized.",
            ));
        }
        let scopes = if let Some(root) = &profile.local {
            vec![Scope::new(
                DirectoryChain::open(&root.path()?)?,
                Access::Write,
            )]
        } else {
            vec![Scope::new(
                DirectoryChain::open(&self.data.path()?)?,
                Access::Read,
            )]
        };
        let lease = Registry::shared().try_acquire(scopes)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let mut job=JobRecord{verified_output:None,metadata_verified:false,private_scope:original.private_scope,review_id:uuid::Uuid::new_v4().to_string(),view:SnapshotJob{id:uuid::Uuid::new_v4().to_string(),profile_id:profile_id.into(),action:"reconcile".into(),state:"queued".into(),phase:"cold_inspection".into(),files:original.view.files,bytes:original.view.bytes.clone(),snapshot_id:None,message:"Inspect existing job-tagged snapshots and compare a fresh restore; no backup is replayed.".into(),started_at:now(),finished_at:None,included:false,structural_checked:false,data_read_verified:false,restored_compared:false,source_retained:true,independent_recovery_verified:false},profile,manifest:original.manifest.clone(),work:None,restore:None,subfolder:None};
        let reply = job.view.clone();
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| unavailable("Snapshot state is unavailable."))?;
            if state.active.is_some() || !state.authorized.contains(profile_id) {
                return Err(error(
                    ErrorCode::Busy,
                    "Another job is active or this repository session was revoked.",
                ));
            }
            self.save_job(&job)?;
            self.event(&job.view.id,"cold_inspection_intent",serde_json::json!({"original_job_id":original_id,"requested_snapshot_id":requested,"frozen_manifest_digest":digest(&original.manifest)?}))?;
            state.active = Some(Active {
                id: job.view.id.clone(),
                roots: job.profile.view.root_ids.clone(),
                cancel: cancel.clone(),
            });
        }
        let workflow = self.clone();
        let spawn = std::thread::Builder::new()
            .name("snapshot-reconcile".into())
            .spawn(move || {
                let _lease = lease;
                let _library = library;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    workflow.inspect_interrupted(&mut job, original, requested, &cancel)
                }));
                let _ = workflow.finish_job(&mut job, &cancel, result);
            });
        if spawn.is_err() {
            self.state
                .lock()
                .map_err(|_| unavailable("Snapshot state is unavailable."))?
                .active = None;
            return Err(error(
                ErrorCode::WorkerFailed,
                "The cold inspection worker could not start. Its queued record remains for inspection.",
            ));
        }
        Ok(reply)
    }

    fn inspect_interrupted(
        &self,
        job: &mut JobRecord,
        mut original: JobRecord,
        requested: Option<String>,
        cancel: &AtomicBool,
    ) -> Result<()> {
        self.authorized(&job.profile)?;
        job.view.state = "running".into();
        let work = self.workspace(&job.view.id)?;
        job.work = Some(Binding::capture(
            work.path.clone(),
            work.identity.clone(),
            work.volume.clone(),
        )?);
        self.phase(job, "repository_inspection")?;
        let binding = self
            .repository(&job.profile)?
            .ok_or_else(|| conflict("The repository binding disappeared."))?;
        let repository_id = self.read_repository_id(&job.profile, &binding, Some(&work), cancel)?;
        if repository_id
            != original
                .manifest
                .repository_id
                .clone()
                .ok_or_else(|| conflict("Missing concrete repository identity."))?
        {
            return Err(conflict("The interrupted repository identifier changed."));
        }
        let context = self.context(&job.profile, &binding, Some(&work), cancel)?;
        let tag = format!("stillharbour-job:{}", original.view.id);
        let output = context.run(&["snapshots", "--tag", &tag], cancel)?;
        let candidates: serde_json::Value = serde_json::from_slice(&output)
            .map_err(|_| invalid("Snapshot inspection returned invalid structured data."))?;
        let candidates = candidates
            .as_array()
            .ok_or_else(|| invalid("Snapshot inspection did not return a bounded list."))?;
        if candidates.len() > 1000 {
            return Err(invalid(
                "The interrupted job has too many candidate snapshots.",
            ));
        }
        let mut ids = Vec::new();
        for candidate in candidates {
            let key = candidate
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| invalid("Snapshot inspection lacks a concrete identifier."))?;
            snapshot_id(key)?;
            if !candidate
                .get("tags")
                .and_then(|value| value.as_array())
                .is_some_and(|tags| tags.iter().any(|value| value.as_str() == Some(&tag)))
            {
                return Err(conflict(
                    "A candidate does not carry the exact frozen job tag.",
                ));
            }
            ids.push(key.to_string());
        }
        ids.sort();
        ids.dedup();
        let key = match requested {
            Some(key) if ids.contains(&key) => key,
            Some(_) => {
                return Err(conflict(
                    "The selected snapshot does not carry this exact interrupted job tag.",
                ));
            }
            None if ids.len() == 1 => ids.remove(0),
            None if ids.is_empty() => {
                return Err(conflict(NO_COMPLETED_SNAPSHOT_MESSAGE));
            }
            None => {
                return Err(error(
                    ErrorCode::Conflict,
                    &format!(
                        "Choose one concrete snapshot identifier for this ambiguous job: {}",
                        ids.iter().take(8).cloned().collect::<Vec<_>>().join(", ")
                    ),
                ));
            }
        };
        let payload_name = format!("payload-{}", original.view.id);
        let subfolder =
            self.inspect_snapshot(&context, &key, &payload_name, &original.manifest, cancel)?;
        job.view.snapshot_id = Some(key.clone());
        job.view.included = true;
        job.subfolder = Some(subfolder.clone());
        self.phase(job, "existing_snapshot_included")?;
        self.verify_job(job, &context, &work, cancel)?;
        let record = Record {
            metadata_verified: job.metadata_verified,
            view: SnapshotRecord {
                id: key.clone(),
                profile_id: original.view.profile_id.clone(),
                time: original.view.started_at.clone(),
                files: original.view.files,
                bytes: original.view.bytes.clone(),
                root_ids: original.profile.view.root_ids.clone(),
                included: true,
                structural_checked: true,
                data_read_verified: true,
                restored_compared: true,
                catalog_included: true,
                source_retained: true,
            },
            manifest: original.manifest.clone(),
            subfolder,
            repository_id,
            profile_namespace: job.profile.namespace_id.clone(),
        };
        let record_id = key.clone();
        let text=self.catalog.call(move|db|Ok(db.query_row("SELECT CASE WHEN length(CAST(record AS BLOB))<=33554432 THEN record END FROM backup_snapshot_records WHERE id=?",[record_id],|row|row.get::<_,String>(0)).optional()?))?;
        if let Some(text) = text {
            let prior: Record = decode(text)?;
            if prior.profile_namespace != record.profile_namespace
                || prior.repository_id != record.repository_id
                || digest(&prior.manifest)? != digest(&record.manifest)?
                || prior.subfolder != record.subfolder
            {
                return Err(conflict(
                    "An existing immutable record disagrees with this concrete snapshot proof.",
                ));
            }
        } else {
            let text = encode(&record)?;
            let profile = record.view.profile_id.clone();
            let id = key.clone();
            self.catalog.call(move |db| {
                db.execute(
                    "INSERT INTO backup_snapshot_records VALUES(?1,?2,?3)",
                    params![id, profile, text],
                )?;
                Ok(())
            })?;
        }
        original.view.snapshot_id = Some(key);
        original.view.included = true;
        original.view.structural_checked = true;
        original.view.data_read_verified = true;
        original.view.restored_compared = true;
        original.metadata_verified = job.metadata_verified;
        original.subfolder = job.subfolder.clone();
        original.view.state = "completed_verified".into();
        original.view.finished_at = Some(now());
        original.view.message="The existing concrete snapshot was reconciled through fresh structure/data-read and restored byte/catalog/native-metadata checks. No upload approval was replayed.".into();
        self.phase(&mut original, "reconciled_existing_snapshot")?;
        job.view.state = "completed_verified".into();
        job.view.message="Existing snapshot reconciled and fully compared in a fresh owned output; no source file, partial restore or repository lock was removed.".into();
        self.event(&job.view.id,"cold_inspection_completed",serde_json::json!({"original_job_id":original.view.id,"snapshot_id":job.view.snapshot_id,"frozen_manifest_digest":digest(&job.manifest)?}))?;
        Ok(())
    }
}
