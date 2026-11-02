//! Read-only native admission for M4B append planning; no transfer authority.
use super::*;
use crate::backup::append::{AppendPlan, AppendReview, plan};

#[derive(Clone)]
pub(super) struct FrozenScope {
    destination: Binding,
    namespace: PathBuf,
    source_objects: BTreeMap<PathBuf, ComparisonEntryKind>,
    destination_objects: BTreeMap<PathBuf, ComparisonEntryKind>,
    source_directories: BTreeMap<PathBuf, String>,
    destination_directories: BTreeMap<PathBuf, String>,
}
impl FrozenScope {
    pub(super) fn capture(
        source: &Inventory,
        destination: &Inventory,
        parent: &Binding,
        namespace: &Path,
    ) -> Self {
        Self {
            destination: parent.clone(),
            namespace: namespace.into(),
            source_objects: source.objects.clone(),
            destination_objects: destination.objects.clone(),
            source_directories: source.directories.iter().cloned().collect(),
            destination_directories: destination.directories.iter().cloned().collect(),
        }
    }
}

impl Library {
    #[cfg(any(test, feature = "fixture-probes"))]
    pub(crate) fn append_fixture_projection(
        &self,
        run_id: String,
        keep: Vec<String>,
    ) -> Result<crate::executor::append_fixture::Projection> {
        use crate::executor::append_fixture::{Observed, Projection};
        let plan = self.backup_append_review(run_id.clone(), keep)?;
        let current = self.comparisons.review.lock().map_err(|_| unavailable())?;
        let review = current
            .as_ref()
            .filter(|r| {
                r.run.id == run_id && r.run.plan_digest.as_deref() == Some(&plan.comparison_digest)
            })
            .ok_or_else(unavailable)?;
        canceled(&review.cancel)?;
        let frozen = review.append_scope.as_ref().ok_or_else(unavailable)?;
        let source = review.source.as_ref().ok_or_else(unavailable)?;
        let mut sources = Vec::new();
        let mut destinations = Vec::new();
        let copied: BTreeSet<_> = plan
            .items
            .iter()
            .filter(|item| item.action == crate::backup::append::AppendAction::CopyNew)
            .map(|item| item.entry_id.as_str())
            .collect();
        for (id, (path, a, b)) in &review.observations {
            let copy = copied.contains(id.as_str());
            let observed = |s: &Snapshot| Observed {
                path: native::encode_path(path),
                identity: s.identity.clone(),
                revision: s.revision.clone(),
                bytes: s.bytes,
                copy,
            };
            if let Some(a) = a {
                sources.push(observed(a));
            }
            if let Some(b) = b {
                destinations.push(observed(b));
            }
        }
        Ok(Projection {
            run_id,
            digest: plan.digest,
            catalog_id: review.run.catalog_id.clone(),
            root_id: review.run.source_root_id.clone(),
            source: native::encode_path(&source.path),
            source_identity: source.identity.clone(),
            source_volume: source.volume.clone(),
            destination: native::encode_path(&frozen.destination.path),
            destination_identity: frozen.destination.identity.clone(),
            destination_volume: frozen.destination.volume.clone(),
            namespace: native::encode_path(&frozen.namespace),
            sources,
            destinations,
            source_directories: frozen
                .source_directories
                .iter()
                .map(|(p, r)| (native::encode_path(p), r.clone()))
                .collect(),
            uncopied: plan.uncopied_source_revisions,
            destination_directories: frozen
                .destination_directories
                .iter()
                .map(|(path, revision)| {
                    let binding = review.destination.as_ref().ok_or_else(changed)?;
                    let directory = binding.directory(path)?;
                    if native::revision(&directory)? != *revision {
                        return Err(changed());
                    }
                    Ok((native::encode_path(path), filesystem::identity(&directory)?))
                })
                .collect::<Result<Vec<_>>>()?,
        })
    }
    /// Backend-only review. IDs refer to the complete current session report;
    /// display paths and the returned digest cannot be used to start I/O.
    pub fn backup_append_review(
        &self,
        run_id: String,
        keep_destination: Vec<String>,
    ) -> Result<AppendPlan> {
        let _gate = self.comparisons.preview_gate.try_lock().map_err(|_| {
            error(
                ErrorCode::Busy,
                "A comparison preview or append review is already running.",
            )
        })?;
        let review = self
            .comparisons
            .review
            .lock()
            .map_err(|_| unavailable())?
            .as_ref()
            .filter(|r| {
                r.run.id == run_id
                    && r.run.state == ComparisonState::NeedsReview
                    && r.run.mode == CompareMode::Verified
            })
            .cloned()
            .ok_or_else(unavailable)?;
        canceled(&review.cancel)?;
        let frozen = review.append_scope.as_ref().ok_or_else(unavailable)?;
        let source = review.source.as_ref().ok_or_else(unavailable)?;
        let scope = self.comparison_scope(
            frozen.destination.path.clone(),
            (
                frozen.destination.identity.clone(),
                frozen.destination.volume.clone(),
            ),
            Some(&review.run.source_root_id),
        )?;
        if scope.catalog_id != review.run.catalog_id
            || scope.namespace != frozen.namespace
            || scope.source.path != source.path
            || scope.source.identity != source.identity
            || scope.source.volume != source.volume
        {
            return Err(changed());
        }
        let mut src = Inventory::default();
        src.walk(source, Path::new(""), &review.cancel, 0)?;
        let mut dst = Inventory::default();
        match (
            frozen.destination.optional_directory(&frozen.namespace)?,
            &review.destination,
        ) {
            (None, None) => {}
            (Some(file), Some(binding))
                if same_directory(&filesystem::identity(&file)?, &binding.identity)
                    && native::volume_token(&file)? == binding.volume =>
            {
                dst.walk(binding, Path::new(""), &review.cancel, 0)?;
            }
            _ => return Err(changed()),
        }
        if src.incomplete
            || dst.incomplete
            || src.objects != frozen.source_objects
            || dst.objects != frozen.destination_objects
            || src.directories.iter().cloned().collect::<BTreeMap<_, _>>()
                != frozen.source_directories
            || dst.directories.iter().cloned().collect::<BTreeMap<_, _>>()
                != frozen.destination_directories
        {
            return Err(changed());
        }
        for (path, a, b) in review.observations.values() {
            if let Some(snapshot) = a {
                snapshot.validate(&source.file(path)?)?;
            }
            if let Some(snapshot) = b {
                snapshot.validate(
                    &review
                        .destination
                        .as_ref()
                        .ok_or_else(changed)?
                        .file(path)?,
                )?;
            }
        }
        src.validate(source)?;
        if let Some(binding) = &review.destination {
            dst.validate(binding)?;
        }
        frozen.destination.open()?;
        match (
            frozen.destination.optional_directory(&frozen.namespace)?,
            &review.destination,
        ) {
            (None, None) => {}
            (Some(file), Some(binding))
                if same_directory(&filesystem::identity(&file)?, &binding.identity)
                    && native::volume_token(&file)? == binding.volume => {}
            _ => return Err(changed()),
        }
        // Fingerprint lossless paths, volume/object/revision facts and namespace;
        // display strings are never the authority for a prospective target.
        let bindings = [
            Some(source),
            Some(&frozen.destination),
            review.destination.as_ref(),
        ]
        .into_iter()
        .map(|binding| binding.map(|b| (native::encode_path(&b.path), &b.identity, &b.volume)))
        .collect::<Vec<_>>();
        let observations = review
            .observations
            .iter()
            .map(|(id, (path, a, b))| {
                (
                    id,
                    native::encode_path(path),
                    a.as_ref().map(|s| (&s.identity, &s.revision, s.bytes)),
                    b.as_ref().map(|s| (&s.identity, &s.revision, s.bytes)),
                )
            })
            .collect::<Vec<_>>();
        let directories =
            [&frozen.source_directories, &frozen.destination_directories].map(|entries| {
                entries
                    .iter()
                    .map(|(path, revision)| (native::encode_path(path), revision))
                    .collect::<Vec<_>>()
            });
        let encoded = serde_json::to_vec(&(
            bindings,
            native::encode_path(&frozen.namespace),
            observations,
            directories,
        ))
        .map_err(|_| changed())?;
        let native_digest = blake3::hash(&encoded).to_hex().to_string();
        let result = plan(
            AppendReview {
                run_id: &run_id,
                comparison_digest: review.run.plan_digest.as_deref(),
                native_scope_digest: &native_digest,
                mode: review.run.mode,
                state: review.run.state,
                rows: &review.rows,
            },
            &keep_destination,
        )?;
        canceled(&review.cancel)?;
        self.validate_comparison_preview_root(source, &review.run.source_root_id)?;
        // A replacement comparison retires the captured cancellation token.
        let current = self.backup_comparison(run_id)?;
        if current.plan_digest != review.run.plan_digest {
            return Err(changed());
        }
        Ok(result)
    }
}
