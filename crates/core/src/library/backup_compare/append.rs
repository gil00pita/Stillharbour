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
    #[cfg(any(test, feature = "fixture-probes", feature = "backup-workflow"))]
    pub(crate) fn backup_replacement_scope(
        &self,
        run_id: String,
        entry_id: String,
    ) -> Result<(
        crate::executor::sync_workflow::Scope,
        crate::executor::sync_workflow::ReplacementVersions,
    )> {
        let keep = {
            let current = self.comparisons.review.lock().map_err(|_| unavailable())?;
            let review = current
                .as_ref()
                .filter(|r| r.run.id == run_id)
                .ok_or_else(unavailable)?;
            review
                .rows
                .iter()
                .filter(|r| {
                    matches!(
                        r.category,
                        ComparisonCategory::DifferentContent
                            | ComparisonCategory::DestinationEditedSinceLastRun
                    )
                })
                .map(|r| r.id.clone())
                .collect()
        };
        let scope = self.backup_sync_scope(run_id.clone(), keep)?;
        let current = self.comparisons.review.lock().map_err(|_| unavailable())?;
        let review = current
            .as_ref()
            .filter(|r| r.run.id == run_id)
            .ok_or_else(unavailable)?;
        let row = review
            .rows
            .iter()
            .find(|r| {
                r.id == entry_id
                    && matches!(
                        r.category,
                        ComparisonCategory::DifferentContent
                            | ComparisonCategory::DestinationEditedSinceLastRun
                    )
            })
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidInput,
                    "Select a verified differing destination version.",
                )
            })?;
        // Reconstruct the complete native companion group. The UI's bounded,
        // escaped companion labels are display only and may be truncated.
        let group = |path: &Path| -> Result<(PathBuf, String)> {
            let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
                error(
                    ErrorCode::Unsupported,
                    "This companion name needs another native-name adapter.",
                )
            })?;
            let mut stem = name.to_lowercase();
            if let Some((base, extension)) = stem.rsplit_once('.') {
                let sidecar = matches!(extension, "xmp" | "acr" | "aae" | "json");
                stem = base.to_owned();
                if sidecar && let Some((base, _)) = stem.rsplit_once('.') {
                    stem = base.to_owned();
                }
            }
            Ok((path.parent().unwrap_or(Path::new("")).into(), stem))
        };
        let key = group(&review.observations.get(&row.id).ok_or_else(changed)?.0)?;
        let mut ids = Vec::new();
        for (id, (path, _, _)) in &review.observations {
            let candidate = group(path)?;
            // Include multi-extension companions, for example
            // image.jpg.supplemental-metadata.json, using native names.
            if candidate.0 == key.0
                && (candidate.1 == key.1
                    || candidate.1.starts_with(&format!("{}.", key.1))
                    || key.1.starts_with(&format!("{}.", candidate.1)))
            {
                ids.push(id.clone());
            }
        }
        if ids.len() > 256 {
            return Err(error(
                ErrorCode::Unsupported,
                "The complete companion group exceeds the 256-version retained-operation bound.",
            ));
        }
        ids.sort();
        ids.dedup();
        let mut versions = Vec::new();
        for id in ids {
            let (path, incoming, old) = review.observations.get(&id).ok_or_else(changed)?;
            let (incoming, old) = (incoming.as_ref().ok_or_else(|| error(ErrorCode::Unsupported,"A companion has no incoming ordinary file; retain the whole group and resolve its scope first."))?, old.as_ref().ok_or_else(|| error(ErrorCode::Unsupported,"A companion has no old destination version; retain the group and resolve its scope first."))?);
            if incoming.identity.links != 1 || old.identity.links != 1 {
                return Err(error(
                    ErrorCode::Unsupported,
                    "Retained replacement requires ordinary single-link versions, including companions.",
                ));
            }
            let observed = |s: &Snapshot| crate::executor::sync_workflow::Observed {
                path: native::encode_path(path),
                identity: s.identity.clone(),
                revision: s.revision.clone(),
                bytes: s.bytes,
                copy: false,
            };
            versions.push((id, observed(incoming), observed(old)));
        }
        Ok((scope, versions))
    }
    #[cfg(any(test, feature = "fixture-probes", feature = "backup-workflow"))]
    pub(crate) fn backup_sync_scope(
        &self,
        run_id: String,
        keep: Vec<String>,
    ) -> Result<crate::executor::sync_workflow::Scope> {
        use crate::executor::sync_workflow::{Observed, Scope};
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
        Ok(Scope {
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
    #[cfg(any(test, feature = "fixture-probes"))]
    pub(crate) fn append_fixture_projection(
        &self,
        run_id: String,
        keep: Vec<String>,
    ) -> Result<crate::executor::append_fixture::Projection> {
        let scope = self.backup_sync_scope(run_id, keep)?;
        serde_json::from_value(serde_json::to_value(scope).map_err(|e| changed().caused_by(e))?)
            .map_err(|e| changed().caused_by(e))
    }

    #[cfg(any(test, feature = "fixture-probes", feature = "backup-workflow"))]
    pub(crate) fn validate_backup_sync_scope(
        &self,
        scope: &crate::executor::sync_workflow::Scope,
    ) -> Result<()> {
        self.validate_backup_sync_scope_except(scope, &[])
    }
    /// Replacement separately proves every exempted old/history/new object.
    /// Exemptions are backend-native paths, never renderer input.
    #[cfg(any(test, feature = "fixture-probes", feature = "backup-workflow"))]
    fn validate_backup_sync_bindings(
        &self,
        scope: &crate::executor::sync_workflow::Scope,
    ) -> Result<(Binding, Binding)> {
        let source_path = native::decode_path(&scope.source)?;
        let destination_path = native::decode_path(&scope.destination)?;
        let roots = store::roots(&self.catalog)?;
        let root = roots
            .iter()
            .find(|r| r.view.id == scope.root_id)
            .ok_or_else(changed)?;
        crate::library::original_jobs::require_reconciled_root(
            &self.catalog,
            scope.root_id.clone(),
        )?;
        if self.catalog.summary()?.id != scope.catalog_id
            || root.path != source_path
            || !same_directory(&root.identity, &scope.source_identity)
            || root.volume != scope.source_volume
            || root.view.availability != "online"
        {
            return Err(changed());
        }
        let source = Binding {
            path: source_path,
            identity: scope.source_identity.clone(),
            volume: scope.source_volume.clone(),
        };
        let destination = Binding {
            path: destination_path,
            identity: scope.destination_identity.clone(),
            volume: scope.destination_volume.clone(),
        };
        source.open()?;
        destination.open()?;
        let target = DirectoryChain::open(&destination.path.canonicalize()?)?;
        let data = DirectoryChain::open(&self.options.data_directory.canonicalize()?)?;
        if target.overlaps(&data) {
            return Err(changed());
        }
        for known in &roots {
            let file = native::open_root(&known.path, Some(&known.identity))?;
            if native::volume_token(&file)? != known.volume {
                return Err(changed());
            }
            let chain = DirectoryChain::open(&known.path.canonicalize()?)?;
            if target.overlaps(&chain) || (known.view.id == scope.root_id && chain.overlaps(&data))
            {
                return Err(changed());
            }
        }
        Ok((source, destination))
    }
    #[cfg(any(test, feature = "fixture-probes", feature = "backup-workflow"))]
    pub(crate) fn validate_backup_sync_roots(
        &self,
        scope: &crate::executor::sync_workflow::Scope,
    ) -> Result<()> {
        self.validate_backup_sync_bindings(scope).map(|_| ())
    }
    #[cfg(any(test, feature = "fixture-probes", feature = "backup-workflow"))]
    pub(crate) fn validate_backup_sync_scope_except(
        &self,
        scope: &crate::executor::sync_workflow::Scope,
        replaced: &[Vec<u8>],
    ) -> Result<()> {
        let (source, destination) = self.validate_backup_sync_bindings(scope)?;
        let mut current = Inventory::default();
        current.walk(&source, Path::new(""), &AtomicBool::new(false), 0)?;
        if current.incomplete
            || current.files.len() != scope.sources.len()
            || current.directories.len() != scope.source_directories.len()
        {
            return Err(changed());
        }
        for observed in &scope.sources {
            let path = native::decode_path(&observed.path)?;
            let actual = current.files.get(&path).ok_or_else(changed)?;
            if actual.identity != observed.identity
                || actual.revision != observed.revision
                || actual.bytes != observed.bytes
            {
                return Err(changed());
            }
        }
        for (path, revision) in &scope.source_directories {
            let path = native::decode_path(path)?;
            if !current
                .directories
                .iter()
                .any(|(p, r)| p == &path && r == revision)
            {
                return Err(changed());
            }
        }
        let dataset = destination.optional_directory(&native::decode_path(&scope.namespace)?)?;
        if !scope.destination_directories.is_empty() || !scope.destinations.is_empty() {
            let dataset = dataset.ok_or_else(changed)?;
            let binding = Binding {
                path: destination
                    .path
                    .join(native::decode_path(&scope.namespace)?),
                identity: filesystem::identity(&dataset)?,
                volume: scope.destination_volume.clone(),
            };
            for (path, identity) in &scope.destination_directories {
                let file = binding.directory(&native::decode_path(path)?)?;
                if !same_directory(&filesystem::identity(&file)?, identity) {
                    return Err(changed());
                }
            }
            for observed in &scope.destinations {
                if replaced.contains(&observed.path) {
                    continue;
                }
                let file = binding.file(&native::decode_path(&observed.path)?)?;
                if filesystem::identity(&file)? != observed.identity
                    || native::revision(&file)? != observed.revision
                    || file.metadata()?.len() != observed.bytes
                {
                    return Err(changed());
                }
            }
        }
        Ok(())
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
