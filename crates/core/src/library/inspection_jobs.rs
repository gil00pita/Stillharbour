//! One background inspection per catalog. No renderer path or execution grant.
use super::{
    inspection_inventory::Inventory, inspection_sources::NativeSources, inspection_store as store,
    workers::WorkerPaths,
};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    inspection::{candidates, model::*, storage},
    jobs::{
        paced_io::{IoStep, PacedIo},
        resources::ResourceScheduler,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

struct Active {
    id: String,
    cancel: Arc<AtomicBool>,
}
#[derive(Default)]
pub(super) struct InspectionJobs {
    active: Mutex<Option<Active>>,
}
impl Drop for InspectionJobs {
    fn drop(&mut self) {
        if let Ok(active) = self.active.get_mut()
            && let Some(active) = active
        {
            active.cancel.store(true, Ordering::Release);
        }
    }
}
impl InspectionJobs {
    pub(super) fn stop(&self) {
        if let Ok(active) = self.active.lock()
            && let Some(active) = active.as_ref()
        {
            active.cancel.store(true, Ordering::Release);
        }
    }
    pub(super) fn idle(&self) -> Result<bool> {
        Ok(self
            .active
            .lock()
            .map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Inspection state is unavailable.")
            })?
            .is_none())
    }
    pub(super) fn start(
        self: &Arc<Self>,
        catalog: Arc<Catalog>,
        data: PathBuf,
        paths: WorkerPaths,
        resources: ResourceScheduler,
        input: InspectionInput,
    ) -> Result<InspectionJob> {
        input.validate()?;
        let mut active = self.active.lock().map_err(|_| {
            AppError::new(
                ErrorCode::WorkerFailed,
                "Inspection admission is unavailable.",
            )
        })?;
        if let Some(current) = active.as_ref() {
            let job = store::job(&catalog, current.id.clone())?;
            if matches!(
                job.state,
                InspectionState::Queued
                    | InspectionState::Running
                    | InspectionState::CancelRequested
            ) {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "An inspection is active. Cancel it or wait for its retained outcome.",
                ));
            }
            *active = None;
        }
        let job = store::create(&catalog, input)?;
        let cancel = Arc::new(AtomicBool::new(false));
        *active = Some(Active {
            id: job.id.clone(),
            cancel: cancel.clone(),
        });
        let owner = Arc::downgrade(self);
        let queued = job.clone();
        let db = catalog.clone();
        let spawn=std::thread::Builder::new().name("library-inspection".into()).spawn(move||{
            let outcome=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||run(&db,&data,&paths,resources,queued.clone(),cancel.clone())));
            let result=match outcome{Ok(result)=>result,Err(_)=>Err(AppError::new(ErrorCode::WorkerFailed,"Inspection worker failed; originals and retained observations are unchanged."))};
            if let Err(error)=result {
                let mut stopped=store::job(&db,queued.id.clone()).unwrap_or_else(|_|queued.clone());stopped.limited=error.code==ErrorCode::Unsupported;stopped.state=if cancel.load(Ordering::Acquire){InspectionState::Canceled}else{InspectionState::Failed};stopped.message=error.message;stopped.issues=1;
                let _=store::finish(&db,stopped);
            }
            if let Some(owner)=owner.upgrade() && let Ok(mut active)=owner.active.lock() && active.as_ref().is_some_and(|active|active.id==queued.id){*active=None;}
        });
        if let Err(error) = spawn {
            *active = None;
            let mut failed = job.clone();
            failed.state = InspectionState::Failed;
            failed.message = "The inspection worker could not start.".into();
            store::finish(&catalog, failed)?;
            return Err(AppError::new(
                ErrorCode::WorkerFailed,
                "The inspection worker could not start.",
            )
            .caused_by(error));
        }
        Ok(job)
    }
    pub(super) fn cancel(&self, catalog: &Catalog, id: String) -> Result<InspectionJob> {
        crate::operations::validate_id(&id)?;
        let active = self.active.lock().map_err(|_| {
            AppError::new(ErrorCode::WorkerFailed, "Inspection state is unavailable.")
        })?;
        if let Some(active) = active.as_ref().filter(|active| active.id == id) {
            active.cancel.store(true, Ordering::Release);
        }
        store::cancel(catalog, id)
    }
    pub(super) fn root_busy(&self, catalog: &Catalog, root: &str) -> Result<bool> {
        let active = self.active.lock().map_err(|_| {
            AppError::new(ErrorCode::WorkerFailed, "Inspection state is unavailable.")
        })?;
        match active.as_ref() {
            Some(active) => {
                let job = store::job(catalog, active.id.clone())?;
                Ok(job.input.root_ids.iter().any(|id| id == root)
                    && matches!(
                        job.state,
                        InspectionState::Queued
                            | InspectionState::Running
                            | InspectionState::CancelRequested
                    ))
            }
            None => Ok(false),
        }
    }
}
fn run(
    catalog: &Arc<Catalog>,
    data: &std::path::Path,
    paths: &WorkerPaths,
    resources: ResourceScheduler,
    mut job: InspectionJob,
    cancel: Arc<AtomicBool>,
) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        job.state = InspectionState::Canceled;
        job.message = "Inspection canceled before reading files.".into();
        store::finish(catalog, job)?;
        return Ok(());
    }
    store::begin(catalog, job.id.clone())?;
    let sources = NativeSources::new(
        catalog.clone(),
        job.input.root_ids.clone(),
        resources,
        cancel.clone(),
    )?;
    store::save_roots(catalog, job.id.clone(), sources.roots.clone())?;
    let policy = match &job.input.task {
        InspectionTask::Duplicates { options } => Some(&options.rules.name),
        _ => None,
    };
    let mut inventory = Inventory::collect(sources, job.input.scope, data, policy)?;
    job.files_observed = inventory.sources.entries.len() as u32;
    store::progress(catalog, &job)?;
    for (ordinal, (entry, finding)) in inventory
        .sources
        .entries
        .iter()
        .zip(&inventory.findings)
        .enumerate()
    {
        if cancel.load(Ordering::Acquire) {
            break;
        }
        store::insert_item(
            catalog,
            job.id.clone(),
            ordinal as u32,
            entry,
            finding.clone(),
        )?;
    }
    if cancel.load(Ordering::Acquire) {
        job.state = InspectionState::Canceled;
        job.message = "Inspection canceled; partial observations retained.".into();
        store::finish(catalog, job)?;
        return Ok(());
    }
    let mut groups = Vec::new();
    let mut pair_count = 0;
    let mut unknown_pairs = 0;
    let mut exact_count = 0;
    let mut alias_count = 0;
    let mut potential = None;
    let mut issues = inventory.issues.clone();
    if let InspectionTask::Health {
        mode: mode @ (HealthMode::CompareBaseline | HealthMode::EstablishBaseline),
    } = job.input.task
    {
        use crate::inspection::{candidates::Sources, hashing};
        let io = PacedIo::shared();
        let mut stats = hashing::Stats::default();
        for (index, entry) in inventory.sources.entries.iter().enumerate() {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            let finding = &mut inventory.findings[index];
            let outcome = (|| -> Result<IoStep<String>> {
                let mut reader = inventory.sources.open(index)?;
                let digest = hashing::full(
                    &mut reader,
                    entry.bytes,
                    ContentAlgorithm::Blake3,
                    &io,
                    &cancel,
                    &mut stats,
                )?;
                inventory.sources.validate(index, &reader)?;
                Ok(digest)
            })();
            match outcome {
                Ok(IoStep::Canceled) => break,
                Ok(IoStep::Completed(digest)) => {
                    if mode == HealthMode::EstablishBaseline {
                        let baseline = store::append_baseline(
                            catalog,
                            job.id.clone(),
                            entry,
                            &inventory.sources.roots[entry.root],
                            digest,
                        )?;
                        finding.baseline_id = Some(baseline.id);
                        finding.checksum = ChecksumState::MatchesBaseline;
                    } else if let Some(baseline) =
                        store::latest_baseline(catalog, entry.id.clone())?
                    {
                        finding.baseline_id = Some(baseline.id);
                        finding.checksum = if baseline.algorithm == "blake3"
                            && baseline.policy == format!("{}:blake3-1.8.7", hashing::POLICY)
                        {
                            if baseline.digest == digest
                                && baseline.bytes == entry.bytes.to_string()
                            {
                                ChecksumState::MatchesBaseline
                            } else {
                                ChecksumState::ChangedSinceBaseline
                            }
                        } else {
                            ChecksumState::Unavailable
                        };
                    } else {
                        finding.checksum = ChecksumState::NoBaseline;
                    }
                    finding.coverage="Complete newly read BLAKE3 bytes; no image decode. A baseline change is not proof of damage.".into();
                    finding.decoder_policy = Some(format!("{}:blake3-1.8.7", hashing::POLICY));
                    job.files_checked += 1;
                }
                Err(error) => {
                    finding.inventory = super::inspection_sources::inventory_state(&error);
                    finding.checksum = ChecksumState::Unavailable;
                    finding.error = Some(error.clone());
                    issues.push(InspectionIssue {
                        root_id: finding.root_id.clone(),
                        display_path: finding.display_path.clone(),
                        entry_kind: IssueEntryKind::File,
                        inventory: finding.inventory,
                        error,
                    });
                }
            }
            store::update_finding(catalog, job.id.clone(), finding.clone())?;
            if index % 8 == 0 {
                job.bytes_read = inventory
                    .sources
                    .bytes_read
                    .load(Ordering::Relaxed)
                    .to_string();
                store::progress(catalog, &job)?;
            }
        }
        job.bytes_read = stats.bytes_read.to_string();
    }
    job.limited = inventory.sources.limited;
    if let InspectionTask::Duplicates { options } = &job.input.task
        && options.mode != MatchMode::NormalizedPixels
    {
        let outcome = candidates::scan(
            &inventory.sources,
            &inventory.sources.items(),
            options,
            &PacedIo::shared(),
            &cancel,
        )?;
        let IoStep::Completed(report) = outcome else {
            job.state = InspectionState::Canceled;
            job.message =
                "Comparison canceled; retained inventory is not a completed duplicate report."
                    .into();
            store::finish(catalog, job)?;
            return Ok(());
        };
        for fingerprint in report.fingerprints {
            let entry = &inventory.sources.entries[fingerprint.index];
            store::fingerprint(catalog, entry, fingerprint)?;
        }
        pair_count = report.compared_pairs;
        unknown_pairs = report.unknown_pairs;
        job.bytes_read = report.stats.bytes_read.to_string();
        job.limited |= report.limit_reached;
        exact_count = report.groups.len() as u32;
        alias_count = report.aliases.len() as u32;
        let by_id = inventory
            .sources
            .entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry))
            .collect::<BTreeMap<_, _>>();
        if options.mode == MatchMode::ExactContent {
            let mut bytes = 0u128;
            for ids in &report.groups {
                let mut objects = BTreeSet::new();
                let mut size = 0;
                for id in ids {
                    if let Some(entry) = by_id.get(id.as_str()) {
                        objects.insert(entry.object_key());
                        size = entry.bytes;
                    }
                }
                bytes += objects.len().saturating_sub(1) as u128 * u128::from(size);
            }
            potential = Some(bytes.to_string());
        }
        let evidence = if options.mode == MatchMode::ExactContent {
            DuplicateEvidence::ExactBytes
        } else {
            DuplicateEvidence::PropertyCandidate
        };
        groups.extend(report.groups.into_iter().map(|ids| (evidence, ids)));
        groups.extend(
            report
                .aliases
                .into_iter()
                .map(|ids| (DuplicateEvidence::HardLinkAliases, ids)),
        );
        for (id, error) in report.issues {
            if let Some(entry) = by_id.get(id.as_str()) {
                issues.push(InspectionIssue {
                    root_id: inventory.sources.roots[entry.root].view.id.clone(),
                    display_path: inventory.sources.roots[entry.root]
                        .path
                        .join(&entry.relative)
                        .to_string_lossy()
                        .into_owned(),
                    entry_kind: IssueEntryKind::File,
                    inventory: super::inspection_sources::inventory_state(&error),
                    error,
                });
            }
        }
    }
    let decode_mode = match &job.input.task {
        InspectionTask::Health {
            mode: HealthMode::Quick,
        } => Some(crate::inspection::decoder::DecodeMode::Quick),
        InspectionTask::Health {
            mode: HealthMode::Deep,
        } => Some(crate::inspection::decoder::DecodeMode::Deep),
        InspectionTask::Duplicates { options } if options.mode == MatchMode::NormalizedPixels => {
            Some(crate::inspection::decoder::DecodeMode::Pixels)
        }
        _ => None,
    };
    if let Some(mode) = decode_mode {
        let mut pixels = BTreeMap::<(String, String), Vec<String>>::new();
        for index in 0..inventory.sources.entries.len() {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            let finding = &mut inventory.findings[index];
            if matches!(&job.input.task,InspectionTask::Duplicates{options} if options.exclude_hardlinked_objects && inventory.sources.entries[index].identity.links>1)
            {
                continue;
            }
            match inventory.sources.decode(index, paths, mode) {
                Ok(decoded) => {
                    finding.content = decoded.state;
                    finding.decoder_policy = Some(decoded.policy);
                    finding.pixel_signature = decoded.pixel_signature.clone();
                    finding.pixel_digest = decoded.pixel_digest.clone();
                    finding.width = decoded.width;
                    finding.height = decoded.height;
                    finding.format = decoded.format;
                    finding.coverage = decoded.coverage;
                    finding.error = decoded.error;
                    if let (Some(signature), Some(digest)) =
                        (decoded.pixel_signature, decoded.pixel_digest)
                    {
                        store::fingerprint(
                            catalog,
                            &inventory.sources.entries[index],
                            candidates::Fingerprint {
                                index,
                                algorithm: ContentAlgorithm::Blake3,
                                policy: signature.clone(),
                                digest: digest.clone(),
                            },
                        )?;
                        pixels
                            .entry((signature, digest))
                            .or_default()
                            .push(finding.id.clone());
                    }
                    if matches!(
                        finding.content,
                        ContentState::QuickPass | ContentState::DeepPass
                    ) {
                        job.files_checked += 1;
                    }
                }
                Err(error) => {
                    if matches!(error.code, ErrorCode::Io | ErrorCode::Conflict) {
                        finding.inventory = super::inspection_sources::inventory_state(&error);
                    }
                    finding.content = if error.code == ErrorCode::Conflict {
                        ContentState::ChangedDuringRead
                    } else if error.code == ErrorCode::Unsupported {
                        ContentState::Unsupported
                    } else {
                        ContentState::DecoderWarning
                    };
                    finding.coverage =
                        "The supervised decoder could not complete its declared coverage.".into();
                    finding.error = Some(error);
                }
            }
            if let Some(error) = finding.error.clone() {
                issues.push(InspectionIssue {
                    root_id: finding.root_id.clone(),
                    display_path: finding.display_path.clone(),
                    entry_kind: IssueEntryKind::File,
                    inventory: finding.inventory,
                    error,
                });
            }
            store::update_finding(catalog, job.id.clone(), finding.clone())?;
            if index % 8 == 0 {
                job.bytes_read = inventory
                    .sources
                    .bytes_read
                    .load(Ordering::Relaxed)
                    .to_string();
                store::progress(catalog, &job)?;
            }
        }
        let objects = inventory
            .sources
            .entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry.object_key()))
            .collect::<BTreeMap<_, _>>();
        for ids in pixels.into_values().filter(|ids| ids.len() > 1) {
            let independent = ids
                .iter()
                .filter_map(|id| objects.get(id.as_str()))
                .collect::<BTreeSet<_>>();
            if independent.len() < 2 {
                continue;
            }
            if groups.len() >= candidates::MAX_GROUPS {
                job.limited = true;
                break;
            }
            groups.push((DuplicateEvidence::NormalizedPixels, ids));
        }
        exact_count = groups.len() as u32;
    }
    job.bytes_read = inventory
        .sources
        .bytes_read
        .load(Ordering::Relaxed)
        .to_string();
    let sizes = storage::aggregate(
        job.input.scope,
        &inventory.sizes(),
        inventory.coverage.clone(),
    )?;
    let total = sizes
        .folders
        .iter()
        .find(|folder| folder.id == "scope")
        .cloned()
        .ok_or_else(|| crate::inspection::invalid("Storage scope totals are missing."))?;
    let mut volumes = BTreeMap::<String, InspectionVolume>::new();
    for (index, root) in inventory.sources.roots.iter().enumerate() {
        inventory.sources.validate_root(index)?;
        let id = blake3::hash(root.volume.as_bytes()).to_hex().to_string();
        if let Some(volume) = volumes.get_mut(&id) {
            volume.root_ids.push(root.view.id.clone());
            continue;
        }
        let stats = fs2::statvfs(&root.path).ok();
        inventory.sources.validate_root(index)?;
        volumes.insert(id.clone(),InspectionVolume {id,root_ids:vec![root.view.id.clone()],total_bytes:stats.as_ref().map(|stats|stats.total_space().to_string()),available_bytes:stats.as_ref().map(|stats|stats.available_space().to_string()),observed_at:super::store::now(),coverage:"Observed filesystem capacity for this actual volume; available bytes may change and shared pools, reservations, clones and snapshots affect capacity. Selected-scope logical totals are separate; this is no saving or destination reservation.".into()});
    }
    let summary = InspectionReport {
        volumes: volumes.into_values().collect(),
        job_id: job.id.clone(),
        coverage: inventory.coverage.clone(),
        exact_or_candidate_groups: exact_count,
        alias_groups: alias_count,
        compared_pairs: pair_count.to_string(),
        unknown_pairs: unknown_pairs.to_string(),
        storage: Some(StorageSummary {
            total,
            potential_duplicate_logical_bytes: potential,
            reclaimable_physical_bytes: None,
            quarantine_freed_bytes: "0".into(),
            accounting_incomplete: sizes.accounting_incomplete,
        }),
        id3_applicability: "Not applicable to photo-only comparison modes".into(),
    };
    job.issues = issues.len() as u32;
    if !matches!(job.input.task, InspectionTask::Health { .. }) && decode_mode.is_none() {
        job.files_checked = job.files_observed;
    }
    job.limited |= sizes.accounting_incomplete;
    inventory.sources.check_all_roots()?;
    store::save_report(catalog, summary, sizes.folders, groups, issues)?;
    job.state = if cancel.load(Ordering::Acquire) {
        InspectionState::Canceled
    } else if job.issues > 0 || job.limited || !inventory.coverage.enumeration_complete {
        InspectionState::Partial
    } else {
        InspectionState::Completed
    };
    job.message = if job.state == InspectionState::Completed {
        "Inspection finished; originals unchanged. Reports grant no quarantine approval."
    } else {
        "Inspection retained partial or canceled observations; review coverage and issues."
    }
    .into();
    store::finish(catalog, job)?;
    Ok(())
}

