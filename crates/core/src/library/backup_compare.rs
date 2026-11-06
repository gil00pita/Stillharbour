//! Read-only M4A adapter. Session-only reviews have no write/approval commands.
use super::{
    Library,
    inspection_inventory::{child, each},
    native, store,
};
use crate::{
    backup::comparison::*,
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, DirectoryChain, FileIdentity},
    jobs::{
        operation_locks::{Access, Registry, Scope},
        paced_io::{CHUNK_BYTES, IoStep, PacedIo},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

const MAX_ENTRIES: usize = 10_000;
const MAX_NAMES: usize = 16 * 1024 * 1024;
const MAX_ISSUES: usize = 200;

mod append;
pub(super) mod history;
mod inspector;
pub(super) mod profiles;
mod reports;

fn same_directory(a: &FileIdentity, b: &FileIdentity) -> bool {
    a.volume == b.volume && a.file == b.file
}

fn error(code: ErrorCode, message: &str) -> AppError {
    AppError::new(code, message)
}
fn changed() -> AppError {
    error(
        ErrorCode::Conflict,
        "The comparison scope changed. Choose the destination again and recompare.",
    )
}
fn unavailable() -> AppError {
    error(
        ErrorCode::InvalidInput,
        "This session comparison is unavailable. Start a fresh comparison.",
    )
}
fn canceled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(error(
            ErrorCode::Busy,
            "Comparison canceled; both trees are unchanged.",
        ))
    } else {
        Ok(())
    }
}

#[derive(Clone)]
struct Review {
    append_scope: Option<append::FrozenScope>,
    observations: BTreeMap<String, (PathBuf, Option<Snapshot>, Option<Snapshot>)>,
    source: Option<Binding>,
    destination: Option<Binding>,
    run: ComparisonRun,
    rows: Vec<ComparisonEntry>,
    cancel: Arc<AtomicBool>,
}
#[derive(Default)]
pub(super) struct CompareJobs {
    preview_gate: Mutex<()>,
    review: Mutex<Option<Review>>,
}
impl CompareJobs {
    pub(super) fn idle(&self) -> Result<bool> {
        let review = self.review.lock().map_err(|_| changed())?;
        if review
            .as_ref()
            .is_some_and(|r| r.run.state == ComparisonState::Comparing)
        {
            return Ok(false);
        }
        match self.preview_gate.try_lock() {
            Ok(_) => Ok(true),
            Err(std::sync::TryLockError::WouldBlock) => Ok(false),
            Err(_) => Err(changed()),
        }
    }
    pub(super) fn stop(&self) {
        if let Ok(state) = self.review.lock()
            && let Some(review) = state.as_ref()
        {
            review.cancel.store(true, Ordering::Release);
        }
    }
}

struct ComparisonScope {
    base: store::RootRecord,
    source: Binding,
    destination: Binding,
    catalog_id: String,
    namespace: PathBuf,
    lease: crate::jobs::operation_locks::Lease,
}

