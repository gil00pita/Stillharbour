//! Same-catalog, current-schema activation review. Preparation never replaces
//! the active SQLite file. The shell must freeze admission before calling it.
use super::*;
use crate::catalog_restore::{
    self as activation, CatalogFileBinding, CatalogRootMapping, PreparedCatalogRestore,
};

#[derive(Default)]
pub(super) struct CatalogRestoreState {
    reviews: BTreeMap<String, ActivationReview>,
    approvals: BTreeMap<String, (String, String, Instant)>,
}
#[derive(Clone)]
struct ActivationReview {
    view: CatalogRestoreReview,
    source: CatalogFileBinding,
    roots: Vec<CatalogRootMapping>,
    mappings: BTreeMap<String, CatalogRootMapping>,
    schema: Vec<activation::SchemaObject>,
    created: Instant,
}
const ACTIVATION_WARNING: &str = "Activation selects the complete restored catalog after closing and reopening Stillharbour. Current catalog edits are retained in a separate consistent pre-restore catalog; records are not merged. Every root must be confirmed again. Saved schedules and mutation approvals do not grant execution authority.";
fn at_phase(phase: &str, mut failure: AppError) -> AppError {
    failure.message = format!(
        "Catalog activation {phase} did not complete. {}",
        failure.message
    );
    failure
}
fn view_digest(review: &ActivationReview) -> Result<String> {
    digest(&(
        review.view.restore_job_id.as_str(),
        review.view.snapshot_id.as_str(),
        review.view.current_revision.as_str(),
        review.view.restored_revision.as_str(),
        &review.source,
        &review.roots,
        &review.mappings,
    ))
}
fn require_fresh(workflow: &SnapshotWorkflow, review: &ActivationReview) -> Result<()> {
    if review.created.elapsed() > REVIEW_AGE
        || store::revision(&workflow.catalog)? != review.view.current_revision
    {
        return Err(conflict(
            "Catalog edits or the activation review changed. Create a fresh review before activating.",
        ));
    }
    activation::validate_frozen(&review.source)?;
    if workflow.catalog.schema_fingerprint()? != review.schema {
        return Err(conflict(
            "The active catalog schema changed after activation review.",
        ));
    }
    Ok(())
}
impl SnapshotWorkflow {
    fn catalog_restore_profile(&self, original: &str) -> Result<ProfileRecord> {
        let original = self.profile(original)?;
        let authorized = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .authorized
            .clone();
        let mut best: Option<(usize, ProfileRecord)> = None;
        for key in authorized {
            let profile = self.profile(&key)?;
            if profile.namespace_id != original.namespace_id {
                continue;
            }
            let lineage = self.lineage(&profile)?;
            if lineage.contains(&original.view.id)
                && best
                    .as_ref()
                    .is_none_or(|(depth, _)| lineage.len() > *depth)
            {
                best = Some((lineage.len(), profile));
            }
        }
        best.map(|(_,profile)|profile).ok_or_else(||conflict("Natively reauthorize this restore's exact repository profile or its immutable credential successor."))
    }
    pub(super) fn catalog_restore_review(&self, job_id: &str) -> Result<CatalogRestoreReview> {
        id(job_id)?;
        if !self.capabilities().restore_available {
            return Err(unavailable(
                "Catalog activation requires available verified native snapshot helpers.",
            ));
        }
        if !self.idle()? {
            return Err(error(
                ErrorCode::Busy,
                "Finish snapshot work before reviewing catalog activation.",
            ));
        }
        let key = job_id.to_string();
        let text=self.catalog.call(move|db|Ok(db.query_row("SELECT CASE WHEN length(CAST(record AS BLOB))<=33554432 THEN record END FROM backup_snapshot_jobs WHERE id=?",[key],|r|r.get::<_,String>(0))?))?;
        let job: JobRecord = decode(text)?;
        if job.view.id != job_id
            || job.view.action != "restore"
            || job.view.state != "completed_verified"
            || !job.view.restored_compared
            || !job.view.structural_checked
            || !job.view.data_read_verified
            || job.manifest.private_scope
        {
            return Err(invalid(
                "Choose a completed, fully verified whole-snapshot restore from this catalog.",
            ));
        }
        if job.manifest.platform != std::env::consts::OS
            || job.manifest.catalog_schema != crate::catalog::SCHEMA_VERSION
        {
            return Err(unavailable(
                "Catalog activation currently supports the same platform and current schema only. Retain the exported catalog for a separately tested migration.",
            ));
        }
        let summary = self.catalog.summary()?;
        if summary.id != job.manifest.catalog_id {
            return Err(unavailable(
                "Activating a different catalog identity is outside this bounded restore action.",
            ));
        }
        let p = self.catalog_restore_profile(&job.view.profile_id)?;
        self.authorized(&p)?;
        let mut scopes = vec![Scope::new(
            DirectoryChain::open(&self.data.path()?)?,
            Access::Write,
        )];
        if let Some(local) = &p.local {
            scopes.push(Scope::new(
                DirectoryChain::open(&local.path()?)?,
                Access::Write,
            ));
        }
        let _lease = Registry::shared().try_acquire(scopes)?;
        let snapshot = job
            .view
            .snapshot_id
            .clone()
            .ok_or_else(|| invalid("The completed restore lacks a concrete snapshot ID."))?;
        let record = self.record(&p, &snapshot)?;
        if digest(&record.manifest)? != digest(&job.manifest)? {
            return Err(conflict(
                "The restore job and concrete snapshot manifest differ.",
            ));
        }
        let schema = self.catalog.schema_fingerprint()?;
        if schema != crate::catalog::current_schema_fingerprint()? {
            return Err(conflict(
                "The active catalog schema differs from the compiled current schema.",
            ));
        }
        let work = self.workspace(job_id)?;
        let binding = self.repository(&p)?.ok_or_else(|| {
            conflict("Reauthorize the existing repository before catalog activation.")
        })?;
        self.read_repository_id(&p, &binding, Some(&work), &AtomicBool::new(false))?;
        let context = self.context(&p, &binding, Some(&work), &AtomicBool::new(false))?;
        let payload = record
            .subfolder
            .rsplit('/')
            .next()
            .ok_or_else(|| invalid("Invalid recorded snapshot subtree."))?;
        let subtree = self.inspect_snapshot(
            &context,
            &snapshot,
            payload,
            &record.manifest,
            &AtomicBool::new(false),
        )?;
        if subtree != record.subfolder {
            return Err(conflict("The authenticated snapshot subtree changed."));
        }
        let manifest_path = format!("{subtree}/manifest.json");
        let authenticated = context.run(
            &["dump", &snapshot, &manifest_path],
            &AtomicBool::new(false),
        )?;
        if authenticated != encode(&record.manifest)?.as_bytes() {
            return Err(conflict(
                "The repository-authenticated manifest differs from the retained catalog history.",
            ));
        }
        let output=self.catalog.call({let job=job_id.to_string();move|db|Ok(db.query_row("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=262144 THEN evidence END FROM backup_snapshot_events WHERE job_id=? AND phase='restore_published' ORDER BY sequence DESC LIMIT 1",[job],|r|r.get::<_,String>(0))?)})?;
        #[derive(Deserialize)]
        struct Published {
            path: Vec<u8>,
            identity: FileIdentity,
            volume: String,
        }
        let published: Published = decode(output)?;
        let root = Binding::capture(
            native::decode_path(&published.path)?,
            published.identity,
            published.volume,
        )?;
        let source = activation::capture_file(&root.path()?.join("Catalog.snapshot.stillharbour"))?;
        if Some(&source.digest) != record.manifest.catalog_digest.as_ref()
            || source.bytes != record.manifest.catalog_bytes
        {
            return Err(conflict(
                "The published catalog bytes differ from the freshly authenticated snapshot.",
            ));
        }
        let validated = activation::validate_catalog(&source.native_path()?, &summary.id, &schema)?;
        let roots = validated.roots;
        let mut review = ActivationReview {
            view: CatalogRestoreReview {
                id: uuid::Uuid::new_v4().to_string(),
                digest: String::new(),
                restore_job_id: job_id.into(),
                snapshot_id: snapshot,
                catalog_id: summary.id,
                schema: summary.schema,
                current_revision: store::revision(&self.catalog)?,
                restored_revision: validated.revision,
                roots: roots
                    .iter()
                    .map(|r| CatalogRestoreRoot {
                        root_id: r.root_id.clone(),
                        label: format!("Library root {}", r.root_id),
                        saved_display_path: native::decode_path(&r.path)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| "Unsupported native path".into()),
                        confirmed_display_path: None,
                        confirmed: false,
                    })
                    .collect(),
                approval_available: false,
                warning: ACTIVATION_WARNING.into(),
            },
            source,
            roots,
            mappings: BTreeMap::new(),
            schema,
            created: Instant::now(),
        };
        review.view.digest = view_digest(&review)?;
        require_fresh(self, &review)?;
        let view = review.view.clone();
        let mut state = self
            .catalog_restore
            .lock()
            .map_err(|_| unavailable("Catalog activation state is unavailable."))?;
        state
            .reviews
            .retain(|_, r| r.created.elapsed() <= REVIEW_AGE);
        if state.reviews.len() >= 8 {
            return Err(error(
                ErrorCode::Busy,
                "Too many catalog activation reviews are open.",
            ));
        }
        state.reviews.insert(view.id.clone(), review);
        Ok(view)
    }
    pub(super) fn catalog_restore_map(
        &self,
        review_id: &str,
        root_id: &str,
        selection: NativeSnapshotSelection,
    ) -> Result<CatalogRestoreReview> {
        id(review_id)?;
        id(root_id)?;
        id(&selection.id)?;
        let mut state = self
            .catalog_restore
            .lock()
            .map_err(|_| unavailable("Catalog activation state is unavailable."))?;
        let review = state
            .reviews
            .get_mut(review_id)
            .ok_or_else(|| invalid("The catalog activation review expired."))?;
        require_fresh(self, review)?;
        let expected = review
            .roots
            .iter()
            .find(|r| r.root_id == root_id)
            .ok_or_else(|| invalid("Select a root in this restored catalog."))?;
        let actual = Binding::capture(selection.path, selection.identity, selection.volume)?;
        if actual.identity.volume != expected.identity.volume
            || actual.identity.file != expected.identity.file
            || actual.volume != expected.volume
        {
            return Err(conflict(
                "The selected folder is not the original native root identity and volume. No root was relinked.",
            ));
        }
        let root_chain = DirectoryChain::open(&actual.path()?)?;
        if root_chain.overlaps(&DirectoryChain::open(&self.data.path()?)?) {
            return Err(conflict("A library root cannot overlap application data."));
        }
        review.mappings.insert(
            root_id.into(),
            CatalogRootMapping {
                root_id: root_id.into(),
                path: actual.path.clone(),
                identity: actual.identity.clone(),
                volume: actual.volume.clone(),
            },
        );
        for root in &mut review.view.roots {
            if root.root_id == root_id {
                root.confirmed = true;
                root.confirmed_display_path = Some(actual.path()?.display().to_string());
            }
        }
        review.view.approval_available = review.mappings.len() == review.roots.len();
        review.view.digest = view_digest(review)?;
        let view = review.view.clone();
        state.approvals.retain(|_, (r, _, _)| r != review_id);
        Ok(view)
    }
    pub(super) fn catalog_restore_approve(
        &self,
        review_id: &str,
        requested_digest: &str,
    ) -> Result<CatalogRestoreApproval> {
        id(review_id)?;
        let mut state = self
            .catalog_restore
            .lock()
            .map_err(|_| unavailable("Catalog activation state is unavailable."))?;
        let review = state
            .reviews
            .get(review_id)
            .ok_or_else(|| invalid("The catalog activation review expired."))?;
        require_fresh(self, review)?;
        if !review.view.approval_available
            || review.view.digest != requested_digest
            || view_digest(review)? != requested_digest
        {
            return Err(conflict(
                "Confirm every root and approve this exact current catalog activation review.",
            ));
        }
        for mapping in review.mappings.values() {
            Binding::capture(
                native::decode_path(&mapping.path)?,
                mapping.identity.clone(),
                mapping.volume.clone(),
            )?;
        }
        let approval = CatalogRestoreApproval {
            id: uuid::Uuid::new_v4().to_string(),
            review_id: review_id.into(),
            digest: requested_digest.into(),
        };
        state.approvals.retain(|_, (review, _, created)| {
            review != review_id && created.elapsed() <= REVIEW_AGE
        });
        state.approvals.insert(
            approval.id.clone(),
            (review_id.into(), requested_digest.into(), Instant::now()),
        );
        Ok(approval)
    }
}

