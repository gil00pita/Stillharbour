//! Exact-version retained replacement. Native scopes originate in a complete
//! comparison; independent evidence is resolved by the snapshot provider.
//! Catalog records cannot manufacture provider proof or session approval.
use super::*;
use crate::backup::native_metadata::MetadataManifest;
use std::time::{SystemTime, UNIX_EPOCH};
mod evidence;
pub(crate) use evidence::RetainedRecoveryScope;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Version {
    id: String,
    entry_id: String,
    incoming: Observed,
    old: Observed,
    incoming_digest: String,
    old_digest: String,
    modified: String,
    created: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata: Option<MetadataManifest>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RetainedBinding {
    pub(super) operation_id: String,
    pub(super) evidence_id: String,
    origin: Option<String>,
    versions: Vec<Version>,
}
#[derive(Default)]
struct Facts {
    intent: bool,
    directory: Option<Object>,
    old_intents: BTreeMap<usize, ()>,
    old_done: BTreeMap<usize, (Object, String)>,
    new_intents: BTreeMap<usize, Object>,
    new_done: BTreeMap<usize, Object>,
    complete: bool,
    metadata_intents: std::collections::BTreeSet<usize>,
    metadata_done: BTreeMap<usize, (Object, String)>,
}
fn folder(id: &str) -> String {
    format!(".stillharbour-retained-{id}")
}
fn old_leaf(index: usize) -> String {
    format!("old-{index}.retained")
}
fn new_leaf(index: usize) -> String {
    format!("new-{index}.verified")
}
fn copy_leaf(retained: &RetainedBinding, index: usize) -> Result<String> {
    if retained.origin.is_none() {
        return Ok(new_leaf(index));
    }
    let path = source::decode_path(
        &retained
            .versions
            .get(index)
            .ok_or_else(|| failed("Missing retained restore member."))?
            .old
            .path,
    )?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid("Unsupported retained output filename."))?;
    let leaf = format!("restored-{index}-{name}");
    if !crate::organization::portable_filename(&leaf) {
        return Err(invalid(
            "Retained output needs another native filename adapter.",
        ));
    }
    Ok(leaf)
}
fn stamp(value: SystemTime) -> Result<String> {
    Ok(value
        .duration_since(UNIX_EPOCH)
        .map_err(|_| invalid("Pre-epoch native metadata requires another fidelity adapter."))?
        .as_nanos()
        .to_string())
}
fn descriptive(file: &File) -> Result<(String, Option<String>)> {
    let metadata = file.metadata()?;
    Ok((
        stamp(metadata.modified()?)?,
        metadata.created().ok().map(stamp).transpose()?,
    ))
}
fn checked(file: &File, observed: &Observed) -> Result<()> {
    if filesystem::identity(file)? != observed.identity
        || source::revision(file)? != observed.revision
        || file.metadata()?.len() != observed.bytes
    {
        return Err(failed(
            "An exact reviewed retained version changed native identity, revision or size.",
        ));
    }
    Ok(())
}
fn metadata(file: &File) -> Result<Option<MetadataManifest>> {
    match MetadataManifest::inspect(file) {
        Ok(manifest) => Ok(Some(manifest)),
        Err(error) if error.code == ErrorCode::Unsupported => Ok(None),
        Err(error) => Err(error),
    }
}
fn hash(file: &File, cancel: &AtomicBool) -> Result<String> {
    let identity = filesystem::identity(file)?;
    let revision = source::revision(file)?;
    let bytes = file.metadata()?.len();
    let mut input = file.try_clone()?;
    input.seek(SeekFrom::Start(0))?;
    let io = PacedIo::shared();
    let mut buffer = [0; CHUNK_BYTES];
    let mut digest = blake3::Hasher::new();
    let mut read = 0u64;
    loop {
        let IoStep::Completed(n) = io.read(&mut input, &mut buffer, cancel)? else {
            return Err(failed(
                "Retained version verification canceled; every object remains retained.",
            ));
        };
        if n == 0 {
            break;
        }
        read = read
            .checked_add(n as u64)
            .ok_or_else(|| failed("Retained version length overflow."))?;
        digest.update(&buffer[..n]);
    }
    if read != bytes
        || filesystem::identity(file)? != identity
        || source::revision(file)? != revision
    {
        return Err(failed(
            "A retained version changed during complete-byte verification.",
        ));
    }
    Ok(digest.finalize().to_hex().to_string())
}
fn open_parent(root: &File, scope: &Scope, observed: &Observed) -> Result<(File, String)> {
    let path = source::decode_path(&scope.namespace)?.join(source::decode_path(&observed.path)?);
    let text = relative(&path)?;
    let (parent, leaf) = text
        .rsplit_once('/')
        .ok_or_else(|| failed("Invalid retained destination parent."))?;
    Ok((
        locations::Destination::capture(root, parent)?.open(root)?,
        leaf.into(),
    ))
}
fn recovery_files(binding: &Binding) -> Result<Vec<RecoveryFile>> {
    let retained = binding
        .retained
        .as_ref()
        .ok_or_else(|| failed("Missing retained binding."))?;
    retained
        .versions
        .iter()
        .map(|v| {
            Ok(RecoveryFile {
                catalog_id: binding.scope.catalog_id.clone(),
                root_id: binding.scope.root_id.clone(),
                native_root: binding.scope.destination.clone(),
                root_identity: binding.scope.destination_identity.clone(),
                root_volume: binding.scope.destination_volume.clone(),
                relative_path: source::encode_path(
                    &source::decode_path(&binding.scope.namespace)?
                        .join(source::decode_path(&v.old.path)?),
                ),
                identity: v.old.identity.clone(),
                revision: v.old.revision.clone(),
                bytes: v.old.bytes,
                full_digest: v.old_digest.clone(),
                // Actual versioned native metadata, when the strict no-extra-
                // streams policy supports this file. Absence remains a gate;
                // byte-only restore success cannot manufacture this digest.
                essential_metadata_digest: v
                    .metadata
                    .as_ref()
                    .map(MetadataManifest::digest)
                    .transpose()?,
            })
        })
        .collect()
}