#[derive(Clone)]
struct Binding {
    path: PathBuf,
    identity: FileIdentity,
    volume: String,
}
impl Binding {
    fn optional_directory(&self, relative: &Path) -> Result<Option<File>> {
        let mut file = self.open()?;
        for part in relative.components() {
            #[cfg(windows)]
            if filesystem::windows::destination_entry(&file, part.as_os_str())?.is_none() {
                return Ok(None);
            }
            match child(&file, part.as_os_str(), true) {
                Ok(next) => file = next,
                Err(e) if e.io_kind == Some(std::io::ErrorKind::NotFound) => return Ok(None),
                Err(e) => return Err(e),
            }
        }
        Ok(Some(file))
    }
    fn open(&self) -> Result<File> {
        let file = native::open_root(&self.path, Some(&self.identity))?;
        if native::volume_token(&file)? != self.volume {
            return Err(changed());
        }
        Ok(file)
    }
    fn directory(&self, relative: &Path) -> Result<File> {
        let mut file = self.open()?;
        for part in relative.components() {
            file = child(&file, part.as_os_str(), true)?;
        }
        Ok(file)
    }
    fn file(&self, relative: &Path) -> Result<File> {
        native::open_asset(&self.path, &self.identity, &self.volume, relative)
    }
}
#[derive(Clone)]
struct Snapshot {
    bytes: u64,
    modified: Option<String>,
    revision: String,
    identity: FileIdentity,
}
impl Snapshot {
    fn capture(file: &File) -> Result<Self> {
        let m = file.metadata()?;
        if !m.is_file() {
            return Err(error(
                ErrorCode::Unsupported,
                "Only ordinary regular-file content is compared.",
            ));
        }
        Ok(Self {
            bytes: m.len(),
            modified: native::unix_time(m.modified().ok()),
            revision: native::revision(file)?,
            identity: filesystem::identity(file)?,
        })
    }
    fn validate(&self, file: &File) -> Result<()> {
        if self.identity != filesystem::identity(file)? || self.revision != native::revision(file)?
        {
            return Err(changed());
        }
        Ok(())
    }
}
#[derive(Default)]
struct Inventory {
    objects: BTreeMap<PathBuf, ComparisonEntryKind>,
    files: BTreeMap<PathBuf, Snapshot>,
    directories: Vec<(PathBuf, String)>,
    issues: Vec<SyncIssue>,
    incomplete: bool,
    visited: usize,
    names: usize,
}
impl Inventory {
    fn issue(&mut self, path: &Path, message: &str) {
        self.incomplete = true;
        if self.issues.len() < MAX_ISSUES {
            self.issues.push(SyncIssue {
                side: None,
                display_path: path.to_string_lossy().into_owned(),
                message: message.into(),
            });
        }
    }
    fn walk(
        &mut self,
        root: &Binding,
        relative: &Path,
        cancel: &AtomicBool,
        depth: usize,
    ) -> Result<()> {
        canceled(cancel)?;
        if depth > 64 {
            self.issue(
                relative,
                "Directory depth limit reached; inventory is incomplete.",
            );
            return Ok(());
        }
        let result = (|| -> Result<()> {
            let parent = root.directory(relative)?;
            let revision = native::revision(&parent)?;
            each(&parent, |entry| {
                canceled(cancel)?;
                self.visited += 1;
                let path = relative.join(&entry.name);
                self.names = self
                    .names
                    .saturating_add(native::encode_path(&path).len().saturating_mul(6))
                    .saturating_add(512);
                if self.visited > MAX_ENTRIES || self.names > MAX_NAMES {
                    return Err(error(
                        ErrorCode::Unsupported,
                        "Comparison reached its 10,000-entry or 16 MiB name-memory bound; inventory is incomplete.",
                    ));
                }
                self.objects.insert(
                    path.clone(),
                    if entry.blocked {
                        ComparisonEntryKind::Unsupported
                    } else if entry.directory {
                        ComparisonEntryKind::Directory
                    } else {
                        ComparisonEntryKind::File
                    },
                );
                if entry.blocked {
                    self.issue(&path, "Link, special object or cloud placeholder is excluded; it has not been read or protected.");
                } else if entry.directory {
                    self.walk(root, &path, cancel, depth + 1)?;
                } else {
                    match root.file(&path).and_then(|file| Snapshot::capture(&file)) {
                        Ok(snapshot) => {
                            self.files.insert(path, snapshot);
                        }
                        Err(e) => self.issue(&path, &e.message),
                    }
                }
                Ok(())
            })?;
            if revision != native::revision(&root.directory(relative)?)? {
                return Err(changed());
            }
            self.directories.push((relative.to_owned(), revision));
            Ok(())
        })();
        if let Err(e) = result {
            canceled(cancel)?;
            self.issue(relative, &e.message);
        }
        Ok(())
    }
    fn validate(&self, root: &Binding) -> Result<()> {
        for (path, version) in &self.directories {
            if native::revision(&root.directory(path)?)? != *version {
                return Err(changed());
            }
        }
        for (path, snapshot) in &self.files {
            snapshot.validate(&root.file(path)?)?;
        }
        Ok(())
    }
}

