//! Fresh concrete snapshot restore proof for executor-owned relocated OLD objects.
use super::*;
use std::io::{Seek, SeekFrom};

impl SnapshotWorkflow {
    pub(crate) fn verify_retained_recovery(
        &self,
        scope: &crate::executor::sync_workflow::RetainedRecoveryScope<'_>,
        evidence_id: &str,
    ) -> Result<crate::backup::sync::RecoveryProof> {
        let files = scope.originals();
        let current_objects = scope.current()?;
        let expected_indices = (0..files.len()).collect::<BTreeSet<_>>();
        if current_objects.len() != files.len()
            || current_objects
                .iter()
                .map(|object| object.index())
                .collect::<BTreeSet<_>>()
                != expected_indices
        {
            return Err(conflict(
                "Retained recovery requires every unique original group index.",
            ));
        }
        snapshot_id(evidence_id)?;
        if files.is_empty() || files.len() > MAX_FILES {
            return Err(invalid(
                "Exact recovery requires a nonempty bounded backend scope.",
            ));
        }
        let key = evidence_id.to_string();
        let text = self
            .catalog
            .call(move |db| {
                Ok(db
                    .query_row(
                        "SELECT CASE WHEN length(CAST(record AS BLOB))<=33554432 THEN record END FROM backup_snapshot_records WHERE id=?",
                        [key],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?)
            })?
            .ok_or_else(|| {
                conflict("No concrete application snapshot records this recovery evidence.")
            })?;
        let record: Record = decode(text)?;
        if !record.view.included
            || !record.view.structural_checked
            || !record.view.data_read_verified
            || !record.view.restored_compared
        {
            return Err(conflict(
                "Snapshot evidence lacks full inclusion/read/restore byte proof.",
            ));
        }
        self.require_execution()?;
        let authorized = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .authorized
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut profiles = Vec::new();
        for id in authorized {
            let profile = self.profile(&id)?;
            if profile.namespace_id == record.profile_namespace
                && self.lineage(&profile)?.contains(&record.view.profile_id)
            {
                profiles.push(profile);
            }
        }
        profiles.sort_by(|a, b| b.view.created_at.cmp(&a.view.created_at));
        let profile=profiles.into_iter().next().ok_or_else(||conflict("Explicitly open this repository's native profile session before requesting recovery proof."))?;
        self.authorized(&profile)?;
        validate_manifest(&record.manifest, &profile)?;
        let repository = self
            .repository(&profile)?
            .ok_or_else(|| conflict("Concrete recovery repository binding is missing."))?;
        if repository.repository_id.as_deref() != Some(&record.repository_id) {
            return Err(conflict(
                "The recovery record belongs to another repository identifier.",
            ));
        }
        let repository_root=repository.local.as_ref().ok_or_else(||unavailable("Remote failure-domain independence is unknown; loopback/disposable servers never certify an independent role."))?;
        let repository_handle = repository_root.open()?;
        let mut historical_scope = Vec::new();
        let mut seen = BTreeSet::new();
        for (index, file) in files.iter().enumerate() {
            if !seen.insert((&file.root_id, &file.relative_path)) {
                return Err(invalid(
                    "The exact recovery request repeats an object path.",
                ));
            }
            let root=record.manifest.roots.iter().find(|r|r.root.id==file.root_id&&r.root.binding.path==file.native_root&&r.root.binding.identity==file.root_identity&&r.root.binding.volume==file.root_volume).ok_or_else(||conflict("The evidence does not include this exact authorized dataset root; source-root backups cannot protect an unrelated old destination."))?;
            root.root.binding.open()?;
            let captured = record.manifest.files.iter().find(|f| {
                f.root_id == file.root_id
                    && f.relative == file.relative_path
                    && f.identity == file.identity
                    && f.revision == file.revision
                    && f.bytes == file.bytes
                    && f.digest == file.full_digest
            });
            if file.catalog_id != record.manifest.catalog_id || captured.is_none() {
                return Err(conflict(
                    "The recovery snapshot does not protect the exact requested old version and companions.",
                ));
            }
            let captured = captured.ok_or_else(|| conflict("Missing exact recovery file."))?;
            let metadata=captured.metadata.as_ref().ok_or_else(||unavailable("This exact snapshot version has byte-only evidence; native essential metadata is unavailable."))?;
            if file.essential_metadata_digest.as_deref() != Some(metadata.digest()?.as_str())
                || !metadata.certified_replacement()?
            {
                return Err(unavailable(
                    "Exact essential metadata, including native security/audit state, is not certified for this replacement scope.",
                ));
            }
            let current_object = current_objects
                .iter()
                .find(|object| object.index() == index)
                .ok_or_else(|| conflict("Incomplete executor-owned retained recovery group."))?;
            let mut current = current_object.file().try_clone()?;
            current.seek(SeekFrom::Start(0))?;
            if !crate::backup::failure_domains::independent_local(&current, &repository_handle)? {
                return Err(unavailable(
                    "The exact source file and repository share a physical failure domain, or independence is unknown.",
                ));
            }
            if filesystem::identity(&current)? != file.identity
                || native::revision(&current)? != current_object.revision()
                || current.metadata()?.len() != file.bytes
                || bridge::digest(&mut current, &AtomicBool::new(false))? != file.full_digest
            {
                return Err(conflict(
                    "The requested exact old version changed before fresh recovery verification.",
                ));
            }
            metadata.compare(&current)?;
            historical_scope.push((
                &file.catalog_id,
                &file.root_id,
                &file.native_root,
                &file.root_identity,
                &file.root_volume,
                &file.relative_path,
                &file.identity,
                &file.revision,
                file.bytes,
                &file.full_digest,
                &file.essential_metadata_digest,
            ));
        }
        historical_scope.sort_by(|a, b| (&a.1, &a.5).cmp(&(&b.1, &b.5)));
        let scope_digest = digest(&historical_scope)?;
        let _lease = Registry::shared().try_acquire(vec![Scope::new(
            DirectoryChain::open(&repository_root.path()?)?,
            Access::Write,
        )])?;
        let job_id = uuid::Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| unavailable("Snapshot state is unavailable."))?;
            if state.active.is_some() {
                return Err(error(
                    ErrorCode::Busy,
                    "Another snapshot operation is active.",
                ));
            }
            state.active = Some(Active {
                id: job_id.clone(),
                roots: files.iter().map(|f| f.root_id.clone()).collect(),
                cancel: cancel.clone(),
            });
        }
        let _activity = RecoveryActivity {
            workflow: self,
            id: job_id.clone(),
        };
        let view=SnapshotJob{id:job_id,profile_id:profile.view.id.clone(),action:"verify".into(),state:"queued".into(),phase:"exact_recovery_review".into(),files:record.view.files,bytes:record.view.bytes.clone(),snapshot_id:Some(evidence_id.into()),message:"Fresh exact-version recovery verification; original and destination files remain unchanged.".into(),started_at:now(),finished_at:None,included:false,structural_checked:false,data_read_verified:false,restored_compared:false,source_retained:true,independent_recovery_verified:false};
        let review = Review {
            private_scope: false,
            view: SnapshotReview {
                items: review_items(&record.manifest),
                id: uuid::Uuid::new_v4().to_string(),
                profile_id: profile.view.id.clone(),
                action: "verify".into(),
                digest: record.manifest.reviewed_digest()?,
                files: record.view.files,
                bytes: record.view.bytes.clone(),
                root_ids: record.view.root_ids.clone(),
                catalog_included: true,
                source_display_paths: Vec::new(),
                source_retained: true,
                target_display: "Fresh owned exact-recovery verification output".into(),
                issues: Vec::new(),
                execution_available: true,
                unavailable_reason: None,
            },
            profile: profile.clone(),
            manifest: record.manifest.clone(),
            restore: None,
            record: Some(record.clone()),
            created: Instant::now(),
        };
        let mut job = JobRecord {
            verified_output: None,
            metadata_verified: false,
            private_scope: false,
            review_id: review.view.id.clone(),
            view,
            profile,
            manifest: record.manifest.clone(),
            work: None,
            restore: None,
            subfolder: Some(record.subfolder.clone()),
        };
        self.save_job(&job)?;
        self.event(
            &job.view.id,
            "exact_recovery_scope",
            serde_json::json!({"evidence_id":evidence_id,"scope_digest":scope_digest}),
        )?;
        let result = (|| {
            self.execute(&mut job, &review, &cancel)?;
            let output = job
                .verified_output
                .as_ref()
                .ok_or_else(|| conflict("Fresh recovery output was not recorded."))?;
            let reopened = scope.current()?;
            if reopened.len() != files.len()
                || reopened
                    .iter()
                    .map(|object| object.index())
                    .collect::<BTreeSet<_>>()
                    != expected_indices
            {
                return Err(conflict(
                    "Retained recovery group changed during fresh restore.",
                ));
            }
            for (index, file) in files.iter().enumerate() {
                let captured = record
                    .manifest
                    .files
                    .iter()
                    .find(|f| f.root_id == file.root_id && f.relative == file.relative_path)
                    .ok_or_else(|| conflict("Exact recovery scope disappeared."))?;
                let metadata = captured
                    .metadata
                    .as_ref()
                    .ok_or_else(|| conflict("Essential metadata proof disappeared."))?;
                let restored = native::open_asset(
                    &output.path()?,
                    &output.identity,
                    &output.volume,
                    Path::new(&captured.archive),
                )?;
                metadata.compare(&restored)?;
                let current_object = reopened
                    .iter()
                    .find(|object| object.index() == index)
                    .ok_or_else(|| conflict("Retained recovery group became incomplete."))?;
                let initial = current_objects
                    .iter()
                    .find(|object| object.index() == index)
                    .ok_or_else(|| conflict("Initial retained recovery witness disappeared."))?;
                if current_object.revision() != initial.revision() {
                    return Err(conflict(
                        "Retained OLD object changed during fresh recovery restore.",
                    ));
                }
                let current = current_object.file();
                if filesystem::identity(current)? != file.identity
                    || native::revision(current)? != current_object.revision()
                {
                    return Err(conflict(
                        "The exact old version changed during recovery verification.",
                    ));
                }
                metadata.compare(current)?;
                if !crate::backup::failure_domains::independent_local(
                    current,
                    &repository_root.open()?,
                )? {
                    return Err(conflict(
                        "Physical recovery independence changed during verification.",
                    ));
                }
            }
            Ok(())
        })();
        job.view.finished_at = Some(now());
        if result.is_ok() {
            job.view.independent_recovery_verified = true;
        } else {
            job.view.state = "needs_recovery".into();
            job.view.message =
                "Fresh exact recovery verification failed; replacement is not authorized.".into();
        }
        self.phase(&mut job, "terminal")?;
        result?;
        Ok(crate::backup::sync::RecoveryProof {
            evidence_id: evidence_id.into(),
            scope_digest,
            independent_failure_domain: true,
            essential_metadata_verified: true,
        })
    }
}