impl super::Library {
    pub fn inspection_issues(
        &self,
        id: String,
        after: Option<String>,
    ) -> Result<InspectionIssuePage> {
        store::issues(&self.catalog, id, after)
    }
    pub fn inspection_keeper_latest(
        &self,
        job: String,
        group: String,
    ) -> Result<Option<crate::inspection::KeeperReview>> {
        store::latest_keeper_review(&self.catalog, job, group)
    }
    pub fn inspection_start(&self, input: InspectionInput) -> Result<InspectionJob> {
        self.inspections.start(
            self.catalog.clone(),
            self.options.data_directory.clone(),
            self.options.workers.clone(),
            self.resources.clone(),
            input,
        )
    }
    pub fn inspection_cancel(&self, id: String) -> Result<InspectionJob> {
        self.inspections.cancel(&self.catalog, id)
    }
    pub fn inspection_job(&self, id: String) -> Result<InspectionJob> {
        store::job(&self.catalog, id)
    }
    pub fn inspection_jobs(&self) -> Result<Vec<InspectionJob>> {
        store::jobs(&self.catalog)
    }
    pub fn inspection_findings(&self, id: String, after: Option<String>) -> Result<FindingPage> {
        store::findings(&self.catalog, id, after)
    }
    pub fn inspection_groups(&self, id: String, after: Option<String>) -> Result<DuplicatePage> {
        store::groups(&self.catalog, id, after)
    }
    pub fn inspection_report(&self, id: String) -> Result<Option<InspectionReport>> {
        store::report(&self.catalog, id)
    }
    pub fn inspection_group_members(
        &self,
        id: String,
        group: String,
        after: Option<String>,
    ) -> Result<FindingPage> {
        store::group_members(&self.catalog, id, group, after)
    }
    pub fn inspection_folders(
        &self,
        id: String,
        parent: String,
        after: Option<String>,
    ) -> Result<FolderPage> {
        store::folders(&self.catalog, id, parent, after)
    }
}