/// Fill separately: Read::read may return different short reads for equal files.
fn fill(
    file: &mut impl std::io::Read,
    buffer: &mut [u8],
    cancel: &AtomicBool,
    count: &mut u64,
) -> Result<usize> {
    let mut used = 0;
    while used < buffer.len() {
        match PacedIo::shared().read(file, &mut buffer[used..], cancel)? {
            IoStep::Canceled => return Err(error(ErrorCode::Busy, "Comparison canceled.")),
            IoStep::Completed(0) => break,
            IoStep::Completed(n) => {
                used += n;
                *count = count.saturating_add(n as u64);
            }
        }
    }
    Ok(used)
}
fn equal(
    source: &Binding,
    destination: &Binding,
    path: &Path,
    a: &Snapshot,
    b: &Snapshot,
    cancel: &AtomicBool,
    count: &mut u64,
) -> Result<bool> {
    let mut left = source.file(path)?;
    let mut right = destination.file(path)?;
    a.validate(&left)?;
    b.validate(&right)?;
    let mut x = [0; CHUNK_BYTES];
    let mut y = [0; CHUNK_BYTES];
    let mut matches = a.bytes == b.bytes;
    if matches {
        loop {
            let n = fill(&mut left, &mut x, cancel, count)?;
            let m = fill(&mut right, &mut y, cancel, count)?;
            if n != m || x[..n] != y[..m] {
                matches = false;
                break;
            }
            if n == 0 {
                break;
            }
        }
    }
    a.validate(&left)?;
    b.validate(&right)?;
    a.validate(&source.file(path)?)?;
    b.validate(&destination.file(path)?)?;
    Ok(matches)
}