impl Library {
    /// Validate without consuming approval or stopping any work. The shell calls
    /// this before acquiring its irreversible global activation barrier.
    pub fn validate_catalog_restore_prepare(&self, approval_id: &str) -> Result<()> {
        id(approval_id)?;
        if !self.snapshots.capabilities().restore_available {
            return Err(unavailable(
                "Native catalog activation is unavailable in this session.",
            ));
        }
        let review = {
            let state = self
                .snapshots
                .catalog_restore
                .lock()
                .map_err(|_| unavailable("Catalog activation state is unavailable."))?;
            let (review_id, approved, created) =
                state.approvals.get(approval_id).ok_or_else(|| {
                    invalid("The catalog activation approval expired or was already used.")
                })?;
            let review = state
                .reviews
                .get(review_id)
                .ok_or_else(|| invalid("The catalog activation review expired."))?;
            if created.elapsed() > REVIEW_AGE
                || approved != &view_digest(review)?
                || !review.view.approval_available
            {
                return Err(conflict("The catalog activation approval is stale."));
            }
            review.clone()
        };
        require_fresh(&self.snapshots, &review)?;
        for mapping in review.mappings.values() {
            Binding::capture(
                native::decode_path(&mapping.path)?,
                mapping.identity.clone(),
                mapping.volume.clone(),
            )?;
        }
        Ok(())
    }
    /// Native-only: caller first holds the global RPC admission guard, drains
    /// jobs and freezes the active catalog actor. Failure requires reopening.
    pub fn prepare_catalog_restore(
        &self,
        approval_id: &str,
        appdata: &Path,
    ) -> Result<PreparedCatalogRestore> {
        self.snapshots
            .prepare_catalog_restore(self, approval_id, appdata)
    }
}
impl SnapshotWorkflow {
    fn prepare_catalog_restore(
        &self,
        library: &Library,
        approval_id: &str,
        appdata: &Path,
    ) -> Result<PreparedCatalogRestore> {
        id(approval_id)?;
        if !self
            .catalog
            .call(|db| Ok(db.query_row("PRAGMA query_only", [], |r| r.get::<_, bool>(0))?))?
        {
            return Err(conflict(
                "The active catalog must be frozen by the native activation admission guard.",
            ));
        }
        if !self.idle()? || !library.scan_idle() {
            return Err(error(
                ErrorCode::Busy,
                "Catalog activation requires all owned work to stop.",
            ));
        }
        self.data.open()?;
        let app_chain = DirectoryChain::open(appdata)?;
        if app_chain.identity().volume != self.data.identity.volume
            || app_chain.identity().file != self.data.identity.file
        {
            return Err(conflict("The native application-data binding changed."));
        }
        let review = {
            let mut state = self
                .catalog_restore
                .lock()
                .map_err(|_| unavailable("Catalog activation state is unavailable."))?;
            let (review_id, approved, created) =
                state.approvals.remove(approval_id).ok_or_else(|| {
                    invalid("The catalog activation approval expired or was already used.")
                })?;
            let review = state
                .reviews
                .remove(&review_id)
                .ok_or_else(|| invalid("The catalog activation review expired."))?;
            if created.elapsed() > REVIEW_AGE || approved != view_digest(&review)? {
                return Err(conflict("The catalog activation approval is stale."));
            }
            review
        };
        require_fresh(self, &review)?;
        for root in store::roots(&self.catalog)? {
            super::super::original_jobs::require_reconciled_root(&self.catalog, root.view.id)?;
        }
        for mapping in review.mappings.values() {
            Binding::capture(
                native::decode_path(&mapping.path)?,
                mapping.identity.clone(),
                mapping.volume.clone(),
            )?;
        }
        let parent_path = appdata.join("catalog-restores");
        let chain = DirectoryChain::open(&parent_path)?;
        if !chain
            .identities()
            .iter()
            .any(|id| id.volume == self.data.identity.volume && id.file == self.data.identity.file)
        {
            return Err(conflict(
                "The catalog activation namespace is outside application data.",
            ));
        }
        let parent = Root::existing(
            parent_path,
            chain.identity().clone(),
            native::volume_token(chain.leaf())?,
        );
        let transaction_id = uuid::Uuid::new_v4().to_string();
        let transaction = bridge::make_directory(&parent, &transaction_id)?;
        bridge::write_new(&transaction,"intent.json",encode(&serde_json::json!({"version":1,"transaction_id":transaction_id,"review_digest":review.view.digest,"current_revision":review.view.current_revision,"source":review.source}))?.as_bytes())?;
        let pre_path = transaction.path.join("pre-restore.stillharbour");
        self.catalog
            .snapshot(pre_path.clone())
            .map_err(|e| at_phase("current-catalog backup", e))?;
        activation::validate_catalog(&pre_path, &review.view.catalog_id, &review.schema)?;
        let pre_restore = activation::capture_file(&pre_path)?;
        if store::revision(&self.catalog)? != review.view.current_revision {
            return Err(conflict(
                "Current catalog edits changed during preparation; retained catalogs remain untouched.",
            ));
        }
        activation::validate_frozen(&review.source)?;
        let source_path = review.source.native_path()?;
        let source_parent = source_path
            .parent()
            .ok_or_else(|| invalid("Invalid restored catalog namespace."))?;
        let source_chain = DirectoryChain::open(source_parent)?;
        let source_root = Root::existing(
            source_parent.into(),
            source_chain.identity().clone(),
            native::volume_token(source_chain.leaf())?,
        );
        let mut source_file = native::open_asset(
            source_parent,
            &source_root.identity,
            &source_root.volume,
            Path::new(
                source_path
                    .file_name()
                    .ok_or_else(|| invalid("Missing restored catalog filename."))?,
            ),
        )?;
        let spec = FileSpec {
            metadata: None,
            relative: native::encode_path(Path::new(
                source_path
                    .file_name()
                    .ok_or_else(|| invalid("Missing restored catalog filename."))?,
            )),
            target: "candidate.stillharbour".into(),
            identity: filesystem::identity(&source_file)?,
            revision: native::revision(&source_file)?,
            bytes: review.source.bytes,
            digest: bridge::digest(&mut source_file, &AtomicBool::new(false))?,
        };
        // Keep the journal scope beside the publication root. Its held ancestor
        // handles must not pin the directory receiving the final native rename.
        let journals = bridge::make_directory(&parent, &format!("journals-{transaction_id}"))?;
        bridge::copy(
            &source_root,
            &transaction,
            &[spec],
            &[],
            &journals,
            &AtomicBool::new(false),
            &|| {
                activation::validate_frozen(&review.source)?;
                Ok(())
            },
        )
        .map_err(|e| at_phase("candidate publication", e))?;
        let candidate_path = transaction.path.join("candidate.stillharbour");
        activation::validate_catalog(&candidate_path, &review.view.catalog_id, &review.schema)?;
        // All SQL is compiled application SQL, after exact owned-schema equality.
        {
            let mut db = Connection::open_with_flags(
                &candidate_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            db.execute_batch(
                "PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;",
            )?;
            let tx = db.transaction()?;
            for root in review.mappings.values() {
                tx.execute("UPDATE roots SET native_path=?,display_path=?,availability='online' WHERE id=?",params![root.path,native::decode_path(&root.path)?.display().to_string(),root.root_id])?;
            }
            tx.execute(
                "UPDATE library_state SET revision=revision+1 WHERE singleton=1",
                [],
            )?;
            tx.commit()?;
            db.close().map_err(|(_, e)| AppError::from(e))?;
        }
        let candidate_validated =
            activation::validate_catalog(&candidate_path, &review.view.catalog_id, &review.schema)?;
        bridge::sync_owned_file(&transaction, "candidate.stillharbour")
            .map_err(|e| at_phase("candidate flush", e))?;
        let candidate = activation::capture_file(&candidate_path)?;
        let current = activation::capture_current(self.catalog.native_path())?;
        let prepared = PreparedCatalogRestore {
            version: 1,
            transaction_id,
            catalog_id: review.view.catalog_id,
            source_restore_job_id: review.view.restore_job_id,
            source_snapshot_id: review.view.snapshot_id,
            source_catalog_revision: review.view.restored_revision,
            candidate_catalog_revision: candidate_validated.revision,
            current_catalog_revision: review.view.current_revision,
            schema: crate::catalog::SCHEMA_VERSION,
            current,
            pre_restore,
            candidate,
            roots: review.mappings.into_values().collect(),
        };
        bridge::write_new(&transaction, "prepared.json", encode(&prepared)?.as_bytes())?;
        #[cfg(unix)]
        filesystem::sync_directory(&transaction.path)?;
        Ok(prepared)
    }
}

#[cfg(test)]
mod tests {
    use super::super::application_tests::{Fixture, wait};
    use super::*;
    struct Run {
        fixture: Fixture,
        catalog: String,
        credential: String,
        replacement: Option<String>,
    }
    impl Drop for Run {
        fn drop(&mut self) {
            self.fixture.library.snapshots.cancel_all();
            if std::thread::panicking() || !self.fixture.library.snapshots.idle().unwrap_or(false) {
                self.fixture._run.disable_cleanup(true);
                return;
            }
            if CredentialStore::delete_owned(
                &self.catalog,
                CredentialPurpose::Repository,
                &self.credential,
            )
            .is_err()
            {
                self.fixture._run.disable_cleanup(true);
            }
            if let Some(reference) = &self.replacement
                && CredentialStore::delete_owned(
                    &self.catalog,
                    CredentialPurpose::Repository,
                    reference,
                )
                .is_err()
            {
                self.fixture._run.disable_cleanup(true);
            }
        }
    }
    fn start(f: &Fixture, review: SnapshotReview, action: &str) -> SnapshotJob {
        let s = &f.library.snapshots;
        let approval = s.approve(&review.id, &review.digest, action).unwrap();
        let frozen = s.take_approval(&review.id, &approval.id, action).unwrap();
        let job = s.start(f.library.clone(), frozen, action).unwrap();
        let done = wait(&f.library, &job.id);
        assert_eq!(done.state, "completed_verified", "{}", done.message);
        done
    }
    #[test]
    #[ignore = "Actual native generated catalog activation, OS vault and pinned restic"]
    fn native_catalog_restore_prepares_verified_candidate_and_retains_newer_local_edits() {
        assert!(std::env::var_os("STILLHARBOUR_M4_GENERATED_REPOSITORY_DRIVE").is_none());
        let f = Fixture::new();
        let helpers = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/backup-helpers")
            .join(if cfg!(windows) {
                "windows-x86_64"
            } else {
                "macos-aarch64"
            })
            .canonicalize()
            .unwrap();
        f.library.configure_snapshot_helpers(helpers).unwrap();
        let catalog = f.library.catalog.summary().unwrap().id;
        let password = uuid::Uuid::new_v4().to_string();
        let credential =
            CredentialStore::store(&catalog, CredentialPurpose::Repository, &password).unwrap();
        let mut run = Run {
            fixture: f,
            catalog,
            credential,
            replacement: None,
        };
        let f = &run.fixture;
        let selected = Fixture::selection(&f.backup);
        let profile = f
            .library
            .snapshots
            .profile_create(
                &f.library,
                "Generated catalog activation".into(),
                vec![f.root.clone()],
                run.credential.clone(),
                None,
                SnapshotDestinationInput::Local {
                    selection_id: selected.id.clone(),
                },
                Some(selected),
            )
            .unwrap();
        let backup = start(
            f,
            f.library
                .snapshots
                .review(f.library.snapshots.profile(&profile.id).unwrap())
                .unwrap(),
            "backup",
        );
        let restored = start(
            f,
            f.library
                .snapshots
                .restore_review(
                    &f.library,
                    f.library.snapshots.profile(&profile.id).unwrap(),
                    backup.snapshot_id.as_ref().unwrap(),
                    Fixture::selection(&f.output),
                )
                .unwrap(),
            "restore",
        );
        let s = &f.library.snapshots;
        let replacement =
            CredentialStore::store(&run.catalog, CredentialPurpose::Repository, &password).unwrap();
        run.replacement = Some(replacement.clone());
        CredentialStore::delete_owned(&run.catalog, CredentialPurpose::Repository, &run.credential)
            .unwrap();
        let rebound = f
            .library
            .snapshot_profile_rebind_credential(
                &profile.id,
                CredentialPurpose::Repository,
                &replacement,
                Some(Fixture::selection(&f.backup)),
                None,
            )
            .unwrap();
        assert_ne!(rebound.id, profile.id);
        assert_eq!(
            s.catalog_restore_profile(&profile.id).unwrap().view.id,
            rebound.id
        );
        let first = s.catalog_restore_review(&restored.id).unwrap();
        assert!(!first.approval_available);
        assert!(s.catalog_restore_approve(&first.id, &first.digest).is_err());
        assert!(
            s.catalog_restore_map(&first.id, &f.root, Fixture::selection(&f.output))
                .is_err()
        );
        f.library
            .catalog
            .call(|db| {
                db.execute("UPDATE roots SET label='newer generated local edit'", [])?;
                db.execute(
                    "UPDATE library_state SET revision=revision+1 WHERE singleton=1",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(
            s.catalog_restore_map(&first.id, &f.root, Fixture::selection(&f.source))
                .is_err()
        );
        let review = s.catalog_restore_review(&restored.id).unwrap();
        let mapped = s
            .catalog_restore_map(&review.id, &f.root, Fixture::selection(&f.source))
            .unwrap();
        assert!(mapped.approval_available);
        assert_ne!(mapped.digest, review.digest);
        assert!(
            s.catalog_restore_approve(&mapped.id, &review.digest)
                .is_err()
        );
        let replaced_approval = s
            .catalog_restore_approve(&mapped.id, &mapped.digest)
            .unwrap();
        let approval = s
            .catalog_restore_approve(&mapped.id, &mapped.digest)
            .unwrap();
        assert_ne!(approval.id, replaced_approval.id);
        assert_eq!(s.catalog_restore.lock().unwrap().approvals.len(), 1);
        assert!(
            f.library
                .validate_catalog_restore_prepare(&replaced_approval.id)
                .is_err()
        );
        assert!(
            f.library
                .validate_catalog_restore_prepare(&uuid::Uuid::new_v4().to_string())
                .is_err()
        );
        f.library
            .validate_catalog_restore_prepare(&approval.id)
            .unwrap();
        let appdata = f.library.options.data_directory.clone();
        assert!(
            f.library
                .prepare_catalog_restore(&approval.id, &appdata)
                .is_err()
        );
        // The native activation store owns this fixed parent in application
        // integration. Here only the asserted generated app scope is created.
        bridge::make_directory(&s.data.root().unwrap(), "catalog-restores").unwrap();
        // Directory link counts may change as owned siblings appear (macOS).
        // The captured data scope remains the same native volume/file object.
        bridge::make_directory(&s.data.root().unwrap(), "generated-unrelated-sibling").unwrap();
        let current_data = DirectoryChain::open(&appdata).unwrap();
        assert_eq!(
            (current_data.identity().volume, current_data.identity().file),
            (s.data.identity.volume, s.data.identity.file)
        );
        drop(current_data);
        let _quiesced = f.library.quiesce_for_catalog_restore().unwrap();
        let prepared = f
            .library
            .prepare_catalog_restore(&approval.id, &appdata)
            .unwrap();
        assert_eq!(prepared.catalog_id, run.catalog);
        assert_eq!(prepared.source_restore_job_id, restored.id);
        assert_eq!(prepared.current_catalog_revision, mapped.current_revision);
        assert_ne!(
            prepared.current_catalog_revision,
            prepared.source_catalog_revision
        );
        assert_ne!(prepared.pre_restore.path, prepared.candidate.path);
        activation::validate_frozen(&prepared.pre_restore).unwrap();
        activation::validate_frozen(&prepared.candidate).unwrap();
        let old = Connection::open_with_flags(
            prepared.candidate.native_path().unwrap(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let retained = Connection::open_with_flags(
            prepared.pre_restore.native_path().unwrap(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let current_label: String = retained
            .query_row("SELECT label FROM roots WHERE id=?", [&f.root], |r| {
                r.get(0)
            })
            .unwrap();
        let old_label: String = old
            .query_row("SELECT label FROM roots WHERE id=?", [&f.root], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(current_label, "newer generated local edit");
        assert_eq!(old_label, "Generated source");
        assert!(
            f.library
                .catalog
                .call(|db| Ok(db.execute("UPDATE roots SET label='must remain frozen'", [])?))
                .is_err()
        );
        assert!(
            f.library
                .prepare_catalog_restore(&approval.id, &appdata)
                .is_err()
        );
        assert_eq!(
            std::fs::read(f.source.join("original.txt")).unwrap(),
            b"generated preserved original"
        );
        assert_eq!(
            std::fs::read(f.source.join("hidden-source/companion.xmp")).unwrap(),
            b"generated sidecar outside gallery filter"
        );
    }
}