impl super::Library {
    pub fn inspection_keeper_review(
        &self,
        input: crate::inspection::KeeperReviewInput,
    ) -> Result<crate::inspection::KeeperReview> {
        use super::inspection_sources::Snapshot;
        use crate::inspection::{
            KeeperReview,
            keepers::{self, Member},
        };
        if input
            .manual_keepers
            .iter()
            .chain(&input.protected_folders)
            .chain(&input.preferred_folders)
            .any(|id| id.len() > 128)
        {
            return Err(crate::inspection::invalid(
                "Invalid keeper member/folder identity.",
            ));
        }
        if input.manual_keepers.len() > 4096
            || input.protected_folders.len() > 64
            || input.preferred_folders.len() > 64
        {
            return Err(crate::inspection::invalid(
                "Keeper policy exceeds its bounded selection budget.",
            ));
        }
        let (evidence, retained) =
            store::reviewed_members(&self.catalog, input.job_id.clone(), input.group_id.clone())?;
        let mut folders = BTreeSet::new();
        for record in &retained {
            let mut parent = record.relative.parent();
            while let Some(path) = parent {
                folders.insert(Snapshot::folder_id(&record.finding.root_id, path));
                parent = path.parent();
            }
        }
        if input
            .protected_folders
            .iter()
            .chain(&input.preferred_folders)
            .any(|id| !folders.contains(id))
        {
            return Err(crate::inspection::invalid(
                "A keeper folder is outside the retained group scope.",
            ));
        }
        let roots = retained
            .iter()
            .map(|record| record.finding.root_id.clone())
            .collect::<BTreeSet<_>>();
        let mut scopes = BTreeMap::new();
        for root in roots {
            if let Ok(scope) = NativeSources::new(
                self.catalog.clone(),
                vec![root.clone()],
                self.resources.clone(),
                Arc::new(AtomicBool::new(false)),
            ) {
                scopes.insert(root, scope);
            }
        }
        let mut members = Vec::new();
        for record in retained {
            let mut online = false;
            if let Some(scope) = scopes.get_mut(&record.finding.root_id)
                && scope
                    .capture(0, record.relative.clone(), record.finding.id.clone())
                    .is_ok()
                && let Some(current) = scope.entries.last()
            {
                online = current.object_key() == record.object
                    && current.revision == record.revision
                    && current.bytes.to_string() == record.finding.bytes;
            }
            let mut ancestors = BTreeSet::new();
            let mut parent = record.relative.parent();
            while let Some(path) = parent {
                ancestors.insert(Snapshot::folder_id(&record.finding.root_id, path));
                parent = path.parent();
            }
            let date = |value: Option<String>| {
                value
                    .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
                    .map(|time| time.timestamp_millis())
            };
            members.push(Member {
                id: record.finding.id,
                object: record.object,
                online,
                protected: input
                    .protected_folders
                    .iter()
                    .any(|id| ancestors.contains(id)),
                preferred_rank: input
                    .preferred_folders
                    .iter()
                    .position(|id| ancestors.contains(id))
                    .map_or(u32::MAX, |rank| rank as u32),
                modified_millis: date(record.finding.filesystem_modified),
                created_millis: date(record.finding.filesystem_created),
                // Inventory companions are candidates. A separate confirmed, freshly
                // compared bundle plan is required before any cleanup operation.
                bundle_complete: false,
                unique_companion: false,
            });
        }
        let (keepers, suggestion_block) =
            match keepers::suggest(&members, input.order, &input.manual_keepers)? {
                Ok(ids) => (ids, None),
                Err(reason) => (Vec::new(), Some(reason)),
            };
        let selected = members
            .iter()
            .filter(|member| !keepers.contains(&member.id))
            .map(|member| member.id.clone())
            .collect();
        let mode = match evidence {
            DuplicateEvidence::ExactBytes => MatchMode::ExactContent,
            DuplicateEvidence::NormalizedPixels => MatchMode::NormalizedPixels,
            _ => MatchMode::Properties,
        };
        let blocks = keepers::review_selection(&members, &selected, mode, &BTreeSet::new())?;
        store::save_keeper_review(&self.catalog,KeeperReview { id:uuid::Uuid::new_v4().to_string(),input,keepers,suggestion_block,blocks,mutation_available:false,explanation:"Protected folders and explicit manual keepers are retained first. Otherwise preferred folder order precedes the selected filesystem date; unknown dates require review and ties use stable IDs. Fresh bytes, confirmed complete bundles and independent recovery remain required. This retained review grants no mutation approval.".into() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        filesystem,
        library::{
            model::{RootMode, ScanFilter},
            native, store as roots,
        },
    };
    fn settled(catalog: &Catalog, id: &str) -> InspectionJob {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            let job = store::job(catalog, id.to_owned()).unwrap();
            if !matches!(
                job.state,
                InspectionState::Queued
                    | InspectionState::Running
                    | InspectionState::CancelRequested
            ) {
                return job;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Inspection did not settle: {}",
                job.message
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[test]
    fn native_job_pipeline_retains_exact_groups_sizes_and_append_only_changed_baselines() {
        let owned = tempfile::tempdir().unwrap();
        let source = owned.path().join("source");
        let data = owned.path().join("data");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&data).unwrap();
        for (name, count, byte) in [
            ("one.bin", 100, 1),
            ("different-name.dat", 100, 1),
            ("two.bin", 200, 2),
            ("three.bin", 300, 3),
        ] {
            let path = source.join(name);
            assert!(path.starts_with(owned.path()));
            std::fs::write(path, vec![byte; count]).unwrap();
        }
        let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
        let handle = native::open_root(&source, None).unwrap();
        let root = roots::add_root(
            &catalog,
            source.clone(),
            filesystem::identity(&handle).unwrap(),
            native::volume_token(&handle).unwrap(),
            "Generated M3 source".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
        drop(handle);
        let controller = Arc::new(InspectionJobs::default());
        let run = |task| {
            let job = controller
                .start(
                    catalog.clone(),
                    data.clone(),
                    WorkerPaths {
                        executable: "not-used-by-these-tasks".into(),
                        exiftool: None,
                        working_directory: data.clone(),
                    },
                    ResourceScheduler::default(),
                    InspectionInput {
                        root_ids: vec![root.clone()],
                        scope: crate::inspection::InspectionScope::AllEligibleFiles,
                        task,
                    },
                )
                .unwrap();
            let job = settled(&catalog, &job.id);
            assert_eq!(job.state, InspectionState::Completed, "{}", job.message);
            job
        };
        let sizes = run(InspectionTask::Storage {});
        let page = store::findings(&catalog, sizes.id.clone(), None).unwrap();
        assert_eq!(
            page.rows
                .iter()
                .map(|row| row.bytes.as_str())
                .collect::<Vec<_>>(),
            ["300", "200", "100", "100"]
        );
        assert_eq!(
            store::report(&catalog, sizes.id)
                .unwrap()
                .unwrap()
                .storage
                .unwrap()
                .total
                .logical_bytes,
            "700"
        );
        let options = DuplicateOptions {
            mode: MatchMode::ExactContent,
            algorithm: ContentAlgorithm::Sha256,
            rules: PropertyRules {
                logic: RuleLogic::All,
                filename: false,
                extension: false,
                size: false,
                modified: false,
                created: false,
                readonly: false,
                content: true,
                tolerance_millis: 0,
                name: NamePolicy::default(),
            },
            exclude_hardlinked_objects: false,
        };
        let exact = run(InspectionTask::Duplicates { options });
        let groups = store::groups(&catalog, exact.id.clone(), None).unwrap();
        assert_eq!(groups.groups.len(), 1);
        assert_eq!(groups.groups[0].evidence, DuplicateEvidence::ExactBytes);
        assert_eq!(
            store::group_members(&catalog, exact.id, groups.groups[0].id.clone(), None)
                .unwrap()
                .rows
                .len(),
            2
        );
        let compared = run(InspectionTask::Health {
            mode: HealthMode::CompareBaseline,
        });
        let rows = store::findings(&catalog, compared.id, None).unwrap().rows;
        assert!(
            rows.iter()
                .all(|row| row.checksum == ChecksumState::NoBaseline)
        );
        let target = rows
            .iter()
            .find(|row| row.name == "one.bin")
            .unwrap()
            .id
            .clone();
        assert!(
            store::latest_baseline(&catalog, target.clone())
                .unwrap()
                .is_none()
        );
        let established = run(InspectionTask::Health {
            mode: HealthMode::EstablishBaseline,
        });
        assert!(
            store::findings(&catalog, established.id, None)
                .unwrap()
                .rows
                .iter()
                .all(|row| row.checksum == ChecksumState::MatchesBaseline)
        );
        let original = store::latest_baseline(&catalog, target.clone())
            .unwrap()
            .unwrap();
        let path = source.join("one.bin");
        assert!(path.starts_with(owned.path()));
        std::fs::write(&path, vec![9; 100]).unwrap();
        let changed = run(InspectionTask::Health {
            mode: HealthMode::CompareBaseline,
        });
        let changed = store::findings(&catalog, changed.id, None).unwrap();
        let row = changed.rows.iter().find(|row| row.id == target).unwrap();
        assert_eq!(row.checksum, ChecksumState::ChangedSinceBaseline);
        assert_eq!(row.content, ContentState::NotChecked);
        assert_eq!(
            store::latest_baseline(&catalog, target.clone())
                .unwrap()
                .unwrap()
                .id,
            original.id
        );
        run(InspectionTask::Health {
            mode: HealthMode::EstablishBaseline,
        });
        let next = store::latest_baseline(&catalog, target).unwrap().unwrap();
        assert_eq!(next.previous_id, Some(original.id));
        assert_ne!(next.digest, original.digest);
        assert_eq!(std::fs::read(path).unwrap(), vec![9; 100]);
        assert_eq!(
            std::fs::read(source.join("different-name.dat")).unwrap(),
            vec![1; 100]
        );
        assert_eq!(store::jobs(&catalog).unwrap().len(), 6);
    }
}