impl SyncWorkflow {
    pub(crate) fn recovery_snapshot_scope(
        &self,
        library: &Library,
        run: String,
        entry: String,
    ) -> Result<Vec<RecoveryFile>> {
        let (scope, versions) = library.backup_replacement_scope(run, entry)?;
        library.validate_backup_sync_scope(&scope)?;
        let root = output_chain(&source::decode_path(&scope.destination)?)?;
        let mut files = Vec::with_capacity(versions.len());
        for (_, _, old) in versions {
            let (parent, leaf) = open_parent(root.leaf(), &scope, &old)?;
            let file = native::reopen_stage(&parent, &leaf, &old.identity)?;
            checked(&file, &old)?;
            files.push(RecoveryFile {
                catalog_id: scope.catalog_id.clone(),
                root_id: scope.root_id.clone(),
                native_root: scope.destination.clone(),
                root_identity: scope.destination_identity.clone(),
                root_volume: scope.destination_volume.clone(),
                relative_path: source::encode_path(
                    &source::decode_path(&scope.namespace)?.join(source::decode_path(&old.path)?),
                ),
                identity: old.identity,
                revision: old.revision,
                bytes: old.bytes,
                full_digest: hash(&file, &AtomicBool::new(false))?,
                essential_metadata_digest: metadata(&file)?
                    .as_ref()
                    .map(MetadataManifest::digest)
                    .transpose()?,
            });
        }
        library.validate_backup_sync_scope(&scope)?;
        Ok(files)
    }
    fn retained_proof(&self, library: &Library, binding: &Binding) -> Result<RecoveryProof> {
        let retained = binding
            .retained
            .as_ref()
            .ok_or_else(|| failed("Missing retained binding."))?;
        let files = recovery_files(binding)?;
        if files
            .iter()
            .any(|file| file.essential_metadata_digest.is_none())
        {
            return Err(AppError::new(
                ErrorCode::MutationUnavailable,
                "A selected OLD destination or companion has unsupported essential metadata/streams. Preserve every version; byte-only snapshot evidence cannot authorize replacement.",
            ));
        }
        #[cfg(test)]
        let override_proof = self
            .recovery_proof
            .lock()
            .map_err(|_| failed("Test evidence unavailable."))?
            .clone();
        #[cfg(not(test))]
        let override_proof: Option<RecoveryProof> = None;
        if override_proof.is_none()
            && retained
                .versions
                .iter()
                .map(|v| {
                    v.metadata
                        .as_ref()
                        .map(MetadataManifest::certified_replacement)
                        .transpose()
                })
                .collect::<Result<Vec<_>>>()?
                .iter()
                .any(|ready| *ready != Some(true))
        {
            return Err(AppError::new(
                ErrorCode::MutationUnavailable,
                "Ordinary essential metadata was recorded, but relevant native security fidelity is not certified under current read rights. Keep every version; strict replacement remains unavailable.",
            ));
        }
        let proof = match override_proof {
            Some(proof) => proof,
            None => {
                let facts = self.retained_facts(&retained.operation_id, binding)?;
                if facts.old_intents.is_empty() {
                    library.sync_recovery_proof(&files, &retained.evidence_id)?
                } else {
                    let scope = RetainedRecoveryScope::new(self, library, binding)?;
                    library.retained_recovery_proof(&scope, &retained.evidence_id)?
                }
            }
        };
        if proof.evidence_id != retained.evidence_id
            || proof.scope_digest.len() != 64
            || !proof.independent_failure_domain
            || !proof.essential_metadata_verified
        {
            return Err(AppError::new(
                ErrorCode::MutationUnavailable,
                "Replacement requires restored exact OLD destination versions, essential native metadata and an independently proven failure domain. Ordinary byte backup success is insufficient.",
            ));
        }
        Ok(proof)
    }
    pub(super) fn validate_binding(&self, library: &Library, binding: &Binding) -> Result<()> {
        let Some(retained) = &binding.retained else {
            return library.validate_backup_sync_scope(&binding.scope);
        };
        if let Some(origin) = &retained.origin {
            let original = self.binding(origin)?;
            if original.retained.as_ref().is_none_or(|r| {
                r.origin.is_some() || encode(&r.versions).ok() != encode(&retained.versions).ok()
            }) {
                return Err(failed(
                    "Restore scope differs from its immutable retained version group.",
                ));
            }
            self.validate_retained_history(library, &original)?;
            if self.retained_facts(origin, &original)?.old_done.len() != retained.versions.len() {
                return Err(failed(
                    "Identify and verify every retained old companion before restoring the group.",
                ));
            }
            return Ok(());
        }
        let paths = retained
            .versions
            .iter()
            .map(|v| v.old.path.clone())
            .collect::<Vec<_>>();
        library.validate_backup_sync_scope_except(&binding.scope, &paths)?;
        let root = output_chain(&source::decode_path(&binding.scope.destination)?)?;
        let facts = self.retained_facts(&retained.operation_id, binding)?;
        let history = match &facts.directory {
            Some(object) => {
                let f = native::read_directory(root.leaf(), &folder(&retained.operation_id))?;
                if !object.matches(&f)? {
                    return Err(failed("Retained history directory changed identity."));
                }
                Some(f)
            }
            None => None,
        };
        let plan = self.plan(&retained.operation_id)?;
        let copy_objects = if let Some(plan) = &plan {
            let journal = storage::SyncJournal::new_retained(
                self.catalog.clone(),
                retained.operation_id.clone(),
                plan.clone(),
            )?;
            journal
                .history()?
                .0
                .into_iter()
                .map(|c| c.object)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for (index, version) in retained.versions.iter().enumerate() {
            let (parent, leaf) = open_parent(root.leaf(), &binding.scope, &version.old)?;
            let old = if facts.old_intents.contains_key(&index) {
                let history = history.as_ref().ok_or_else(|| {
                    failed("Old-version move has no identified history directory.")
                })?;
                let at_history = native::lookup(history, &old_leaf(index))?;
                match (at_history, native::lookup(&parent, &leaf)?) {
                    (Some(entry), target)
                        if entry.kind == native::EntryKind::File
                            && entry.object
                                == (Object {
                                    volume: version.old.identity.volume,
                                    file: version.old.identity.file,
                                }) =>
                    {
                        if let Some(target) = target
                            && !facts
                                .new_intents
                                .get(&index)
                                .is_some_and(|o| o == &target.object)
                        {
                            return Err(failed(
                                "A late target occupant is retained; replacement cannot overwrite it.",
                            ));
                        }
                        native::reopen_stage(history, &old_leaf(index), &version.old.identity)?
                    }
                    (None, Some(entry))
                        if !facts.old_done.contains_key(&index)
                            && entry.object
                                == (Object {
                                    volume: version.old.identity.volume,
                                    file: version.old.identity.file,
                                }) =>
                    {
                        native::reopen_stage(&parent, &leaf, &version.old.identity)?
                    }
                    _ => {
                        return Err(failed(
                            "Old destination ownership is uncertain; retain all locations and inspect recovery.",
                        ));
                    }
                }
            } else {
                native::reopen_stage(&parent, &leaf, &version.old.identity)?
            };
            let in_history = history
                .as_ref()
                .map(|h| native::lookup(h, &old_leaf(index)))
                .transpose()?
                .flatten()
                .is_some();
            if in_history {
                if filesystem::identity(&old)? != version.old.identity
                    || old.metadata()?.len() != version.old.bytes
                    || facts.old_done.get(&index).is_some_and(|(_, revision)| {
                        source::revision(&old).as_ref().ok() != Some(revision)
                    })
                    || hash(&old, &AtomicBool::new(false))? != version.old_digest
                {
                    return Err(failed(
                        "Identified retained old version changed after its native move.",
                    ));
                }
            } else {
                checked(&old, &version.old)?;
            }
            if descriptive(&old)? != (version.modified.clone(), version.created.clone()) {
                return Err(failed("Retained original descriptive metadata changed."));
            }
            if let Some(metadata) = &version.metadata {
                metadata.compare(&old)?;
            }
            if let Some(expected) = facts.new_intents.get(&index) {
                if copy_objects.get(index).and_then(|o| o.as_ref()) != Some(expected) {
                    return Err(failed(
                        "Replacement publication is not bound to its verified incoming stream.",
                    ));
                }
                let at_target = native::lookup(&parent, &leaf)?;
                if facts.new_done.contains_key(&index)
                    && at_target.as_ref().is_none_or(|e| &e.object != expected)
                {
                    return Err(failed(
                        "An identified replacement output disappeared or changed.",
                    ));
                }
            }
        }
        Ok(())
    }
    fn validate_retained_history(&self, library: &Library, binding: &Binding) -> Result<()> {
        library.validate_backup_sync_roots(&binding.scope)?;
        let retained = binding
            .retained
            .as_ref()
            .filter(|r| r.origin.is_none())
            .ok_or_else(|| failed("Expected an original retained history binding."))?;
        let facts = self.retained_facts(&retained.operation_id, binding)?;
        let Some(expected) = facts.directory else {
            return Ok(());
        };
        let root = output_chain(&source::decode_path(&binding.scope.destination)?)?;
        let history = native::read_directory(root.leaf(), &folder(&retained.operation_id))?;
        if !expected.matches(&history)? {
            return Err(failed(
                "Retained history directory no longer has its exact journaled identity.",
            ));
        }
        for (index, (object, revision)) in facts.old_done {
            let version = retained
                .versions
                .get(index)
                .ok_or_else(|| failed("Invalid retained history index."))?;
            let file = native::read(&history, &old_leaf(index))?;
            if !object.matches(&file)?
                || filesystem::identity(&file)? != version.old.identity
                || source::revision(&file)? != revision
                || descriptive(&file)? != (version.modified.clone(), version.created.clone())
                || hash(&file, &AtomicBool::new(false))? != version.old_digest
            {
                return Err(failed(
                    "Retained history version changed; preserve every object and inspect it.",
                ));
            }
            if let Some(metadata) = &version.metadata {
                metadata.compare(&file)?;
            }
        }
        Ok(())
    }
    pub(super) fn review_replacement(
        &self,
        library: &Library,
        run: String,
        entry: String,
        evidence: String,
    ) -> Result<SyncReview> {
        if evidence.len() != 64 || !evidence.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid(
                "Select a concrete backend-recorded 64-digit snapshot identifier.",
            ));
        }
        let (scope, versions) = library.backup_replacement_scope(run, entry)?;
        library.validate_backup_sync_scope(&scope)?;
        let src = source::open_root(
            &source::decode_path(&scope.source)?,
            Some(&scope.source_identity),
        )?;
        let dst = output_chain(&source::decode_path(&scope.destination)?)?;
        let cancel = AtomicBool::new(false);
        let mut checked_versions = Vec::new();
        for (entry_id, incoming, old) in versions {
            let a = locations::Source::capture(&src, &source::decode_path(&incoming.path)?)?
                .open(&src)?;
            let (parent, leaf) = open_parent(dst.leaf(), &scope, &old)?;
            let b = native::reopen_stage(&parent, &leaf, &old.identity)?;
            checked(&a, &incoming)?;
            checked(&b, &old)?;
            let (modified, created) = descriptive(&b)?;
            let metadata = metadata(&b)?;
            checked_versions.push(Version {
                id: uuid::Uuid::new_v4().to_string(),
                entry_id,
                incoming,
                old,
                incoming_digest: hash(&a, &cancel)?,
                old_digest: hash(&b, &cancel)?,
                modified,
                created,
                metadata,
            });
        }
        let operation_id = uuid::Uuid::new_v4().to_string();
        let bytes = checked_versions.iter().try_fold(0u64, |sum, v| {
            sum.checked_add(v.incoming.bytes)
                .ok_or_else(|| invalid("Retained scope byte count exceeds its bound."))
        })?;
        let required = u128::from(bytes)
            .saturating_add(RESERVE)
            .saturating_add(checked_versions.len() as u128 * 16384);
        let available = fs2::statvfs(&source::decode_path(&scope.destination)?)?.available_space();
        let view=SyncReview{id:uuid::Uuid::new_v4().to_string(),job_id:None,run_id:scope.run_id.clone(),action:SyncAction::ReplaceRetaining,digest:String::new(),source_root_id:scope.root_id.clone(),source_display_path:source::decode_path(&scope.source)?.to_string_lossy().into_owned(),dataset_display_path:source::decode_path(&scope.destination)?.join(source::decode_path(&scope.namespace)?).to_string_lossy().into_owned(),copy_files:checked_versions.len() as u32,copy_bytes:bytes.to_string(),retained_destination_files:checked_versions.len() as u32,uncopied_source_revisions:(scope.sources.iter().filter(|f|f.copy).count() as u32).saturating_add(scope.uncopied).saturating_sub(checked_versions.iter().filter(|v|v.old_digest!=v.incoming_digest).count() as u32),available_bytes:Some(available.to_string()),required_bytes:required.to_string(),metadata_warning:"Replacement retains the exact native old objects and every companion in unique history. Independent exact-version restore and essential metadata evidence are mandatory. Restoration later creates new protected outputs.".into(),execution_available:false,reason:String::new(),approval_lifetime_seconds:600,items:checked_versions.iter().map(|v|SyncPlanItem{entry_id:v.entry_id.clone(),display_path:source::decode_path(&v.old.path).map(|p|p.to_string_lossy().into_owned()).unwrap_or_else(|_|"Invalid path".into()),action:SyncItemAction::ReplaceRetaining,source_bytes:Some(v.incoming.bytes.to_string())}).collect()};
        let parents = preparation::capture_parents(
            dst.leaf(),
            &relative(&source::decode_path(&scope.namespace)?)?,
        )?;
        let mut binding = Binding {
            scope,
            view: view.clone(),
            parents,
            retained: Some(RetainedBinding {
                operation_id,
                evidence_id: evidence,
                origin: None,
                versions: checked_versions,
            }),
        };
        let mut view = view;
        if let Some(metadata) = binding
            .retained
            .as_ref()
            .and_then(|r| r.versions.first())
            .and_then(|v| v.metadata.as_ref())
        {
            view.metadata_warning = format!("{} {}", view.metadata_warning, metadata.description());
        }
        match self.retained_proof(library, &binding) {
            Ok(_) => {
                view.execution_available =
                    Self::capabilities()?.execution_available && u128::from(available) >= required;
                view.reason="The provider verified the exact old destination group. Approval stages/readbacks incoming bytes, retains every original version, then publishes using native no-replace.".into();
            }
            Err(e) => view.reason = e.message,
        }
        view.digest = blake3::hash(encode(&binding)?.as_bytes())
            .to_hex()
            .to_string();
        binding.view = view.clone();
        self.retain_review(Review {
            binding,
            view,
            stage: Stage::Retained,
            created: Instant::now(),
        })
    }
    pub(super) fn validate_retained_review(
        &self,
        library: &Library,
        review: &Review,
    ) -> Result<()> {
        self.validate_binding(library, &review.binding)?;
        let retained = review
            .binding
            .retained
            .as_ref()
            .ok_or_else(|| failed("Missing retained review scope."))?;
        if retained.origin.is_none() {
            self.retained_proof(library, &review.binding)?;
        }
        if review.view.job_id.is_some()
            && self.retained_stream_digest(library, &review.binding)? != review.view.digest
        {
            return Err(failed(
                "Retained recovery observations changed after review. Inspect and approve again.",
            ));
        }
        Ok(())
    }
    fn retained_stream_digest(&self, library: &Library, binding: &Binding) -> Result<String> {
        self.validate_binding(library, binding)?;
        let retained = binding
            .retained
            .as_ref()
            .ok_or_else(|| failed("Missing retained recovery scope."))?;
        let events = self.events(&retained.operation_id)?;
        let Some(plan) = self.plan(&retained.operation_id)? else {
            return Ok(blake3::hash(encode(&(binding, events))?.as_bytes())
                .to_hex()
                .to_string());
        };
        let src = source::open_root(
            &source::decode_path(&binding.scope.source)?,
            Some(&binding.scope.source_identity),
        )?;
        let dst = output_chain(&source::decode_path(&binding.scope.destination)?)?;
        let copy_source = if retained.origin.is_some() {
            dst.leaf()
        } else {
            &src
        };
        self.validate_retained_plan(binding, &plan, copy_source, dst.leaf())?;
        let journal = storage::SyncJournal::new_retained(
            self.catalog.clone(),
            retained.operation_id.clone(),
            plan.clone(),
        )?;
        let (cursors, terminal, digest) = journal.history()?;
        let io = PacedIo::shared();
        let cancel = AtomicBool::new(false);
        let validate = || self.validate_binding(library, binding);
        if !terminal {
            let inputs = CopyInputs {
                source_root: copy_source,
                destination: dst.leaf(),
                io: &io,
                cancel: &cancel,
                validate_scope: &validate,
            };
            let inspection = resume::Review::capture(&plan, &cursors, digest, &inputs)?;
            return Ok(
                blake3::hash(encode(&(binding, events, inspection.digest()))?.as_bytes())
                    .to_hex()
                    .to_string(),
            );
        }
        let facts = self.retained_facts(&retained.operation_id, binding)?;
        let directory = native::read_directory(dst.leaf(), &folder(&retained.operation_id))?;
        let mut observations = Vec::new();
        for (index, version) in retained.versions.iter().enumerate() {
            let expected = cursors
                .get(index)
                .and_then(|c| c.object.as_ref())
                .ok_or_else(|| failed("Missing incoming native recovery identity."))?;
            let (parent, leaf) = if facts.new_intents.contains_key(&index) {
                let (parent, leaf) = open_parent(dst.leaf(), &binding.scope, &version.old)?;
                if native::lookup(&parent, &leaf)?.is_some() {
                    (parent, leaf)
                } else {
                    (directory.try_clone()?, copy_leaf(retained, index)?)
                }
            } else {
                (directory.try_clone()?, copy_leaf(retained, index)?)
            };
            let mut current = native::read(&parent, &leaf)?;
            if !expected.matches(&current)? {
                return Err(failed(
                    "Retained incoming recovery object changed. Preserve every occupant.",
                ));
            }
            if let Some((object, revision)) = facts.metadata_done.get(&index) {
                if !object.matches(&current)? || source::revision(&current)? != *revision {
                    return Err(failed(
                        "An identified restored metadata completion changed. Preserve the output and inspect it.",
                    ));
                }
                version
                    .metadata
                    .as_ref()
                    .ok_or_else(|| failed("Missing restored metadata policy."))?
                    .compare(&current)?;
            }
            let mut original = plan.files[index]
                .source_location
                .as_ref()
                .ok_or_else(|| failed("Missing retained source locator."))?
                .open(copy_source)?;
            let IoStep::Completed(witness) = compare(
                &io,
                &mut original,
                &mut current,
                &plan.files[index],
                &cancel,
            )?
            else {
                return Err(failed("Retained recovery byte comparison canceled."));
            };
            witness.revalidate(&parent, &leaf, &current)?;
            if &witness.digest
                != if retained.origin.is_some() {
                    &version.old_digest
                } else {
                    &version.incoming_digest
                }
            {
                return Err(failed(
                    "Retained incoming recovery bytes differ from the exact reviewed version.",
                ));
            }
            observations.push((
                index,
                expected.clone(),
                source::revision(&current)?,
                current.metadata()?.len(),
                witness.digest,
            ));
        }
        Ok(
            blake3::hash(encode(&(binding, events, observations))?.as_bytes())
                .to_hex()
                .to_string(),
        )
    }
    fn retained_facts(&self, id: &str, binding: &Binding) -> Result<Facts> {
        let retained = binding
            .retained
            .as_ref()
            .ok_or_else(|| failed("Missing retained operation scope."))?;
        // A review has no persisted job yet. Existence is not inferred from a
        // missing directory; only the catalog actor says whether a job exists.
        let key = id.to_string();
        let exists = self.catalog.call(move |db| {
            Ok(db.query_row(
                "SELECT EXISTS(SELECT 1 FROM backup_sync_jobs WHERE id=?)",
                [key],
                |r| r.get::<_, bool>(0),
            )?)
        })?;
        if !exists {
            return Ok(Facts::default());
        }
        let mut facts = Facts::default();
        for (item, phase, evidence) in self.events(id)? {
            if phase.starts_with("copy/") {
                continue;
            }
            if facts.complete || item.is_some() {
                return Err(failed(
                    "Unexpected retained event after completion or with item authority.",
                ));
            }
            match phase.as_str() {
                "retained/directory_intent"
                    if !facts.intent && evidence == serde_json::json!({"name":folder(id)}) =>
                {
                    facts.intent = true
                }
                "retained/directory_ready" if facts.intent && facts.directory.is_none() => {
                    let object: Object = serde_json::from_value(
                        evidence
                            .get("object")
                            .cloned()
                            .ok_or_else(|| failed("Missing retained directory identity."))?,
                    )
                    .map_err(|_| failed("Invalid retained directory identity."))?;
                    if evidence != serde_json::json!({"name":folder(id),"object":object}) {
                        return Err(failed("Retained directory binding changed."));
                    }
                    facts.directory = Some(object);
                }
                "retained/old_intent"
                | "retained/old_done"
                | "retained/new_intent"
                | "retained/new_done" => {
                    let index = evidence
                        .get("index")
                        .and_then(|v| v.as_u64())
                        .and_then(|v| usize::try_from(v).ok())
                        .filter(|i| *i < retained.versions.len())
                        .ok_or_else(|| failed("Invalid retained event index."))?;
                    let version = &retained.versions[index];
                    if facts.directory.is_none() || retained.origin.is_some() {
                        return Err(failed(
                            "Retained move has no owned directory or belongs to a restore-only action.",
                        ));
                    }
                    match phase.as_str() {
                        "retained/old_intent" => {
                            if evidence
                                != serde_json::json!({"index":index,"identity":version.old.identity,"digest":version.old_digest})
                                || facts.old_intents.insert(index, ()).is_some()
                            {
                                return Err(failed("Invalid repeated old-version intent."));
                            }
                        }
                        "retained/old_done" => {
                            let object = Object {
                                volume: version.old.identity.volume,
                                file: version.old.identity.file,
                            };
                            let revision = evidence
                                .get("revision")
                                .and_then(|v| v.as_str())
                                .filter(|v| v.len() == 64)
                                .ok_or_else(|| {
                                    failed(
                                        "Retained old completion has no native post-move revision.",
                                    )
                                })?
                                .to_owned();
                            if !facts.old_intents.contains_key(&index)
                                || evidence
                                    != serde_json::json!({"index":index,"object":object,"digest":version.old_digest,"revision":revision})
                                || facts.old_done.insert(index, (object, revision)).is_some()
                            {
                                return Err(failed("Invalid old-version completion."));
                            }
                        }
                        "retained/new_intent" => {
                            let object: Object = serde_json::from_value(
                                evidence
                                    .get("object")
                                    .cloned()
                                    .ok_or_else(|| failed("Missing incoming identity."))?,
                            )
                            .map_err(|_| failed("Invalid incoming identity."))?;
                            if facts.old_done.len() != retained.versions.len()
                                || evidence != serde_json::json!({"index":index,"object":object})
                                || facts.new_intents.insert(index, object).is_some()
                            {
                                return Err(failed(
                                    "Incoming publication precedes complete old-version retention.",
                                ));
                            }
                        }
                        _ => {
                            let object = facts
                                .new_intents
                                .get(&index)
                                .ok_or_else(|| failed("Incoming publication has no intent."))?
                                .clone();
                            if evidence
                                != serde_json::json!({"index":index,"object":object,"digest":version.incoming_digest})
                                || facts.new_done.insert(index, object).is_some()
                            {
                                return Err(failed("Invalid incoming publication completion."));
                            }
                        }
                    }
                }
                "retained/completed" => {
                    if facts.directory.is_none()
                        || (retained.origin.is_none()
                            && facts.new_done.len() != retained.versions.len())
                        || (retained.origin.is_some()
                            && facts.metadata_done.len()
                                != retained
                                    .versions
                                    .iter()
                                    .filter(|v| v.metadata.is_some())
                                    .count())
                        || evidence
                            != serde_json::json!({"files":retained.versions.len(),"restore":retained.origin.is_some()})
                    {
                        return Err(failed("Incomplete retained operation cannot be committed."));
                    }
                    facts.complete = true;
                }
                "retained/metadata_intent" | "retained/metadata_verified" => {
                    let index = evidence
                        .get("index")
                        .and_then(|v| v.as_u64())
                        .and_then(|v| usize::try_from(v).ok())
                        .ok_or_else(|| failed("Invalid retained restore metadata index."))?;
                    let version = retained
                        .versions
                        .get(index)
                        .and_then(|v| v.metadata.as_ref())
                        .ok_or_else(|| {
                            failed("Restore metadata is absent from the immutable version group.")
                        })?;
                    let digest = version.digest()?;
                    if retained.origin.is_none() || facts.directory.is_none() {
                        return Err(failed(
                            "Only an owned new restore output can receive metadata.",
                        ));
                    }
                    if phase == "retained/metadata_intent" {
                        if evidence != serde_json::json!({"index":index,"metadata_digest":digest})
                            || !facts.metadata_intents.insert(index)
                        {
                            return Err(failed("Invalid/repeated retained metadata intent."));
                        }
                    } else {
                        let object: Object =
                            serde_json::from_value(evidence.get("object").cloned().ok_or_else(
                                || failed("Missing restore metadata object identity."),
                            )?)
                            .map_err(|_| failed("Invalid restore metadata object."))?;
                        let revision = evidence
                            .get("revision")
                            .and_then(|v| v.as_str())
                            .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
                            .ok_or_else(|| failed("Missing native metadata completion revision."))?
                            .to_owned();
                        if !facts.metadata_intents.contains(&index)
                            || evidence
                                != serde_json::json!({"index":index,"object":object,"revision":revision,"metadata_digest":digest})
                            || facts
                                .metadata_done
                                .insert(index, (object, revision))
                                .is_some()
                        {
                            return Err(failed("Invalid/repeated retained metadata completion."));
                        }
                    }
                }
                _ => {
                    return Err(failed(
                        "Unknown or reordered retained journal event; retain every object.",
                    ));
                }
            }
        }
        if !facts.old_intents.is_empty() || !facts.metadata_intents.is_empty() {
            let plan = self.plan(id)?.ok_or_else(|| {
                failed("Retained mutation lacks its immutable incoming copy plan.")
            })?;
            let journal =
                storage::SyncJournal::new_retained(self.catalog.clone(), id.into(), plan)?;
            let (cursors, terminal, _) = journal.history()?;
            if !terminal {
                return Err(failed(
                    "Retained moves/metadata require a completely verified incoming copy stream.",
                ));
            }
            for (index, (object, _)) in &facts.metadata_done {
                if cursors.get(*index).and_then(|c| c.object.as_ref()) != Some(object) {
                    return Err(failed(
                        "Restore metadata completion is not bound to its newly copied object.",
                    ));
                }
            }
        }
        Ok(facts)
    }
    pub(super) fn retained_recovery_review(
        &self,
        library: &Library,
        id: &str,
        binding: Binding,
    ) -> Result<SyncReview> {
        self.validate_binding(library, &binding)?;
        let facts = self.retained_facts(id, &binding)?;
        let retained = binding
            .retained
            .as_ref()
            .ok_or_else(|| failed("Missing retained scope."))?;
        let root = output_chain(&source::decode_path(&binding.scope.destination)?)?;
        let uncertain = facts.intent
            && facts.directory.is_none()
            && native::lookup(root.leaf(), &folder(id))?.is_some();
        let mut view = binding.view.clone();
        view.id = uuid::Uuid::new_v4().to_string();
        view.job_id = Some(id.into());
        view.available_bytes = Some(
            fs2::statvfs(&source::decode_path(&binding.scope.destination)?)?
                .available_space()
                .to_string(),
        );
        view.digest = blake3::hash(encode(&(&binding, &self.events(id)?))?.as_bytes())
            .to_hex()
            .to_string();
        view.execution_available = Self::capabilities()?.execution_available && !uncertain;
        view.reason = if uncertain {
            "A retained directory creation has no identified completion. Preserve the occupant; automatic adoption and deletion are forbidden.".into()
        } else {
            if retained.origin.is_some() {
                "Fresh recovery resumes byte comparison and the captured supported metadata policy only on already-owned restore outputs. Original history and current destination versions remain retained.".into()
            } else {
                "Fresh retained recovery review continues the existing exact history/new-object stream. It cannot remove an occupant, recreate a missing original or overwrite a destination.".into()
            }
        };
        if retained.origin.is_none()
            && let Err(e) = self.retained_proof(library, &binding)
        {
            view.execution_available = false;
            view.reason = e.message;
        }
        match self.retained_stream_digest(library, &binding) {
            Ok(digest) => view.digest = digest,
            Err(error) => {
                view.execution_available = false;
                view.reason = error.message;
            }
        }
        self.retain_review(Review {
            binding,
            view,
            stage: Stage::Retained,
            created: Instant::now(),
        })
    }
    pub(super) fn retained_versions(
        &self,
        library: &Library,
        id: &str,
    ) -> Result<Vec<SyncRetainedVersion>> {
        let binding = self.binding(id)?;
        self.validate_retained_history(library, &binding)?;
        let retained = binding
            .retained
            .as_ref()
            .filter(|r| r.origin.is_none())
            .ok_or_else(|| invalid("Select an original retained replacement job."))?;
        let facts = self.retained_facts(id, &binding)?;
        Ok(retained.versions.iter().enumerate().map(|(index,v)|SyncRetainedVersion{id:v.id.clone(),job_id:id.into(),display_path:source::decode_path(&v.old.path).map(|p|p.to_string_lossy().into_owned()).unwrap_or_else(|_|"Invalid path".into()),bytes:v.old.bytes.to_string(),full_digest:v.old_digest.clone(),retained:facts.old_done.contains_key(&index),restore_available:facts.old_done.len()==retained.versions.len() && Self::capabilities().is_ok_and(|c|c.execution_available),reason:if facts.old_done.len()==retained.versions.len(){"Restore copies the entire retained companion group into a new protected output; retained originals and the current destination remain unchanged.".into()}else{"Finish a fresh recovery review before restoring this version group.".into()}}).collect())
    }
    pub(super) fn review_retained_restore(
        &self,
        library: &Library,
        id: &str,
    ) -> Result<SyncReview> {
        let mut binding = self.binding(id)?;
        self.validate_retained_history(library, &binding)?;
        let facts = self.retained_facts(id, &binding)?;
        if facts.old_done.len()
            != binding
                .retained
                .as_ref()
                .ok_or_else(|| failed("Missing retained group."))?
                .versions
                .len()
        {
            return Err(invalid(
                "Identify and verify every retained old companion before restoring the group.",
            ));
        }
        let retained = binding
            .retained
            .as_mut()
            .filter(|r| r.origin.is_none())
            .ok_or_else(|| invalid("Select an original retained replacement job."))?;
        retained.origin = Some(id.into());
        retained.operation_id = uuid::Uuid::new_v4().to_string();
        let mut view = binding.view.clone();
        view.id = uuid::Uuid::new_v4().to_string();
        view.job_id = None;
        view.action = SyncAction::RestoreRetained;
        view.execution_available = Self::capabilities()?.execution_available;
        view.dataset_display_path = source::decode_path(&binding.scope.destination)?
            .join(folder(&retained.operation_id))
            .to_string_lossy()
            .into_owned();
        view.copy_bytes = retained
            .versions
            .iter()
            .map(|v| v.old.bytes)
            .sum::<u64>()
            .to_string();
        view.required_bytes = (view
            .copy_bytes
            .parse::<u128>()
            .map_err(|_| failed("Invalid restore byte count."))?
            + RESERVE
            + retained.versions.len() as u128 * 16384)
            .to_string();
        view.reason="Copy the complete retained old-version group into a new uniquely named protected output. Existing destination versions and history are preserved. Captured supported metadata is restored and compared; full native/security fidelity is limited to the disclosed policy.".into();
        view.metadata_warning = retained
            .versions
            .first()
            .and_then(|v| v.metadata.as_ref())
            .map(|m| m.description().to_owned())
            .unwrap_or_else(|| METADATA_WARNING.into());
        for (index, item) in view.items.iter_mut().enumerate() {
            item.action = SyncItemAction::RestoreToNewOutput;
            item.display_path = copy_leaf(retained, index)?;
            item.source_bytes = Some(retained.versions[index].old.bytes.to_string());
        }
        view.digest = blake3::hash(encode(&(&binding, &view))?.as_bytes())
            .to_hex()
            .to_string();
        binding.view = view.clone();
        self.retain_review(Review {
            binding,
            view,
            stage: Stage::Retained,
            created: Instant::now(),
        })
    }
    pub(super) fn run_retained(
        &self,
        library: &Library,
        id: &str,
        binding: &Binding,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let retained = binding
            .retained
            .as_ref()
            .filter(|r| r.operation_id == id)
            .ok_or_else(|| failed("Retained operation ID differs from its immutable plan."))?;
        self.validate_binding(library, binding)?;
        if retained.origin.is_none() {
            self.retained_proof(library, binding)?;
        }
        let src = source::open_root(
            &source::decode_path(&binding.scope.source)?,
            Some(&binding.scope.source_identity),
        )?;
        let dst = output_chain(&source::decode_path(&binding.scope.destination)?)?;
        let mut facts = self.retained_facts(id, binding)?;
        if !facts.intent {
            self.event(
                id,
                None,
                "retained/directory_intent",
                serde_json::json!({"name":folder(id)}),
            )?;
            facts.intent = true;
        }
        let directory = match (&facts.directory, native::lookup(dst.leaf(), &folder(id))?) {
            (Some(object), Some(entry))
                if entry.kind == native::EntryKind::Directory && entry.object == *object =>
            {
                native::read_directory(dst.leaf(), &folder(id))?
            }
            (None, None) => {
                let file = native::create_directory(dst.leaf(), &folder(id))?;
                #[cfg(test)]
                self.boundary(Fault::RetainedDirectoryCreated)?;
                let object = Object::of(&file)?;
                self.event(
                    id,
                    None,
                    "retained/directory_ready",
                    serde_json::json!({"name":folder(id),"object":object}),
                )?;
                facts.directory = Some(object);
                file
            }
            _ => {
                return Err(failed(
                    "Retained directory ownership is uncertain. Preserve the occupant and inspect recovery.",
                ));
            }
        };
        let restore = retained.origin.is_some();
        let copy_source = if restore { dst.leaf() } else { &src };
        let plan = if let Some(plan) = self.plan(id)? {
            plan
        } else {
            let mut files = Vec::new();
            for (index, v) in retained.versions.iter().enumerate() {
                let path = if let Some(origin) = &retained.origin {
                    PathBuf::from(folder(origin)).join(old_leaf(index))
                } else {
                    source::decode_path(&v.incoming.path)?
                };
                let location = locations::Source::capture(copy_source, &path)?;
                let source_file = location.open(copy_source)?;
                if restore {
                    if filesystem::identity(&source_file)? != v.old.identity
                        || source_file.metadata()?.len() != v.old.bytes
                        || hash(&source_file, cancel)? != v.old_digest
                    {
                        return Err(failed(
                            "Retained restore source differs from its exact original version.",
                        ));
                    }
                } else {
                    checked(&source_file, &v.incoming)?;
                }
                files.push(CopyFile {
                    source: format!("reviewed version {index}"),
                    target: copy_leaf(retained, index)?,
                    stage: format!("incoming-{index}.partial"),
                    object: Object::of(&source_file)?,
                    revision: source::revision(&source_file)?,
                    bytes: source_file.metadata()?.len(),
                    source_location: Some(location),
                    destination_location: Some(locations::Destination::capture(
                        dst.leaf(),
                        &folder(id),
                    )?),
                    stage_location: Some(locations::Destination::capture(dst.leaf(), &folder(id))?),
                    prepared_stage: None,
                });
            }
            let mut plan = CopyPlan {
                id: id.into(),
                digest: String::new(),
                source: Object::of(copy_source)?,
                destination: Object::of(dst.leaf())?,
                source_volume: if restore {
                    binding.scope.destination_volume.clone()
                } else {
                    binding.scope.source_volume.clone()
                },
                destination_volume: binding.scope.destination_volume.clone(),
                files,
            };
            plan.digest = plan.digest()?;
            self.bind_plan(id, &plan)?;
            plan
        };
        self.validate_retained_plan(binding, &plan, copy_source, dst.leaf())?;
        let journal =
            storage::SyncJournal::new_retained(self.catalog.clone(), id.into(), plan.clone())?;
        let (cursors, terminal, digest) = journal.history()?;
        let io = PacedIo::shared();
        let validate = || self.validate_binding(library, binding);
        if !terminal {
            let outcome = if cursors.iter().all(|c| c.phase == 0) {
                CopyContext {
                    plan: &plan,
                    journal: &journal,
                    source_root: copy_source,
                    destination: dst.leaf(),
                    io: &io,
                    cancel,
                    validate_scope: &validate,
                }
                .execute(self.copy_fault())?
            } else {
                let inputs = CopyInputs {
                    source_root: copy_source,
                    destination: dst.leaf(),
                    io: &io,
                    cancel,
                    validate_scope: &validate,
                };
                let inspection = resume::Review::capture(&plan, &cursors, digest.clone(), &inputs)?;
                resume::Continuation {
                    plan: &plan,
                    journal: &journal,
                    cursors: &cursors,
                    terminal,
                    review: &inspection,
                    history_digest: digest,
                    inputs: &inputs,
                    fault: self.copy_fault(),
                    progress: None,
                }
                .execute()?
            };
            if outcome != CopyOutcome::CompletedVerified {
                return Err(failed(
                    "Incoming staging stopped; retain all partial stages and inspect a fresh recovery review.",
                ));
            }
        }
        let (cursors, terminal, _) = journal.history()?;
        if !terminal {
            return Err(failed(
                "Retained publication requires the complete verified incoming stream.",
            ));
        }
        // A durable prior verification is not a fresh byte witness. Read every
        // incoming member back again BEFORE moving any old destination, also
        // when recovering a terminal copy stream after a process exit.
        for (index, version) in retained.versions.iter().enumerate() {
            let expected = cursors
                .get(index)
                .and_then(|c| c.object.as_ref())
                .ok_or_else(|| failed("Verified incoming stream lost its native identity."))?;
            let (parent, leaf) = if restore || !facts.new_intents.contains_key(&index) {
                (directory.try_clone()?, copy_leaf(retained, index)?)
            } else {
                let (parent, leaf) = open_parent(dst.leaf(), &binding.scope, &version.old)?;
                if native::lookup(&parent, &leaf)?.is_some() {
                    (parent, leaf)
                } else {
                    (directory.try_clone()?, new_leaf(index))
                }
            };
            let mut current = native::read(&parent, &leaf)?;
            if !expected.matches(&current)? {
                return Err(failed(
                    "Verified incoming object changed; every old version stays retained.",
                ));
            }
            let mut original = plan.files[index]
                .source_location
                .as_ref()
                .ok_or_else(|| failed("Incoming source locator is missing."))?
                .open(copy_source)?;
            let IoStep::Completed(witness) =
                compare(&io, &mut original, &mut current, &plan.files[index], cancel)?
            else {
                return Err(failed(
                    "Fresh incoming comparison canceled before old-version movement.",
                ));
            };
            witness.revalidate(&parent, &leaf, &current)?;
            if &witness.digest
                != if restore {
                    &version.old_digest
                } else {
                    &version.incoming_digest
                }
            {
                return Err(failed(
                    "Fresh staged incoming bytes differ from the immutable review.",
                ));
            }
        }
        if restore {
            for (index, v) in retained.versions.iter().enumerate() {
                let object = cursors
                    .get(index)
                    .and_then(|c| c.object.as_ref())
                    .ok_or_else(|| failed("Missing protected restore output identity."))?;
                let file = native::reopen_stage(
                    &directory,
                    &copy_leaf(retained, index)?,
                    &filesystem::FileIdentity {
                        volume: object.volume,
                        file: object.file,
                        links: 1,
                    },
                )?;
                if hash(&file, cancel)? != v.old_digest {
                    return Err(failed(
                        "Protected restore output no longer matches its retained old version.",
                    ));
                }
                if let Some(metadata) = &v.metadata {
                    let digest = metadata.digest()?;
                    if let Some((expected, revision)) = facts.metadata_done.get(&index) {
                        if !expected.matches(&file)? || source::revision(&file)? != *revision {
                            return Err(failed(
                                "Protected restored metadata changed after its identified completion.",
                            ));
                        }
                        metadata.compare(&file)?;
                    } else {
                        if !facts.metadata_intents.contains(&index) {
                            self.event(
                                id,
                                None,
                                "retained/metadata_intent",
                                serde_json::json!({"index":index,"metadata_digest":digest}),
                            )?;
                            facts.metadata_intents.insert(index);
                        }
                        metadata.restore_new(&file)?;
                        #[cfg(test)]
                        self.boundary(Fault::RetainedMetadataApplied)?;
                        file.sync_all()?;
                        metadata.compare(&file)?;
                        let revision = source::revision(&file)?;
                        self.event(id,None,"retained/metadata_verified",serde_json::json!({"index":index,"object":object,"revision":revision,"metadata_digest":digest}))?;
                        facts
                            .metadata_done
                            .insert(index, (object.clone(), revision));
                        #[cfg(test)]
                        self.boundary(Fault::RetainedMetadataRecorded)?;
                    }
                }
            }
        } else {
            for (index, v) in retained.versions.iter().enumerate() {
                preparation::canceled(cancel)?;
                self.validate_binding(library, binding)?;
                self.retained_proof(library, binding)?;
                let (parent, leaf) = open_parent(dst.leaf(), &binding.scope, &v.old)?;
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    facts.old_intents.entry(index)
                {
                    self.event(id,None,"retained/old_intent",serde_json::json!({"index":index,"identity":v.old.identity,"digest":v.old_digest}))?;
                    entry.insert(());
                    #[cfg(test)]
                    self.boundary(Fault::RetainedOldIntent)?;
                }
                let old = if let Some(file) = native::maybe_read(&directory, &old_leaf(index))? {
                    file
                } else {
                    let file = native::reopen_stage(&parent, &leaf, &v.old.identity)?;
                    checked(&file, &v.old)?;
                    if hash(&file, cancel)? != v.old_digest {
                        return Err(failed("OLD destination bytes changed before retention."));
                    }
                    native::publish_to(
                        &parent,
                        &leaf,
                        file,
                        &directory,
                        &old_leaf(index),
                        &v.old.identity,
                    )?
                };
                #[cfg(test)]
                self.boundary(Fault::RetainedOldMoved)?;
                if filesystem::identity(&old)? != v.old.identity
                    || old.metadata()?.len() != v.old.bytes
                {
                    return Err(failed("Retained OLD object identity or bytes changed."));
                }
                if descriptive(&old)? != (v.modified.clone(), v.created.clone())
                    || hash(&old, cancel)? != v.old_digest
                {
                    return Err(failed(
                        "Retained OLD destination bytes or descriptive metadata changed.",
                    ));
                }
                if let Some(metadata) = &v.metadata {
                    metadata.compare(&old)?;
                }
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    facts.old_done.entry(index)
                {
                    let object = Object::of(&old)?;
                    let revision = source::revision(&old)?;
                    self.event(
                        id,
                        None,
                        "retained/old_done",
                        serde_json::json!({"index":index,"object":object,"digest":v.old_digest,"revision":revision}),
                    )?;
                    entry.insert((object, revision));
                    #[cfg(test)]
                    self.boundary(Fault::RetainedOldRecorded)?;
                }
            }
            for (index, v) in retained.versions.iter().enumerate() {
                preparation::canceled(cancel)?;
                self.validate_binding(library, binding)?;
                let object = cursors
                    .get(index)
                    .and_then(|c| c.object.as_ref())
                    .ok_or_else(|| {
                        failed("Incoming verified stream has no retained object identity.")
                    })?
                    .clone();
                let (parent, leaf) = open_parent(dst.leaf(), &binding.scope, &v.old)?;
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    facts.new_intents.entry(index)
                {
                    self.event(
                        id,
                        None,
                        "retained/new_intent",
                        serde_json::json!({"index":index,"object":object}),
                    )?;
                    entry.insert(object.clone());
                    #[cfg(test)]
                    self.boundary(Fault::RetainedNewIntent)?;
                }
                let new = if let Some(file) = native::maybe_read(&parent, &leaf)? {
                    if !object.matches(&file)? {
                        return Err(failed(
                            "Late destination occupant is retained. No force or overwrite exists.",
                        ));
                    }
                    file
                } else {
                    let file = native::reopen_stage(
                        &directory,
                        &new_leaf(index),
                        &filesystem::FileIdentity {
                            volume: object.volume,
                            file: object.file,
                            links: 1,
                        },
                    )?;
                    if hash(&file, cancel)? != v.incoming_digest {
                        return Err(failed(
                            "Verified incoming bytes changed before publication.",
                        ));
                    }
                    native::publish_to(
                        &directory,
                        &new_leaf(index),
                        file,
                        &parent,
                        &leaf,
                        &filesystem::FileIdentity {
                            volume: object.volume,
                            file: object.file,
                            links: 1,
                        },
                    )?
                };
                #[cfg(test)]
                self.boundary(Fault::RetainedNewMoved)?;
                let location =
                    locations::Source::capture(&src, &source::decode_path(&v.incoming.path)?)?;
                let mut original = location.open(&src)?;
                let file = &plan.files[index];
                let IoStep::Completed(witness) =
                    compare(&io, &mut original, &mut new.try_clone()?, file, cancel)?
                else {
                    return Err(failed("New destination verification canceled."));
                };
                witness.revalidate(&parent, &leaf, &new)?;
                if witness.digest != v.incoming_digest {
                    return Err(failed(
                        "Published replacement differs from the exact reviewed incoming bytes.",
                    ));
                }
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    facts.new_done.entry(index)
                {
                    self.event(id,None,"retained/new_done",serde_json::json!({"index":index,"object":object,"digest":v.incoming_digest}))?;
                    entry.insert(object);
                    #[cfg(test)]
                    self.boundary(Fault::RetainedNewRecorded)?;
                }
            }
        }
        self.validate_binding(library, binding)?;
        if !facts.complete {
            self.event(
                id,
                None,
                "retained/completed",
                serde_json::json!({"files":retained.versions.len(),"restore":restore}),
            )?;
        }
        #[cfg(test)]
        self.boundary(Fault::RetainedCompleted)?;
        self.update(id,SyncState::CompletedVerified,"retained_complete",binding.view.action,None,if restore{"Every retained old version and companion was copied and byte-verified into a NEW protected output. The captured supported metadata policy was restored and compared. Current destination and all history remain retained; broader native/security fidelity is not certified."}else{"Every old destination version and companion remains retained as its original native object. New destination bytes were completely compared. All source and prior history versions remain retained."})
    }
    fn validate_retained_plan(
        &self,
        binding: &Binding,
        plan: &CopyPlan,
        src: &File,
        dst: &File,
    ) -> Result<()> {
        let retained = binding
            .retained
            .as_ref()
            .ok_or_else(|| failed("Missing retained plan binding."))?;
        if plan.id != retained.operation_id
            || plan.digest()? != plan.digest
            || !plan.source.matches(src)?
            || !plan.destination.matches(dst)?
            || plan.files.len() != retained.versions.len()
        {
            return Err(failed(
                "Retained copy plan differs from its exact native scope.",
            ));
        }
        for (index, (version, file)) in retained.versions.iter().zip(&plan.files).enumerate() {
            let (path, version) = if let Some(origin) = &retained.origin {
                (
                    PathBuf::from(folder(origin)).join(old_leaf(index)),
                    &version.old,
                )
            } else {
                (
                    source::decode_path(&version.incoming.path)?,
                    &version.incoming,
                )
            };
            let location = locations::Source::capture(src, &path)?;
            let native_source = location.open(src)?;
            let revision = if retained.origin.is_some() {
                source::revision(&native_source)?
            } else {
                version.revision.clone()
            };
            let destination =
                locations::Destination::capture(dst, &folder(&retained.operation_id))?;
            if file.object
                != (Object {
                    volume: version.identity.volume,
                    file: version.identity.file,
                })
                || file.bytes != version.bytes
                || file.revision != revision
                || file.target != copy_leaf(retained, index)?
                || file.stage != format!("incoming-{index}.partial")
                || file.prepared_stage.is_some()
                || file
                    .source_location
                    .as_ref()
                    .map(encode)
                    .transpose()?
                    .as_deref()
                    != Some(encode(&location)?.as_str())
                || file
                    .destination_location
                    .as_ref()
                    .map(encode)
                    .transpose()?
                    .as_deref()
                    != Some(encode(&destination)?.as_str())
                || file
                    .stage_location
                    .as_ref()
                    .map(encode)
                    .transpose()?
                    .as_deref()
                    != Some(encode(&destination)?.as_str())
            {
                return Err(failed(
                    "Retained copy locator differs from its immutable exact version.",
                ));
            }
        }
        Ok(())
    }
}
