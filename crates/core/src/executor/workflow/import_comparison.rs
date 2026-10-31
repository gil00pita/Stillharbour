//! Bounded ephemeral read jobs. Their reports never authorize skipping a copy.
use super::*;
use crate::executor::fingerprints::{self, Entry, Stats};
use crate::filesystem::DirectoryChain;
use crate::jobs::{
    operation_locks::{Lease, Registry, Scope},
    paced_io::IoStep,
};
use rusqlite::{OptionalExtension, params};
use std::{collections::BTreeSet, fs::File};

const MAX_JOBS: usize = 8;
const MAX_SOURCES: usize = 512;
const MAX_CANDIDATES: usize = 4096;
const MAX_MATCHES: usize = 4;
fn active(state: ImportComparisonState) -> bool {
    matches!(
        state,
        ImportComparisonState::Queued
            | ImportComparisonState::Running
            | ImportComparisonState::CancelRequested
    )
}
struct Comparison {
    view: Mutex<ImportComparisonJob>,
    cancel: AtomicBool,
    discard: AtomicBool,
}
#[derive(Default)]
pub(super) struct Comparisons(Mutex<BTreeMap<String, Arc<Comparison>>>);
impl Comparisons {
    fn get(&self, id: &str) -> Result<Arc<Comparison>> {
        crate::operations::validate_id(id)?;
        self.0.lock().map_err(|_| failed("Import comparison jobs are unavailable."))?
            .get(id).cloned().ok_or_else(|| failed("This comparison is no longer retained. Compare again; reports do not survive restart."))
    }
    pub(super) fn root(&self, root: &str, cancel: bool) -> Result<bool> {
        let entries = self
            .0
            .lock()
            .map_err(|_| failed("Import comparison jobs are unavailable."))?;
        let mut found = false;
        for entry in entries.values() {
            let mut view = entry
                .view
                .lock()
                .map_err(|_| failed("Import comparison state is unavailable."))?;
            if active(view.state)
                && (view.source_root_id == root || view.scope_root_ids.iter().any(|id| id == root))
            {
                found = true;
                if cancel {
                    entry.cancel.store(true, Ordering::Release);
                    view.state = ImportComparisonState::CancelRequested;
                }
            }
        }
        Ok(found)
    }
}
struct Root {
    id: String,
    path: PathBuf,
    object: Object,
    volume: String,
    scan: Option<String>,
}
impl Root {
    fn validate(&self, catalog: &Catalog) -> Result<()> {
        let id = self.id.clone();
        let (path, object, volume, scan, availability) = catalog.call(move |db| {
            let (path, object, volume) = super::super::sources::root(db, &id, false)?;
            let (scan, availability): (Option<String>, String) = db.query_row(
                "SELECT last_scan,availability FROM roots WHERE id=?",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            Ok((path, object, volume, scan, availability))
        })?;
        if object != self.object
            || volume != self.volume
            || native::encode_path(&self.path) != path
            || scan != self.scan
            || availability != "online"
        {
            return Err(failed(
                "Comparison scope changed or is offline/incomplete. Reconnect or finish its scan, then compare again.",
            ));
        }
        let file = native::open_root(&self.path, None)?;
        if !self.object.matches(&file)? || native::volume_token(&file)? != self.volume {
            return Err(failed(
                "The comparison root is inaccessible or changed native identity.",
            ));
        }
        Ok(())
    }
}
struct Raw {
    id: String,
    location: Vec<u8>,
    native_file: String,
    revision: String,
    bytes: u64,
}
fn load_root(
    catalog: &Catalog,
    id: String,
    remaining: usize,
) -> Result<(Root, Vec<Raw>, u32, bool)> {
    catalog.call(move|db| {
        let (path,object,volume)=super::super::sources::root(db,&id,false)?;
        let (scan,availability):(Option<String>,String)=db.query_row("SELECT last_scan,availability FROM roots WHERE id=?",[&id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if availability!="online" {return Err(failed("A selected comparison root is offline, inaccessible or still scanning."));}
        let latest:Option<String>=db.query_row("SELECT state FROM scans WHERE root_id=? AND id NOT LIKE 'thumbnail:%' ORDER BY started_at DESC,id DESC LIMIT 1",[&id],|r|r.get(0)).optional()?;
        let complete_scan=scan.is_some() && latest.as_deref()==Some("completed");
        let unresolved:u32=db.query_row("SELECT count(*) FROM assets WHERE root_id=? AND presence NOT IN ('present','missing','excluded')",[&id],|r|r.get(0))?;
        let complete=complete_scan && unresolved==0;
        let excluded:u32=db.query_row("SELECT count(*) FROM assets WHERE root_id=? AND presence IN ('missing','excluded')",[&id],|r|r.get(0))?;
        let mut query=db.prepare("SELECT CASE WHEN length(CAST(id AS BLOB))<=128 THEN id END,CASE WHEN length(relative_native)<=8192 THEN relative_native END,CASE WHEN length(native_file)<=128 THEN native_file END,CASE WHEN length(revision)=64 THEN revision END,bytes FROM assets WHERE root_id=?1 AND presence='present' ORDER BY id LIMIT ?2")?;
        let mut rows=query.query(params![id,(remaining+1) as i64])?;
        let mut files=Vec::new();
        while let Some(row)=rows.next()? {
            let asset_id:Option<String>=row.get(0)?;
            let location:Option<Vec<u8>>=row.get(1)?;
            let identity:Option<String>=row.get(2)?;
            let revision:Option<String>=row.get(3)?;
            files.push(Raw{id:asset_id.ok_or_else(||failed("An indexed asset ID exceeds its bound."))?,location:location.ok_or_else(||failed("An indexed native locator exceeds its bound."))?,native_file:identity.ok_or_else(||failed("An indexed native identity exceeds its bound."))?,revision:revision.ok_or_else(||failed("An indexed file revision is invalid."))?,bytes:row.get::<_,i64>(4)?.try_into().map_err(|_|failed("An indexed file length is negative."))?});
            if files.len()>remaining {return Err(failed("Compare at most 4,096 indexed candidate files. Choose a smaller root scope."));}
        }
        Ok((Root{id,path:native::decode_path(&path)?,object,volume,scan},files,excluded,complete))
    })
}
fn entry(raw: Raw, root: &Root, file: &File) -> Result<Entry> {
    crate::operations::validate_id(&raw.id)?;
    if !raw.revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(failed("An indexed file revision is invalid."));
    }
    let (volume, inode) = raw
        .native_file
        .split_once(':')
        .ok_or_else(|| failed("An indexed native identity is invalid."))?;
    let object = Object {
        volume: volume
            .parse()
            .map_err(|_| failed("Invalid indexed volume identity."))?,
        file: inode
            .parse()
            .map_err(|_| failed("Invalid indexed file identity."))?,
    };
    if object.volume != root.object.volume {
        return Err(failed("An indexed file has an unexpected volume identity."));
    }
    let location =
        super::super::locations::Source::capture(file, &native::decode_path(&raw.location)?)?;
    let entry = Entry {
        id: raw.id,
        root_id: root.id.clone(),
        location,
        object,
        revision: raw.revision,
        bytes: raw.bytes,
        display: native::decode_path(&raw.location)?
            .to_string_lossy()
            .into_owned(),
    };
    entry.open(file)?;
    Ok(entry)
}
fn update(job: &Comparison, change: impl FnOnce(&mut ImportComparisonJob)) -> Result<()> {
    let mut view = job
        .view
        .lock()
        .map_err(|_| failed("Import comparison state is unavailable."))?;
    change(&mut view);
    Ok(())
}
fn short_display(mut text: String) -> (String, bool) {
    if text.len() <= 1024 {
        return (text, false);
    }
    let mut boundary = 1021;
    while !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text.truncate(boundary);
    text.push('…');
    (text, true)
}
fn issue(root: &str, asset: Option<String>, error: AppError) -> ImportComparisonIssue {
    ImportComparisonIssue {
        root_id: root.into(),
        asset_id: asset,
        error,
    }
}
struct ScopeData {
    roots: Vec<Root>,
    raw: Vec<Vec<Raw>>,
    lease: Lease,
    issues: Vec<ImportComparisonIssue>,
    excluded: u32,
}
fn admit(catalog: &Catalog, input: &ImportComparisonInput, source: &Snapshot) -> Result<ScopeData> {
    let mut roots = Vec::new();
    let mut raw = Vec::new();
    let mut scopes = Vec::new();
    let mut issues = Vec::new();
    let mut count = 0;
    let mut excluded = 0u32;
    for id in &input.scope_root_ids {
        match load_root(catalog, id.clone(), MAX_CANDIDATES - count) {
            Ok((root, files, omitted, complete)) => {
                // Identity checks precede scope leasing and are repeated after it.
                root.validate(catalog)?;
                if root.object == source.source {
                    return Err(failed(
                        "The selected source root cannot also be its own existing-library comparison scope.",
                    ));
                }
                let chain = DirectoryChain::open(&root.path.canonicalize()?)?;
                if !root.object.matches(chain.leaf())? {
                    return Err(failed("Comparison root changed before read admission."));
                }
                count += files.len();
                excluded = excluded
                    .checked_add(omitted)
                    .ok_or_else(|| failed("Indexed exclusion count overflow."))?;
                if !complete {
                    issues.push(issue(id,None,failed("This scope has an incomplete latest inventory or inaccessible/unresolved indexed files. Indexed matches may be shown, but absence is not established.")));
                }
                scopes.push(Scope::new(chain, Access::Read));
                roots.push(root);
                raw.push(files);
            }
            Err(error) => issues.push(issue(id, None, error)),
        }
    }
    // No valid root is an error, never an empty comparison scope.
    if scopes.is_empty() {
        return Err(issues.first().map(|issue|issue.error.clone()).unwrap_or_else(||failed("No selected comparison root can be admitted. Offline roots are not empty libraries.")));
    }
    let lease = Registry::shared().try_acquire(scopes)?;
    for root in &roots {
        root.validate(catalog)?;
    }
    Ok(ScopeData {
        roots,
        raw,
        lease,
        issues,
        excluded,
    })
}

fn run(
    workflow: &CopyWorkflow,
    input: &ImportComparisonInput,
    job: &Comparison,
    key_map: fn(&str) -> String,
) -> Result<IoStep<ImportComparisonReport>> {
    if job.cancel.load(Ordering::Acquire) {
        return Ok(IoStep::Canceled);
    }
    let source = Snapshot::build_read(
        &workflow.catalog,
        input.plan_id.clone(),
        input.manifest_id.clone(),
    )?;
    let source_scope = source.open_scope(&workflow.catalog, Access::Read)?;
    let source_root = source_scope.directory(0).leaf();
    let mut scope = admit(&workflow.catalog, input, &source)?;
    let io = PacedIo::shared();
    let mut stats = Stats::default();
    let sources = source
        .files
        .iter()
        .map(|file| {
            Ok(Entry {
                id: file.id.clone(),
                root_id: source.root_id.clone(),
                location: file.location.clone(),
                object: file.object.clone(),
                revision: file.revision.clone(),
                bytes: file.bytes,
                display: native::decode_path(&file.location.relative_native)?
                    .to_string_lossy()
                    .into_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if sources
        .iter()
        .any(|file| file.object.volume != source.source.volume)
    {
        return Err(failed(
            "Compare files from their actual registered volume root; nested volume crossings require separate scope review.",
        ));
    }
    let source_sizes = sources
        .iter()
        .map(|file| file.bytes)
        .collect::<BTreeSet<_>>();
    let mut candidates = Vec::new();
    for index in 0..scope.roots.len() {
        let root = &scope.roots[index];
        let file = scope.lease.directory(index).leaf();
        for raw in std::mem::take(&mut scope.raw[index]) {
            if job.cancel.load(Ordering::Acquire) {
                return Ok(IoStep::Canceled);
            }
            let id = raw.id.clone();
            match entry(raw, root, file) {
                Ok(entry) => candidates.push((index, entry)),
                Err(error) => scope.issues.push(issue(&root.id, Some(id), error)),
            }
        }
    }
    update(job, |view| {
        view.candidate_files = candidates.len() as u32;
        view.stage = ImportComparisonStage::Sampling;
    })?;
    let candidate_sizes = candidates
        .iter()
        .map(|(_, file)| file.bytes)
        .collect::<BTreeSet<_>>();
    let mut source_samples = Vec::new();
    let mut source_valid = Vec::new();
    let mut sample_keys = BTreeSet::new();
    for file in &sources {
        let result = if candidate_sizes.contains(&file.bytes) {
            fingerprints::sample(
                file,
                source_root,
                &io,
                &job.cancel,
                &mut stats,
                fingerprints::key,
            )
        } else {
            file.open(source_root)
                .map(|_| IoStep::Completed(String::new()))
        };
        update(job, |view| view.bytes_read = stats.bytes_read.to_string())?;
        match result {
            Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
            Ok(IoStep::Completed(sample)) => {
                let sample = if sample.is_empty() {
                    None
                } else {
                    Some(key_map(&sample))
                };
                if let Some(sample) = &sample {
                    sample_keys.insert((file.bytes, sample.clone()));
                }
                source_samples.push(sample);
                source_valid.push(true);
            }
            Err(error) => {
                scope.issues.push(issue(&source.root_id, None, error));
                source_samples.push(None);
                source_valid.push(false);
            }
        }
    }
    let mut sampled = Vec::new();
    for (index, (root_index, file)) in candidates.iter().enumerate() {
        if job.cancel.load(Ordering::Acquire) {
            return Ok(IoStep::Canceled);
        }
        if source_sizes.contains(&file.bytes) {
            let sampled_file = fingerprints::sample(
                file,
                scope.lease.directory(*root_index).leaf(),
                &io,
                &job.cancel,
                &mut stats,
                fingerprints::key,
            );
            update(job, |view| view.bytes_read = stats.bytes_read.to_string())?;
            match sampled_file {
                Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
                Ok(IoStep::Completed(sample)) => {
                    if sample_keys.contains(&(file.bytes, key_map(&sample))) {
                        sampled.push(index);
                    }
                }
                Err(error) => scope
                    .issues
                    .push(issue(&file.root_id, Some(file.id.clone()), error)),
            }
        }
        update(job, |view| {
            view.processed_candidates = (index + 1) as u32;
            view.bytes_read = stats.bytes_read.to_string();
        })?;
    }
    update(job, |view| {
        view.stage = ImportComparisonStage::Hashing;
        view.processed_candidates = 0;
    })?;
    let mut full_candidates: BTreeMap<(u64, String), Vec<usize>> = BTreeMap::new();
    for (processed, index) in sampled.iter().enumerate() {
        let (root_index, file) = &candidates[*index];
        let hashed = fingerprints::full(
            file,
            scope.lease.directory(*root_index).leaf(),
            &io,
            &job.cancel,
            &mut stats,
            fingerprints::key,
        );
        update(job, |view| view.bytes_read = stats.bytes_read.to_string())?;
        match hashed {
            Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
            Ok(IoStep::Completed(hash)) => full_candidates
                .entry((file.bytes, key_map(&hash)))
                .or_default()
                .push(*index),
            Err(error) => scope
                .issues
                .push(issue(&file.root_id, Some(file.id.clone()), error)),
        }
        update(job, |view| {
            view.processed_candidates = (processed + 1) as u32;
            view.bytes_read = stats.bytes_read.to_string();
        })?;
    }
    update(job, |view| {
        view.stage = ImportComparisonStage::Comparing;
    })?;
    let mut files = Vec::new();
    let mut verified = Vec::new();
    for (index, file) in sources.iter().enumerate() {
        if job.cancel.load(Ordering::Acquire) {
            return Ok(IoStep::Canceled);
        }
        source.validate_read_roots(&workflow.catalog)?;
        let mut matches = Vec::new();
        let mut further = false;
        if source_valid[index] && source_samples[index].is_some() && !full_candidates.is_empty() {
            let hashed = fingerprints::full(
                file,
                source_root,
                &io,
                &job.cancel,
                &mut stats,
                fingerprints::key,
            );
            update(job, |view| view.bytes_read = stats.bytes_read.to_string())?;
            match hashed {
                Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
                Err(error) => {
                    source_valid[index] = false;
                    scope.issues.push(issue(&source.root_id, None, error));
                }
                Ok(IoStep::Completed(actual_source_hash)) => {
                    if let Some(indices) =
                        full_candidates.get(&(file.bytes, key_map(&actual_source_hash)))
                    {
                        for (position, candidate_index) in indices.iter().enumerate() {
                            let (root_index, candidate) = &candidates[*candidate_index];
                            scope.roots[*root_index].validate(&workflow.catalog)?;
                            let compared = fingerprints::equal(
                                (file, source_root),
                                (candidate, scope.lease.directory(*root_index).leaf()),
                                &io,
                                &job.cancel,
                                &mut stats,
                            );
                            update(job, |view| view.bytes_read = stats.bytes_read.to_string())?;
                            match compared {
                                Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
                                Err(error) => scope.issues.push(issue(
                                    &candidate.root_id,
                                    Some(candidate.id.clone()),
                                    error,
                                )),
                                Ok(IoStep::Completed(None)) => {}
                                Ok(IoStep::Completed(Some(witness))) => {
                                    if witness.digest != actual_source_hash {
                                        return Err(failed(
                                            "Source bytes changed after fingerprinting. Compare again.",
                                        ));
                                    }
                                    let (relative_path, display_truncated) =
                                        short_display(candidate.display.clone());
                                    matches.push(ImportMatch {
                                        asset_id: candidate.id.clone(),
                                        root_id: candidate.root_id.clone(),
                                        relative_path,
                                        display_truncated,
                                        digest: witness.digest,
                                        same_native_object: witness.same_object
                                            && scope.roots[*root_index].volume
                                                == source.source_volume,
                                    });
                                    verified.push((index, *candidate_index));
                                }
                            }
                            if matches.len() == MAX_MATCHES {
                                further = position + 1 < indices.len();
                                break;
                            }
                        }
                    }
                }
            }
        }
        let state = if !matches.is_empty() {
            if matches.iter().all(|found| found.same_native_object) {
                ImportFileComparison::SameNativeObject
            } else {
                ImportFileComparison::ExactBytesCompared
            }
        } else if !source_valid[index] || !scope.issues.is_empty() {
            ImportFileComparison::NeedsAttention
        } else {
            ImportFileComparison::NoIndexedMatch
        };
        files.push(ImportComparisonFile {
            file_id: file.id.clone(),
            source_path: file.display.clone(),
            bytes: file.bytes.to_string(),
            state,
            matches,
            further_candidates_uncompared: further,
        });
        update(job, |view| {
            view.compared_source_files = (index + 1) as u32;
            view.bytes_read = stats.bytes_read.to_string();
        })?;
    }
    // A late error also invalidates earlier absence claims. Positive byte
    // observations remain labeled separately and are revalidated below.
    if !scope.issues.is_empty() {
        for file in &mut files {
            if file.state == ImportFileComparison::NoIndexedMatch {
                file.state = ImportFileComparison::NeedsAttention;
            }
        }
    }
    source.validate_read_roots(&workflow.catalog)?;
    for root in &scope.roots {
        root.validate(&workflow.catalog)?;
    }
    for (source_index, candidate_index) in verified {
        sources[source_index].open(source_root)?;
        let (root_index, candidate) = &candidates[candidate_index];
        candidate.open(scope.lease.directory(*root_index).leaf())?;
    }
    for (index, file) in sources.iter().enumerate() {
        if source_valid[index] {
            file.open(source_root)?;
        }
    }
    Ok(IoStep::Completed(ImportComparisonReport {
        policy: "indexed-import-comparison-v1".into(),
        sample_policy: fingerprints::SAMPLE_POLICY.into(),
        algorithm: "blake3".into(),
        observed_at: crate::library::store::now(),
        indexed_present_files_only: true,
        all_scope_available: scope.issues.is_empty(),
        excluded_or_missing_files: scope.excluded,
        bundle_review_required: true,
        mutation_authorized: false,
        files,
        issues: scope.issues,
    }))
}

struct Guard {
    workflow: Arc<CopyWorkflow>,
    id: String,
    job: Arc<Comparison>,
}
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = PacedIo::shared().set_paused(false);
        if self.job.discard.load(Ordering::Acquire)
            && let Ok(mut entries) = self.workflow.comparisons.0.lock()
        {
            entries.remove(&self.id);
        }
    }
}
impl CopyWorkflow {
    pub(crate) fn start_import_comparison(
        self: &Arc<Self>,
        library: &Library,
        mut input: ImportComparisonInput,
    ) -> Result<ImportComparisonJob> {
        library.ensure_work_admission()?;
        crate::operations::validate_id(&input.plan_id)?;
        crate::operations::validate_id(&input.manifest_id)?;
        if input.scope_root_ids.is_empty() || input.scope_root_ids.len() > 16 {
            return Err(failed(
                "Choose between one and sixteen indexed comparison roots.",
            ));
        }
        for root in &input.scope_root_ids {
            crate::operations::validate_id(root)?;
        }
        input.scope_root_ids.sort();
        if input
            .scope_root_ids
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        {
            return Err(failed("Choose each comparison root once."));
        }
        let manifest = library
            .organization_copy_manifest_open(input.plan_id.clone(), input.manifest_id.clone())?;
        if !manifest.issues.is_empty()
            || manifest.files.is_empty()
            || manifest.files.len() > MAX_SOURCES
        {
            return Err(failed(
                "Compare a conflict-free selection of at most 512 confirmed source files.",
            ));
        }
        let root = manifest.files[0].root_id.clone();
        if manifest.files.iter().any(|file| file.root_id != root)
            || input.scope_root_ids.contains(&root)
        {
            return Err(failed(
                "Choose an existing-library scope separate from the single selected source root.",
            ));
        }
        let permit = self.gate.try_start()?;
        let id = uuid::Uuid::new_v4().to_string();
        let view = ImportComparisonJob {
            id: id.clone(),
            plan_id: input.plan_id.clone(),
            manifest_id: input.manifest_id.clone(),
            source_root_id: root,
            scope_root_ids: input.scope_root_ids.clone(),
            state: ImportComparisonState::Queued,
            stage: ImportComparisonStage::Admitting,
            candidate_files: 0,
            processed_candidates: 0,
            compared_source_files: 0,
            total_source_files: manifest.files.len() as u32,
            bytes_read: "0".into(),
            report: None,
            error: None,
        };
        let job = Arc::new(Comparison {
            view: Mutex::new(view.clone()),
            cancel: AtomicBool::new(false),
            discard: AtomicBool::new(false),
        });
        {
            let mut entries = self
                .comparisons
                .0
                .lock()
                .map_err(|_| failed("Import comparisons are unavailable."))?;
            if entries.len() >= MAX_JOBS {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "Close retained comparison reports before starting another (limit eight).",
                ));
            }
            entries.insert(id.clone(), job.clone());
        }
        if let Err(error) = library.ensure_work_admission() {
            self.comparisons
                .0
                .lock()
                .map_err(|_| failed("Import comparisons are unavailable."))?
                .remove(&id);
            return Err(error);
        }
        let workflow = self.clone();
        let active_job = job.clone();
        let job_id = id.clone();
        let spawned=std::thread::Builder::new().name("stillharbour-import-comparison".into()).spawn(move|| {
            let guard=Guard{workflow:workflow.clone(),id:job_id,job:active_job.clone()};
            let outcome=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                update(&active_job,|view|view.state=if active_job.cancel.load(Ordering::Acquire){ImportComparisonState::CancelRequested}else{ImportComparisonState::Running})?;
                run(&workflow,&input,&active_job,str::to_owned)
            })).unwrap_or_else(|_|Err(AppError::new(ErrorCode::WorkerFailed,"The read-only comparison worker stopped unexpectedly. No mutation was authorized.")));
            // Publish terminal state only after native leases, byte-pause
            // ownership and the worker admission permit have been released.
            drop(guard);drop(permit);
            let _=update(&active_job,|view| {
                view.stage=ImportComparisonStage::Finished;
                match outcome {
                    Ok(IoStep::Completed(report)) if !active_job.cancel.load(Ordering::Acquire)=>{view.state=if report.all_scope_available {ImportComparisonState::Completed}else{ImportComparisonState::CompletedWithIssues};view.report=Some(report);}
                    Ok(_)=>view.state=ImportComparisonState::Canceled,
                    Err(error)=>{view.state=if active_job.cancel.load(Ordering::Acquire){ImportComparisonState::Canceled}else{ImportComparisonState::Failed};view.error=Some(error);}
                }
            });
        });
        if let Err(error) = spawned {
            update(&job, |view| {
                view.state = ImportComparisonState::Failed;
                view.stage = ImportComparisonStage::Finished;
                view.error = Some(
                    AppError::new(
                        ErrorCode::WorkerFailed,
                        "Could not start the read-only comparison worker.",
                    )
                    .caused_by(error),
                );
            })?;
        }
        Ok(view)
    }
    pub(crate) fn import_comparison_job(&self, id: String) -> Result<ImportComparisonJob> {
        let job = self.comparisons.get(&id)?;
        let view = job
            .view
            .lock()
            .map_err(|_| failed("Import comparison state is unavailable."))?
            .clone();
        Ok(view)
    }
    pub(crate) fn cancel_import_comparison(&self, id: String) -> Result<ImportComparisonJob> {
        let job = self.comparisons.get(&id)?;
        job.cancel.store(true, Ordering::Release);
        update(&job, |view| {
            if active(view.state) {
                view.state = ImportComparisonState::CancelRequested;
            }
        })?;
        self.import_comparison_job(id)
    }
    pub(crate) fn discard_import_comparison(&self, id: String) -> Result<()> {
        let job = self.comparisons.get(&id)?;
        job.discard.store(true, Ordering::Release);
        job.cancel.store(true, Ordering::Release);
        let running = active(
            job.view
                .lock()
                .map_err(|_| failed("Import comparison state is unavailable."))?
                .state,
        );
        if !running {
            self.comparisons
                .0
                .lock()
                .map_err(|_| failed("Import comparisons are unavailable."))?
                .remove(&id);
        }
        Ok(())
    }
}

#[cfg(test)]
pub(in crate::executor) fn compare_fixture(
    workflow: &CopyWorkflow,
    input: ImportComparisonInput,
    collisions: bool,
) -> Result<IoStep<ImportComparisonReport>> {
    let snapshot = Snapshot::build_read(
        &workflow.catalog,
        input.plan_id.clone(),
        input.manifest_id.clone(),
    )?;
    let job = Comparison {
        view: Mutex::new(ImportComparisonJob {
            id: uuid::Uuid::new_v4().to_string(),
            plan_id: input.plan_id.clone(),
            manifest_id: input.manifest_id.clone(),
            source_root_id: snapshot.root_id,
            scope_root_ids: input.scope_root_ids.clone(),
            state: ImportComparisonState::Running,
            stage: ImportComparisonStage::Admitting,
            candidate_files: 0,
            processed_candidates: 0,
            compared_source_files: 0,
            total_source_files: snapshot.files.len() as u32,
            bytes_read: "0".into(),
            report: None,
            error: None,
        }),
        cancel: AtomicBool::new(false),
        discard: AtomicBool::new(false),
    };
    run(
        workflow,
        &input,
        &job,
        if collisions {
            |_| "0".repeat(64)
        } else {
            str::to_owned
        },
    )
}