fn content_digest(
    root: &Binding,
    path: &Path,
    snapshot: &Snapshot,
    cancel: &AtomicBool,
    count: &mut u64,
) -> Result<String> {
    let mut file = root.file(path)?;
    snapshot.validate(&file)?;
    let mut hash = blake3::Hasher::new();
    let mut buffer = [0; CHUNK_BYTES];
    loop {
        let n = fill(&mut file, &mut buffer, cancel, count)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    snapshot.validate(&file)?;
    snapshot.validate(&root.file(path)?)?;
    Ok(hash.finalize().to_hex().to_string())
}

fn report_page(
    id: String,
    offset: u32,
    category: Option<ComparisonCategory>,
    rows: &[ComparisonEntry],
) -> Result<ComparisonPage> {
    let start = offset as usize;
    let matches = |row: &&ComparisonEntry| category.is_none_or(|c| c == row.category);
    let total = rows.iter().filter(matches).count();
    if start > total {
        return Err(unavailable());
    }
    let end = (start + 200).min(total);
    Ok(ComparisonPage {
        total: Some(total as u32),
        run_id: id,
        entries: rows
            .iter()
            .filter(matches)
            .skip(start)
            .take(200)
            .cloned()
            .collect(),
        next_offset: (end < total).then_some(end as u32),
    })
}

impl Library {
    pub fn backup_comparison_history(&self) -> Result<Vec<ComparisonRun>> {
        reports::list(&self.catalog)
    }
    pub fn backup_comparison_latest(&self) -> Result<Option<ComparisonRun>> {
        Ok(self
            .comparisons
            .review
            .lock()
            .map_err(|_| unavailable())?
            .as_ref()
            .map(|r| r.run.clone()))
    }
    pub fn backup_compare_start(
        self: &Arc<Self>,
        selection: PathBuf,
        expected: (FileIdentity, String),
        mode: CompareMode,
    ) -> Result<ComparisonRun> {
        self.start_comparison(selection, expected, mode, None)
    }
    fn start_comparison(
        self: &Arc<Self>,
        selection: PathBuf,
        expected: (FileIdentity, String),
        mode: CompareMode,
        profile: Option<profiles::StoredProfile>,
    ) -> Result<ComparisonRun> {
        let mut current = self
            .comparisons
            .review
            .try_lock()
            .map_err(|_| error(ErrorCode::Busy, "Comparison admission is busy."))?;
        if current
            .as_ref()
            .is_some_and(|r| r.run.state == ComparisonState::Comparing)
        {
            return Err(error(
                ErrorCode::Busy,
                "One comparison is already running. Cancel it or wait.",
            ));
        }
        let scope = self.comparison_scope(
            selection,
            expected,
            profile.as_ref().map(|p| p.source_root_id.as_str()),
        )?;
        if let Some(profile) = &profile {
            profile.validate_source(&scope.base)?;
        }
        let ComparisonScope {
            base,
            source,
            destination,
            catalog_id,
            namespace,
            lease,
        } = scope;
        let run = ComparisonRun {
            review_plan: None,
            baseline_id: None,
            history_message: "No historical byte observation has been loaded.".into(),
            summary: None,
            profile_id: profile.as_ref().map(|p| p.id.clone()),
            id: uuid::Uuid::new_v4().to_string(), policy_version: 3, catalog_id,
            source_root_id: base.view.id.clone(), source_label: base.view.label.clone(), source_display_path: base.view.display_path.clone(),
            destination_display_path: destination.path.to_string_lossy().into_owned(), dataset_display_path: destination.path.join(&namespace).to_string_lossy().into_owned(),
            mode, state: ComparisonState::Comparing, started_at: chrono::Utc::now().to_rfc3339(), bytes_read: "0".into(), entries: 0, issues: vec![],
            message: "Comparing all regular files, including hidden and unrecognised files. No changes will be made.".into(), plan_digest: None, capabilities: DestinationCapability::default(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        if let Some(previous) = current.as_ref() {
            previous.cancel.store(true, Ordering::Release);
        }
        *current = Some(Review {
            append_scope: None,
            observations: BTreeMap::new(),
            source: None,
            destination: None,
            run: run.clone(),
            rows: vec![],
            cancel: cancel.clone(),
        });
        let owner = self.clone();
        let id = run.id.clone();
        let spawn = std::thread::Builder::new()
            .name("backup-compare".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    owner.backup_compare_run(&id, source, destination, namespace, mode, &cancel)
                }));
                // A terminal state must never outlive the worker's scope lease.
                drop(lease);
                if let Ok(mut state) = owner.comparisons.review.lock()
                    && let Some(review) = state.as_mut().filter(|r| r.run.id == id)
                {
                    let outcome = result.unwrap_or_else(|_| {
                        Err(error(
                            ErrorCode::WorkerFailed,
                            "The comparison worker stopped unexpectedly.",
                        ))
                    });
                    match outcome {
                        Ok(completed) => review.run.state = completed,
                        Err(e) => {
                            review.run.state = if cancel.load(Ordering::Acquire) {
                                ComparisonState::Canceled
                            } else {
                                ComparisonState::Failed
                            };
                            review.run.message = e.message;
                            review.run.plan_digest = None;
                        }
                    }
                    match reports::save(&owner.catalog, &review.run, &review.rows) {
                        Ok(message) => review.run.history_message.push_str(&format!(" {message}")),
                        Err(error) => review
                            .run
                            .history_message
                            .push_str(&format!(" Scan report was not saved: {}", error.message)),
                    }
                }
            });
        if let Err(e) = spawn {
            *current = None;
            return Err(error(
                ErrorCode::WorkerFailed,
                "The comparison worker could not start.",
            )
            .caused_by(e));
        }
        Ok(run)
    }
    fn comparison_scope(
        &self,
        selection: PathBuf,
        expected: (FileIdentity, String),
        source_root_id: Option<&str>,
    ) -> Result<ComparisonScope> {
        let roots = store::roots(&self.catalog)?;
        let base = roots
            .iter()
            .find(|r| source_root_id.map_or(r.view.is_base, |id| r.view.id == id))
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidInput,
                    "Configure a base library before comparing.",
                )
            })?;
        super::original_jobs::require_reconciled_root(&self.catalog, base.view.id.clone())?;
        if base.view.availability != "online" {
            return Err(error(
                ErrorCode::Io,
                "The base library is unavailable; it is not an empty source.",
            ));
        }
        let source = Binding {
            path: base.path.clone(),
            identity: base.identity.clone(),
            volume: base.volume.clone(),
        };
        source.open()?;
        let destination = Binding {
            path: selection,
            identity: expected.0,
            volume: expected.1,
        };
        destination.open()?;
        let target_chain = DirectoryChain::open(&destination.path.canonicalize()?)?;
        if !same_directory(target_chain.identity(), &destination.identity) {
            return Err(changed());
        }
        let data = DirectoryChain::open(&self.options.data_directory.canonicalize()?)?;
        if target_chain.overlaps(&data) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Destination must be separate from application data.",
            ));
        }
        let mut scopes = vec![Scope::new(target_chain, Access::Read)];
        for root in &roots {
            // An inaccessible registered root cannot establish safe nonoverlap.
            let file = native::open_root(&root.path, Some(&root.identity))?;
            if native::volume_token(&file)? != root.volume {
                return Err(changed());
            }
            let chain = DirectoryChain::open(&root.path.canonicalize()?)?;
            if !same_directory(chain.identity(), &root.identity) {
                return Err(changed());
            }
            let target = DirectoryChain::open(&destination.path.canonicalize()?)?;
            if target.overlaps(&chain) {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Choose a destination outside every library root and its ancestors.",
                ));
            }
            if root.view.id == base.view.id {
                if chain.overlaps(&data) {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Comparison source must be separate from application data.",
                    ));
                }
                scopes.push(Scope::new(chain, Access::Read));
            }
        }
        let lease = Registry::shared().try_acquire(scopes)?;
        let catalog_id = self.catalog.summary()?.id;
        let namespace = PathBuf::from(".stillharbour-copy")
            .join(&catalog_id)
            .join(&base.view.id);
        Ok(ComparisonScope {
            base: base.clone(),
            source,
            destination,
            catalog_id,
            namespace,
            lease,
        })
    }
    pub fn backup_comparison(&self, id: String) -> Result<ComparisonRun> {
        self.comparisons
            .review
            .lock()
            .map_err(|_| unavailable())?
            .as_ref()
            .filter(|r| r.run.id == id)
            .map(|r| r.run.clone())
            .ok_or_else(unavailable)
    }
    pub fn backup_compare_cancel(&self, id: String) -> Result<ComparisonRun> {
        let state = self.comparisons.review.lock().map_err(|_| unavailable())?;
        let review = state
            .as_ref()
            .filter(|r| r.run.id == id)
            .ok_or_else(unavailable)?;
        review.cancel.store(true, Ordering::Release);
        Ok(review.run.clone())
    }
    pub fn backup_comparison_page(&self, id: String, offset: u32) -> Result<ComparisonPage> {
        self.backup_comparison_page_filtered(id, offset, None)
    }
    pub fn backup_comparison_page_filtered(
        &self,
        id: String,
        offset: u32,
        category: Option<ComparisonCategory>,
    ) -> Result<ComparisonPage> {
        crate::operations::validate_id(&id)?;
        let state = self.comparisons.review.lock().map_err(|_| unavailable())?;
        if let Some(review) = state.as_ref().filter(|r| r.run.id == id) {
            return report_page(id, offset, category, &review.rows);
        }
        drop(state);
        let (_, rows) = reports::load(&self.catalog, &id)?;
        report_page(id, offset, category, &rows)
    }
    fn backup_compare_run(
        &self,
        id: &str,
        source: Binding,
        destination: Binding,
        namespace: PathBuf,
        mode: CompareMode,
        cancel: &AtomicBool,
    ) -> Result<ComparisonState> {
        let mut src = Inventory::default();
        src.walk(&source, Path::new(""), cancel, 0)?;
        let mut dst = Inventory::default();
        // A missing dataset is a proposed new namespace, never an empty device.
        let dataset = match destination.optional_directory(&namespace)? {
            Some(file) => Some(Binding {
                path: destination.path.join(&namespace),
                identity: filesystem::identity(&file)?,
                volume: native::volume_token(&file)?,
            }),
            None => None,
        };
        if let Some(root) = &dataset {
            dst.walk(root, Path::new(""), cancel, 0)?;
        }
        let complete = !src.incomplete && !dst.incomplete;
        let history_scope = history::scope(&source, &destination, &namespace)?;
        let prior = history::load(&self.catalog, &history_scope)?;
        let mut observed = BTreeMap::new();
        let mut observations = BTreeMap::new();
        let blocked = crate::backup::names::blocks(&src.objects, &dst.objects);
        let names: BTreeSet<_> = src
            .objects
            .iter()
            .chain(dst.objects.iter())
            .filter(|(path, kind)| {
                **kind != ComparisonEntryKind::Directory || blocked.contains_key(*path)
            })
            .map(|(path, _)| path.clone())
            .collect();
        let mut rows = Vec::new();
        let mut count = 0;
        let mut comparison_issues = Vec::new();
        let mut digest = blake3::Hasher::new();
        digest.update(b"stillharbour-m4a-review-v3");
        digest.update(&native::encode_path(&source.path));
        digest.update(&native::encode_path(&destination.path));
        digest.update(
            serde_json::to_string(&source.identity)
                .map_err(|_| changed())?
                .as_bytes(),
        );
        digest.update(
            serde_json::to_string(&destination.identity)
                .map_err(|_| changed())?
                .as_bytes(),
        );
        digest.update(&[mode as u8]);
        for path in names {
            canceled(cancel)?;
            let a = src.files.get(&path);
            let b = dst.files.get(&path);
            let block = blocked.get(&path);
            let missing_snapshot = (src.objects.get(&path) == Some(&ComparisonEntryKind::File)
                && a.is_none())
                || (dst.objects.get(&path) == Some(&ComparisonEntryKind::File) && b.is_none());
            let comparison =
                if block.is_none() && !missing_snapshot && mode == CompareMode::Verified {
                    match (a, b, &dataset) {
                        (Some(a), Some(b), Some(root)) => {
                            Some(equal(&source, root, &path, a, b, cancel, &mut count))
                        }
                        _ => None,
                    }
                } else {
                    None
                };
            let quick = mode == CompareMode::Quick
                && a.zip(b).is_some_and(|(a, b)| {
                    a.bytes == b.bytes && a.modified.is_some() && a.modified == b.modified
                });
            let (mut category, mut proposal, mut evidence) = if block.is_some() || missing_snapshot
            {
                (ComparisonCategory::Blocked, SyncProposal::Blocked, block.map_or("A regular file could not be captured. Absence and content equality are unproven; retain both trees.", |reason| reason.message()).into())
            } else {
                match comparison {
                    Some(Err(e)) => {
                        if comparison_issues.len() < MAX_ISSUES {
                            comparison_issues.push(SyncIssue {
                                side: Some(ComparisonSide::Comparison),
                                display_path: path.to_string_lossy().into_owned(),
                                message: e.message.clone(),
                            });
                        }
                        (
                            ComparisonCategory::Uncertain,
                            SyncProposal::Recompare,
                            e.message,
                        )
                    }
                    other => {
                        let (c, p) = classify(
                            a.is_some(),
                            b.is_some(),
                            complete,
                            other.map(|r| r.expect("error handled")),
                            quick,
                        );
                        let evidence = match c {
                            ComparisonCategory::VerifiedEqual => {
                                "Complete byte comparison with fresh native revisions; no metadata-fidelity or recovery claim."
                            }
                            ComparisonCategory::DifferentContent => {
                                "Content differs; no winner has been selected."
                            }
                            ComparisonCategory::AppearsUnchangedUnverified => {
                                "Size and modified time match. Content has not been checked."
                            }
                            ComparisonCategory::DestinationOnly => {
                                "Retain destination. No source-removal or destination-deletion action exists."
                            }
                            ComparisonCategory::SourceOnly => {
                                "Proposed copy only; source content has not been backed up or verified."
                            }
                            _ => {
                                "Evidence is incomplete; recompare. Content has not been established as equal or different."
                            }
                        };
                        (c, p, evidence.into())
                    }
                }
            };
            if mode == CompareMode::Verified
                && block.is_none()
                && !missing_snapshot
                && let (Some(snapshot), Some(root)) = (b, dataset.as_ref())
            {
                match content_digest(root, &path, snapshot, cancel, &mut count) {
                    Ok(hash) => {
                        if matches!(
                            category,
                            ComparisonCategory::VerifiedEqual
                                | ComparisonCategory::DifferentContent
                        ) && prior
                            .as_ref()
                            .and_then(|m| m.files.get(&history::key(&path)))
                            .is_some_and(|old| old != &hash)
                        {
                            category = ComparisonCategory::DestinationEditedSinceLastRun;
                            proposal = SyncProposal::ReviewDifference;
                            evidence.push_str(" Destination bytes changed since the retained observation. This is historical context, not proof of damage or permission to replace.");
                        }
                        observed.insert(history::key(&path), hash);
                    }
                    Err(e) => {
                        if comparison_issues.len() < MAX_ISSUES {
                            comparison_issues.push(SyncIssue {
                                side: Some(ComparisonSide::Destination),
                                display_path: path.to_string_lossy().into_owned(),
                                message: e.message.clone(),
                            });
                        }
                        category = ComparisonCategory::Uncertain;
                        proposal = SyncProposal::Recompare;
                        evidence = e.message;
                    }
                }
            }
            let raw = native::encode_path(&path);
            digest.update(&(raw.len() as u64).to_le_bytes());
            digest.update(&raw);
            for snapshot in [a, b] {
                if let Some(snapshot) = snapshot {
                    digest.update(snapshot.revision.as_bytes());
                } else {
                    digest.update(b"absent");
                }
            }
            let entry_id = blake3::hash(&raw).to_hex().to_string();
            observations.insert(entry_id.clone(), (path.clone(), a.cloned(), b.cloned()));
            rows.push(ComparisonEntry {
                companions: vec![],
                source_kind: src.objects.get(&path).copied(),
                destination_kind: dst.objects.get(&path).copied(),
                id: entry_id,
                display_path: path.to_string_lossy().into_owned(),
                source_bytes: a.map(|s| s.bytes.to_string()),
                destination_bytes: b.map(|s| s.bytes.to_string()),
                source_modified: a.and_then(|s| s.modified.clone()),
                destination_modified: b.and_then(|s| s.modified.clone()),
                category,
                proposal,
                evidence,
            });
            if rows.len() % 8 == 0 {
                self.compare_progress(id, count, rows.len() as u32)?;
            }
        }
        src.validate(&source)?;
        destination.open()?;
        if let Some(root) = &dataset {
            dst.validate(root)?;
        } else if destination.optional_directory(&namespace)?.is_some() {
            return Err(changed());
        }
        let mut state = self.comparisons.review.lock().map_err(|_| unavailable())?;
        let review = state
            .as_mut()
            .filter(|r| r.run.id == id)
            .ok_or_else(unavailable)?;
        let root = store::root(&self.catalog, review.run.source_root_id.clone())?;
        if root.path != source.path
            || root.identity != source.identity
            || root.volume != source.volume
            || root.view.availability != "online"
        {
            return Err(changed());
        }
        canceled(cancel)?;
        let append_scope = append::FrozenScope::capture(&src, &dst, &destination, &namespace);
        inspector::companions(&mut rows, &observations);
        review.run.baseline_id = prior.as_ref().map(|m| m.id.clone());
        review.run.bytes_read = count.to_string();
        review.run.entries = rows.len() as u32;
        let incomplete = src.incomplete
            || dst.incomplete
            || !comparison_issues.is_empty()
            || !blocked.is_empty();
        review.run.summary = Some(ComparisonSummary::collect(&rows));
        review.run.history_message = if mode == CompareMode::Verified && !incomplete {
            history::save(&self.catalog, id, history_scope, observed)?
        } else {
            "No history saved: quick, incomplete or canceled evidence cannot replace retained full-byte observations.".into()
        };
        review.run.issues = src
            .issues
            .into_iter()
            .map(|mut issue| {
                issue.side = Some(ComparisonSide::Source);
                issue
            })
            .chain(dst.issues.into_iter().map(|mut issue| {
                issue.side = Some(ComparisonSide::Destination);
                issue
            }))
            .chain(comparison_issues)
            .take(MAX_ISSUES)
            .collect();
        let completed = if incomplete {
            ComparisonState::Incomplete
        } else {
            ComparisonState::NeedsReview
        };
        digest.update(review.run.catalog_id.as_bytes());
        digest.update(review.run.source_root_id.as_bytes());
        if let Some(profile_id) = &review.run.profile_id {
            digest.update(profile_id.as_bytes());
        }
        digest.update(&native::encode_path(&namespace));
        digest.update(&serde_json::to_vec(&rows).map_err(|_| changed())?);
        review.run.plan_digest = (!incomplete).then(|| digest.finalize().to_hex().to_string());
        review.run.review_plan = review
            .run
            .plan_digest
            .as_ref()
            .map(|digest| ImmutableSyncPlan {
                run_id: review.run.id.clone(),
                catalog_id: review.run.catalog_id.clone(),
                source_root_id: review.run.source_root_id.clone(),
                policy_version: review.run.policy_version,
                digest: digest.clone(),
                entries: review.run.entries,
                execution_available: false,
            });
        review.run.message = "Read-only comparison finished. Review is session-only; copying, replacement and snapshot backup are unavailable. Existing dataset contents are not adopted. Other destination files remain outside this scope. Byte equality does not establish metadata fidelity or independent recovery.".into();
        review.rows = rows;
        review.observations = observations;
        review.source = Some(source);
        review.destination = dataset;
        review.append_scope = Some(append_scope);
        Ok(completed)
    }
    fn compare_progress(&self, id: &str, count: u64, entries: u32) -> Result<()> {
        let mut state = self.comparisons.review.lock().map_err(|_| unavailable())?;
        let review = state
            .as_mut()
            .filter(|r| r.run.id == id)
            .ok_or_else(unavailable)?;
        review.run.bytes_read = count.to_string();
        review.run.entries = entries;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
