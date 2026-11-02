//! Closed generated M4B lab. No API takes a caller path, root or manifest.
//! Actual byte work and recovery use the shared executor and event decoder.
use super::*;
mod retained;
pub(super) fn retained_probe() -> Result<serde_json::Value> {
    retained::probe()
}
use crate::{
    backup::comparison::{CompareMode, ComparisonState},
    catalog::Catalog,
    library::model::{RootMode, ScanFilter},
    library::{Library, LibraryOptions, WorkerPaths, native as source, store},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Component, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(test)]
thread_local! { static PREPARATION_FAULT: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) }; }
fn preparation_boundary(point: &'static str) -> Result<()> {
    #[cfg(test)]
    if PREPARATION_FAULT.with(|fault| fault.get() == Some(point)) {
        return Err(AppError::new(
            ErrorCode::WorkerFailed,
            "Injected interruption before preparation completion was recorded.",
        ));
    }
    let _ = point;
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Observed {
    pub path: Vec<u8>,
    pub identity: filesystem::FileIdentity,
    pub revision: String,
    pub bytes: u64,
    pub copy: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Projection {
    pub run_id: String,
    pub digest: String,
    pub catalog_id: String,
    pub root_id: String,
    pub source: Vec<u8>,
    pub source_identity: filesystem::FileIdentity,
    pub source_volume: String,
    pub destination: Vec<u8>,
    pub destination_identity: filesystem::FileIdentity,
    pub destination_volume: String,
    pub namespace: Vec<u8>,
    pub sources: Vec<Observed>,
    pub destinations: Vec<Observed>,
    pub source_directories: Vec<(Vec<u8>, String)>,
    pub uncopied: u32,
    pub destination_directories: Vec<(Vec<u8>, filesystem::FileIdentity)>,
}

enum OwnedScope {
    Run(tempfile::TempDir),
    #[cfg(test)]
    Child(PathBuf),
}
impl OwnedScope {
    fn path(&self) -> &Path {
        match self {
            Self::Run(root) => root.path(),
            #[cfg(test)]
            Self::Child(root) => root,
        }
    }
}
#[cfg(test)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureControl {
    projection: Projection,
    originals: Vec<(Vec<u8>, Vec<u8>)>,
    neighbors: Vec<(Vec<u8>, Vec<u8>)>,
    selection: String,
}
struct Fixture {
    owned: OwnedScope,
    library: Option<Arc<Library>>,
    projection: Projection,
    originals: BTreeMap<PathBuf, Vec<u8>>,
    neighbors: BTreeMap<PathBuf, Vec<u8>>,
    selection: String,
    approvals: approval::Approvals,
}
fn relative(path: &Path) -> Result<String> {
    let parts = path
        .components()
        .map(|p| match p {
            Component::Normal(name) => name
                .to_str()
                .filter(|s| crate::organization::portable_filename(s))
                .map(String::from)
                .ok_or_else(|| {
                    failed("Append target name requires unsupported native preparation.")
                }),
            _ => Err(failed(
                "Append locators must be native relative components.",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    if parts.is_empty() || parts.len() > 64 {
        return Err(failed("Invalid append locator depth."));
    }
    Ok(parts.join("/"))
}
fn check_file(root: &File, observed: &Observed) -> Result<()> {
    let file =
        locations::Source::capture(root, &source::decode_path(&observed.path)?)?.open(root)?;
    if filesystem::identity(&file)? != observed.identity
        || source::revision(&file)? != observed.revision
        || file.metadata()?.len() != observed.bytes
    {
        return Err(failed(
            "An append source or retained destination changed. Retain all objects and recompare.",
        ));
    }
    Ok(())
}
impl Fixture {
    fn new() -> Result<Self> {
        let owned = tempfile::Builder::new()
            .prefix("stillharbour-m4b-")
            .tempdir()?;
        let base = owned.path().canonicalize()?;
        let selection = uuid::Uuid::new_v4().to_string();
        std::fs::write(base.join("fixture-owner"), selection.as_bytes())?;
        let src = base.join("source");
        let dst = base.join("destination");
        let data = base.join("data");
        std::fs::create_dir_all(src.join("Album"))?;
        std::fs::create_dir(&dst)?;
        std::fs::create_dir(&data)?;
        let originals = BTreeMap::from([
            (src.join("Album/movie.mp4"), vec![7; CHUNK_BYTES * 2 + 31]),
            (src.join("Album/movie.xmp"), b"unique sidecar".to_vec()),
            (src.join(".hidden"), b"unknown regular bytes".to_vec()),
            (src.join("empty.bin"), Vec::new()),
            (src.join("edited.jpg"), b"source revision retained".to_vec()),
        ]);
        for (path, bytes) in &originals {
            std::fs::write(path, bytes)?;
        }
        let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour"))?);
        let library = Library::new(
            catalog.clone(),
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: std::env::current_exe()?,
                    exiftool: None,
                    working_directory: data.join("worker"),
                },
            },
        )?;
        let handle = source::open_root(&src, None)?;
        let root = store::add_root(
            &catalog,
            src.clone(),
            filesystem::identity(&handle)?,
            source::volume_token(&handle)?,
            "Generated append source".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )?;
        let dataset = dst
            .join(".stillharbour-copy")
            .join(catalog.summary()?.id)
            .join(root);
        std::fs::create_dir_all(&dataset)?;
        let neighbors = BTreeMap::from([
            (dst.join("neighbor.bin"), b"outside dataset".to_vec()),
            (
                dataset.join("destination-only.bin"),
                b"never delete".to_vec(),
            ),
            (
                dataset.join("edited.jpg"),
                b"destination edit retained".to_vec(),
            ),
        ]);
        for (path, bytes) in &neighbors {
            std::fs::write(path, bytes)?;
        }
        let selected = source::open_root(&dst, None)?;
        let run = library.backup_compare_start(
            dst,
            (
                filesystem::identity(&selected)?,
                source::volume_token(&selected)?,
            ),
            CompareMode::Verified,
        )?;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let state = library.backup_comparison(run.id.clone())?;
            if state.state != ComparisonState::Comparing {
                if state.state != ComparisonState::NeedsReview {
                    return Err(failed("Generated append comparison is incomplete."));
                }
                break;
            }
            if Instant::now() > deadline {
                return Err(failed("Generated append comparison timed out."));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let rows = library.backup_comparison_page(run.id.clone(), 0)?.entries;
        let keep = rows
            .iter()
            .find(|row| row.display_path == "edited.jpg")
            .ok_or_else(|| failed("Missing generated difference."))?
            .id
            .clone();
        let projection = library.append_fixture_projection(run.id, vec![keep])?;
        Ok(Self {
            owned: OwnedScope::Run(owned),
            library: Some(library),
            projection,
            originals,
            neighbors,
            selection,
            approvals: approval::Approvals::new(),
        })
    }
    fn boundary(&self, path: &Path) -> Result<()> {
        let base = self.owned.path().canonicalize()?;
        let resolved = match path.canonicalize() {
            Ok(path) => path,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => path
                .parent()
                .ok_or_else(|| failed("Owned append path has no parent."))?
                .canonicalize()?
                .join(
                    path.file_name()
                        .ok_or_else(|| failed("Owned append path has no filename."))?,
                ),
            Err(e) => return Err(e.into()),
        };
        if !path.is_absolute() || !resolved.starts_with(&base) || resolved == base {
            return Err(failed(
                "Append lab path escaped its constructor-owned scope.",
            ));
        }
        Ok(())
    }
    fn paths(&self) -> Result<(PathBuf, PathBuf)> {
        let src = source::decode_path(&self.projection.source)?;
        let dst = source::decode_path(&self.projection.destination)?;
        self.boundary(&src)?;
        self.boundary(&dst)?;
        Ok((src, dst))
    }
    fn validate(&self) -> Result<()> {
        self.validate_excluding(None)
    }
    fn validate_excluding(&self, excluded: Option<&[u8]>) -> Result<()> {
        let p = &self.projection;
        let (src, dst) = self.paths()?;
        let (catalog_id, (path, object, volume)) = match &self.library {
            Some(library) => (
                library.catalog.summary()?.id,
                library.catalog.call({
                    let id = p.root_id.clone();
                    move |db| super::sources::root(db, &id, false)
                })?,
            ),
            None => {
                #[cfg(test)]
                {
                    let path = self.owned.path().join("data/Fixture.stillharbour");
                    self.boundary(&path)?;
                    let db = Connection::open_with_flags(
                        path,
                        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                    )?;
                    (
                        db.query_row(
                            "SELECT uuid FROM catalog_identity WHERE singleton=1",
                            [],
                            |r| r.get::<_, String>(0),
                        )?,
                        super::sources::root(&db, &p.root_id, false)?,
                    )
                }
                #[cfg(not(test))]
                {
                    return Err(failed(
                        "Only the closed test child can reopen fixture registration.",
                    ));
                }
            }
        };
        if catalog_id != p.catalog_id {
            return Err(failed("Append catalog identity changed."));
        }
        if path != p.source
            || object.volume != p.source_identity.volume
            || object.file != p.source_identity.file
            || volume != p.source_volume
        {
            return Err(failed("Append root registration changed."));
        }
        let a = source::open_root(&src, Some(&p.source_identity))?;
        let b = source::open_root(&dst, Some(&p.destination_identity))?;
        if source::volume_token(&a)? != p.source_volume
            || source::volume_token(&b)? != p.destination_volume
        {
            return Err(failed("Append volume changed."));
        }
        for observed in &p.sources {
            check_file(&a, observed)?;
        }
        for (path, version) in &p.source_directories {
            let path = source::decode_path(path)?;
            let directory = if path.as_os_str().is_empty() {
                a.try_clone()?
            } else {
                let mut parent = a.try_clone()?;
                for part in path.components() {
                    let Component::Normal(name) = part else {
                        return Err(failed("Invalid source directory."));
                    };
                    parent = locations::child(&parent, name, true)?;
                }
                parent
            };
            if source::revision(&directory)? != *version {
                return Err(failed("Append source membership changed."));
            }
        }
        let dataset = dst.join(source::decode_path(&p.namespace)?);
        self.boundary(&dataset)?;
        let directory = source::open_root(&dataset, None)?;
        for (path, expected) in &p.destination_directories {
            let mut parent = directory.try_clone()?;
            for part in source::decode_path(path)?.components() {
                let Component::Normal(name) = part else {
                    return Err(failed("Invalid retained destination directory."));
                };
                parent = locations::child(&parent, name, true)?;
            }
            let actual = filesystem::identity(&parent)?;
            if actual.volume != expected.volume || actual.file != expected.file {
                return Err(failed("Retained destination directory identity changed."));
            }
        }
        for observed in &p.destinations {
            if excluded.is_some_and(|path| path == observed.path) {
                continue;
            }
            check_file(&directory, observed)?;
        }
        Ok(())
    }
    #[cfg(test)]
    fn control(&self) -> FixtureControl {
        FixtureControl {
            projection: self.projection.clone(),
            originals: self
                .originals
                .iter()
                .map(|(p, b)| (source::encode_path(p), b.clone()))
                .collect(),
            neighbors: self
                .neighbors
                .iter()
                .map(|(p, b)| (source::encode_path(p), b.clone()))
                .collect(),
            selection: self.selection.clone(),
        }
    }
    #[cfg(test)]
    fn from_control(root: PathBuf, control: FixtureControl, token: &str) -> Result<Self> {
        let root = root.canonicalize()?;
        if !root.starts_with(std::env::temp_dir().canonicalize()?)
            || !root
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("stillharbour-m4b-"))
            || control.selection != token
            || std::fs::read(root.join("fixture-owner"))? != token.as_bytes()
        {
            return Err(failed("Not the exact generated replacement child scope."));
        }
        let decode = |items: Vec<(Vec<u8>, Vec<u8>)>| {
            items
                .into_iter()
                .map(|(p, b)| Ok((source::decode_path(&p)?, b)))
                .collect::<Result<BTreeMap<_, _>>>()
        };
        let fixture = Self {
            owned: OwnedScope::Child(root.clone()),
            library: None,
            projection: control.projection,
            originals: decode(control.originals)?,
            neighbors: decode(control.neighbors)?,
            selection: token.into(),
            approvals: approval::Approvals::new(),
        };
        let (src, dst) = fixture.paths()?;
        if src != root.join("source") || dst != root.join("destination") {
            return Err(failed("Generated child roots changed."));
        }
        for (path, bytes) in fixture.originals.iter().chain(&fixture.neighbors) {
            fixture.boundary(path)?;
            if std::fs::read(path)? != *bytes {
                return Err(failed("Generated child manifest changed before admission."));
            }
        }
        fixture.validate()?;
        Ok(fixture)
    }
    fn lease(&self) -> Result<crate::jobs::operation_locks::Lease> {
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let (src, dst) = self.paths()?;
        Registry::shared().try_acquire(vec![
            Scope::new(DirectoryChain::open(&src)?, Access::Read),
            Scope::new(output_chain(&dst)?, Access::Write),
        ])
    }
    fn preparation_review(&self) -> Result<approval::Review> {
        approval::Review::new(
            approval::Action::PrepareNames,
            self.projection.run_id.clone(),
            self.projection.digest.clone(),
            self.selection.clone(),
        )
    }
    fn prepare(&self, proof: approval::Proof) -> Result<CopyPlan> {
        proof.authorize(&self.preparation_review()?)?;
        let _lease = self.lease()?;
        self.validate()?;
        let (src, dst) = self.paths()?;
        let source_handle = source::open_root(&src, Some(&self.projection.source_identity))?;
        let destination = output_chain(&dst)?;
        let mut seed = CopyPlan {
            id: uuid::Uuid::new_v4().to_string(),
            digest: String::new(),
            source: Object::of(&source_handle)?,
            destination: Object::of(destination.leaf())?,
            source_volume: self.projection.source_volume.clone(),
            destination_volume: self.projection.destination_volume.clone(),
            files: vec![],
        };
        seed.digest = seed.digest()?;
        let prep_path = self.owned.path().join("data/preparation.sqlite");
        self.boundary(&prep_path)?;
        let preparation = Journal::create(&prep_path, &seed)?;
        preparation.append(
            None,
            "append_scope_intent",
            serde_json::to_value(&self.projection)
                .map_err(|e| failed("Could not retain append scope.").caused_by(e))?,
        )?;
        let stage_relative = format!(".stillharbour-staging-{}", seed.id);
        preparation.append(
            None,
            "staging_directory_intent",
            serde_json::json!({"path":stage_relative}),
        )?;
        let stage = native::create_directory(destination.leaf(), &stage_relative)?;
        preparation_boundary("stage_directory")?;
        preparation.append(
            None,
            "staging_directory_created",
            serde_json::to_value(Object::of(&stage)?)
                .map_err(|e| failed("Could not retain staging identity.").caused_by(e))?,
        )?;
        let namespace = relative(&source::decode_path(&self.projection.namespace)?)?;
        let mut created = BTreeMap::new();
        let mut files = Vec::new();
        let mut selected: Vec<_> = self.projection.sources.iter().filter(|f| f.copy).collect();
        selected.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
        for observed in selected {
            let raw = source::decode_path(&observed.path)?;
            let target = relative(&raw)?;
            let (relative_parent, leaf) = target
                .rsplit_once('/')
                .map_or((namespace.clone(), target.as_str()), |(parent, leaf)| {
                    (format!("{namespace}/{parent}"), leaf)
                });
            let mut parent = destination.leaf().try_clone()?;
            let parts: Vec<_> = relative_parent.split('/').collect();
            let mut prefix = String::new();
            for name in parts {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(name);
                parent = match native::lookup(&parent, name)? {
                    Some(entry) if entry.kind == native::EntryKind::Directory => {
                        if let Some(relative) = prefix.strip_prefix(&format!("{namespace}/")) {
                            let known =
                                self.projection
                                    .destination_directories
                                    .iter()
                                    .find(|(p, _)| {
                                        source::decode_path(p).is_ok_and(|p| {
                                            self::relative(&p).ok().as_deref() == Some(relative)
                                        })
                                    });
                            let expected = created.get(&prefix).cloned().or_else(|| {
                                known.map(|(_, i)| Object {
                                    volume: i.volume,
                                    file: i.file,
                                })
                            });
                            if expected.as_ref() != Some(&entry.object) {
                                return Err(failed(
                                    "An unreviewed destination parent arrived or changed. Recompare before writing.",
                                ));
                            }
                        }
                        native::read_directory(&parent, name)?
                    }
                    Some(_) => {
                        return Err(failed(
                            "Append parent is occupied by an incompatible object.",
                        ));
                    }
                    None => {
                        preparation.append(
                            None,
                            "directory_intent",
                            serde_json::json!({"path":prefix}),
                        )?;
                        let next = native::create_directory(&parent, name)?;
                        preparation_boundary("target_directory")?;
                        preparation.append(
                            None,
                            "directory_created",
                            serde_json::json!({"path":prefix,"object":Object::of(&next)?}),
                        )?;
                        created.insert(prefix.clone(), Object::of(&next)?);
                        next
                    }
                };
            }
            // An exclusive empty leaf proves the native name in this actual
            // parent; relocate it into staging before it ever receives bytes.
            preparation.append(
                None,
                "target_probe_intent",
                serde_json::json!({"path":format!("{relative_parent}/{leaf}")}),
            )?;
            let probe = native::create_file(&parent, leaf)?;
            preparation_boundary("target_probe")?;
            let identity = filesystem::identity(&probe)?;
            preparation.append(None,"target_probe_created",serde_json::json!({"path":format!("{relative_parent}/{leaf}"),"object":Object::of(&probe)?}))?;
            let stage_leaf = format!("{}-{}.partial", seed.id, files.len());
            let moved = native::publish_to(&parent, leaf, probe, &stage, &stage_leaf, &identity)?;
            preparation_boundary("probe_staged")?;
            preparation.append(
                None,
                "target_probe_staged",
                serde_json::json!({"stage":stage_leaf,"object":Object::of(&moved)?}),
            )?;
            let file = CopyFile {
                source: raw.to_string_lossy().into_owned(),
                target: leaf.into(),
                stage: stage_leaf,
                object: Object {
                    volume: observed.identity.volume,
                    file: observed.identity.file,
                },
                revision: observed.revision.clone(),
                bytes: observed.bytes,
                source_location: Some(locations::Source::capture(&source_handle, &raw)?),
                destination_location: Some(locations::Destination::capture(
                    destination.leaf(),
                    &relative_parent,
                )?),
                stage_location: Some(locations::Destination::capture(
                    destination.leaf(),
                    &stage_relative,
                )?),
                prepared_stage: Some(PreparedStage {
                    object: Object::of(&moved)?,
                    revision: source::revision(&moved)?,
                }),
            };
            files.push(file);
        }
        self.validate()?;
        if files.is_empty() {
            return Err(failed("No new append files were reviewed."));
        }
        seed.files = files;
        seed.digest = seed.digest()?;
        let journal_path = self.owned.path().join("data/copy.sqlite");
        self.boundary(&journal_path)?;
        let _journal = Journal::create(&journal_path, &seed)?;
        preparation_boundary("copy_anchor")?;
        preparation.append(None,"copy_plan_bound",serde_json::json!({"id":seed.id,"digest":seed.digest,"createdDirectories":created.len()}))?;
        Ok(seed)
    }
    fn journal(&self, plan: &CopyPlan) -> Result<Journal> {
        let path = self.owned.path().join("data/copy.sqlite");
        self.boundary(&path)?;
        Journal::open(&path, plan)
    }
    fn copy_review(&self, plan: &CopyPlan) -> Result<approval::Review> {
        approval::Review::new(
            approval::Action::CopyFiles,
            plan.id.clone(),
            plan.digest.clone(),
            self.selection.clone(),
        )
    }
    fn copy(
        &self,
        plan: &CopyPlan,
        proof: approval::Proof,
        fault: Option<CopyFault>,
    ) -> Result<CopyOutcome> {
        proof.authorize(&self.copy_review(plan)?)?;
        let _lease = self.lease()?;
        self.validate()?;
        let (src, dst) = self.paths()?;
        let a = source::open_root(&src, Some(&self.projection.source_identity))?;
        let b = output_chain(&dst)?;
        let journal = self.journal(plan)?;
        catalog_journal::append_fixture_history(&journal, plan)?;
        CopyContext {
            plan,
            journal: &journal,
            source_root: &a,
            destination: b.leaf(),
            io: &PacedIo::shared(),
            cancel: &AtomicBool::new(false),
            validate_scope: &|| self.validate(),
        }
        .execute(fault)
    }
    fn inspect(&self, plan: &CopyPlan) -> Result<Vec<RecoveredItem>> {
        let _lease = self.lease()?;
        self.validate()?;
        let (src, dst) = self.paths()?;
        let a = source::open_root(&src, Some(&self.projection.source_identity))?;
        let b = output_chain(&dst)?;
        let journal = self.journal(plan)?;
        catalog_journal::append_fixture_history(&journal, plan)?;
        plan.files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                inspect_item(
                    f,
                    i,
                    &journal,
                    &a,
                    b.leaf(),
                    &PacedIo::shared(),
                    &AtomicBool::new(false),
                )
            })
            .collect()
    }
    fn recovery_review(&self, plan: &CopyPlan) -> Result<resume::Review> {
        let _lease = self.lease()?;
        self.validate()?;
        let (src, dst) = self.paths()?;
        let a = source::open_root(&src, Some(&self.projection.source_identity))?;
        let b = output_chain(&dst)?;
        let journal = self.journal(plan)?;
        let (cursors, _, digest) = catalog_journal::append_fixture_history(&journal, plan)?;
        resume::Review::capture(
            plan,
            &cursors,
            digest,
            &CopyInputs {
                source_root: &a,
                destination: b.leaf(),
                io: &PacedIo::shared(),
                cancel: &AtomicBool::new(false),
                validate_scope: &|| self.validate(),
            },
        )
    }
    fn continue_copy(
        &self,
        plan: &CopyPlan,
        review: &resume::Review,
        proof: approval::Proof,
        fault: Option<CopyFault>,
    ) -> Result<CopyOutcome> {
        proof.authorize(&review.approval(plan, &self.selection)?)?;
        let _lease = self.lease()?;
        self.validate()?;
        let (src, dst) = self.paths()?;
        let a = source::open_root(&src, Some(&self.projection.source_identity))?;
        let b = output_chain(&dst)?;
        let journal = self.journal(plan)?;
        let (cursors, terminal, digest) = catalog_journal::append_fixture_history(&journal, plan)?;
        resume::Continuation {
            plan,
            journal: &journal,
            cursors: &cursors,
            terminal,
            review,
            history_digest: digest,
            inputs: &CopyInputs {
                source_root: &a,
                destination: b.leaf(),
                io: &PacedIo::shared(),
                cancel: &AtomicBool::new(false),
                validate_scope: &|| self.validate(),
            },
            fault,
            progress: None,
        }
        .execute()
    }
    fn audit(&self, plan: &CopyPlan) -> Result<()> {
        self.validate()?;
        for (p, bytes) in self.originals.iter().chain(&self.neighbors) {
            self.boundary(p)?;
            if std::fs::read(p)? != *bytes {
                return Err(failed("Append lab preservation audit failed."));
            }
        }
        let (_, dst) = self.paths()?;
        let output = output_chain(&dst)?;
        for file in &plan.files {
            let parent = destination_parent(output.leaf(), file)?;
            if let Some(mut published) = native::maybe_read(&parent, &file.target)? {
                let (src, _) = self.paths()?;
                let root = source::open_root(&src, None)?;
                let mut original = read_source(&root, file)?;
                if !matches!(
                    compare(
                        &PacedIo::shared(),
                        &mut original,
                        &mut published,
                        file,
                        &AtomicBool::new(false)
                    )?,
                    IoStep::Completed(_)
                ) {
                    return Err(failed("Append output audit canceled."));
                }
            }
        }
        Ok(())
    }
    fn prepare_approved(&self) -> Result<CopyPlan> {
        let review = self.preparation_review()?;
        let ticket = self.approvals.issue(review.clone())?;
        self.prepare(self.approvals.consume(&ticket, &review)?)
    }
    fn copy_approved(&self, plan: &CopyPlan, fault: Option<CopyFault>) -> Result<CopyOutcome> {
        let review = self.copy_review(plan)?;
        let ticket = self.approvals.issue(review.clone())?;
        self.copy(plan, self.approvals.consume(&ticket, &review)?, fault)
    }
    fn continue_approved(&self, plan: &CopyPlan, fault: Option<CopyFault>) -> Result<CopyOutcome> {
        let review = self.recovery_review(plan)?;
        let expected = review.approval(plan, &self.selection)?;
        let ticket = self.approvals.issue(expected.clone())?;
        self.continue_copy(
            plan,
            &review,
            self.approvals.consume(&ticket, &expected)?,
            fault,
        )
    }
}

pub(super) fn probe() -> Result<serde_json::Value> {
    let f = Fixture::new()?;
    let plan = f.prepare_approved()?;
    let result = f.copy_approved(&plan, None)?;
    let inspection = f.inspect(&plan)?;
    f.audit(&plan)?;
    // Even a completed stream is inspected afresh under a separate approval;
    // the retained journal never restores a grant or permits blind replay.
    if f.continue_approved(&plan, None)? != CopyOutcome::CompletedVerified {
        return Err(failed("Completed append recovery did not verify."));
    }
    if result != CopyOutcome::CompletedVerified
        || inspection
            .iter()
            .any(|s| *s != RecoveredItem::PublishedVerified)
    {
        return Err(failed("Generated append fixture did not verify."));
    }
    Ok(
        serde_json::json!({"scope":"constructor-owned-generated-only","nativeOs":std::env::consts::OS,"copyFiles":plan.files.len(),"uncopiedSourceRevisions":f.projection.uncopied,"sourcesPreserved":true,"destinationOnlyPreserved":true,"separateStaging":true,"fullByteVerified":true,"productionExecutionAvailable":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_shared_executor_copies_only_new_files_with_separate_staging() {
        let receipt = probe().unwrap();
        assert_eq!(receipt["copyFiles"], 4);
        assert_eq!(receipt["uncopiedSourceRevisions"], 1);
        assert_eq!(receipt["productionExecutionAvailable"], false);
    }
    const FAULTS: [CopyFault; 9] = [
        CopyFault::AfterIntent,
        CopyFault::AfterStageCreation,
        CopyFault::AfterCopyChunk,
        CopyFault::AfterFlush,
        CopyFault::AfterVerification,
        CopyFault::BeforePublish,
        CopyFault::AfterPublish,
        CopyFault::AfterItemCommit,
        CopyFault::AfterPlanCommit,
    ];
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_cold_child() {
        let Some(root) = std::env::var_os("STILLHARBOUR_M4B_CHILD_ROOT") else {
            return;
        };
        let root = PathBuf::from(root).canonicalize().unwrap();
        let token = std::env::var("STILLHARBOUR_M4B_CHILD_TOKEN").unwrap();
        uuid::Uuid::parse_str(&token).unwrap();
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("stillharbour-m4b-")
        );
        assert_eq!(
            std::fs::read(root.join("fixture-owner")).unwrap(),
            token.as_bytes()
        );
        let db = Connection::open_with_flags(
            root.join("data/copy.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let text: String = db
            .query_row(
                "SELECT payload FROM plan WHERE length(CAST(payload AS BLOB))<=65536",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let plan: CopyPlan = serde_json::from_str(&text).unwrap();
        drop(db);
        assert_eq!(plan.digest().unwrap(), plan.digest);
        assert_eq!(
            plan.digest,
            std::env::var("STILLHARBOUR_M4B_CHILD_DIGEST").unwrap()
        );
        assert_eq!(plan.files.len(), 4);
        let a = DirectoryChain::open(&root.join("source")).unwrap();
        let b = output_chain(&root.join("destination")).unwrap();
        for file in &plan.files {
            assert!(matches!(
                file.target.as_str(),
                "movie.mp4" | "movie.xmp" | ".hidden" | "empty.bin"
            ));
            assert_eq!(
                file.stage_location.as_ref().unwrap().relative_parent,
                format!(".stillharbour-staging-{}", plan.id)
            );
            assert!(
                file.destination_location
                    .as_ref()
                    .unwrap()
                    .relative_parent
                    .starts_with(".stillharbour-copy/")
            );
            let mut source = read_source(a.leaf(), file).unwrap();
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut source, &mut bytes).unwrap();
            match file.target.as_str() {
                "movie.mp4" => assert_eq!(bytes, vec![7; CHUNK_BYTES * 2 + 31]),
                "movie.xmp" => assert_eq!(bytes, b"unique sidecar"),
                ".hidden" => assert_eq!(bytes, b"unknown regular bytes"),
                "empty.bin" => assert!(bytes.is_empty()),
                _ => unreachable!(),
            }
        }
        let fault = FAULTS
            .into_iter()
            .find(|fault| {
                format!("{fault:?}") == std::env::var("STILLHARBOUR_M4B_CHILD_FAULT").unwrap()
            })
            .unwrap();
        let journal = Journal::open(&root.join("data/copy.sqlite"), &plan).unwrap();
        let validate = || {
            let a = DirectoryChain::open(&root.join("source"))?;
            let b = output_chain(&root.join("destination"))?;
            if !plan.source.matches(a.leaf())?
                || !plan.destination.matches(b.leaf())?
                || source::volume_token(a.leaf())? != plan.source_volume
                || source::volume_token(b.leaf())? != plan.destination_volume
            {
                return Err(failed("Cold append scope changed."));
            }
            Ok(())
        };
        CRASH_ON_FAULT.store(true, Ordering::Release);
        let _ = CopyContext {
            plan: &plan,
            journal: &journal,
            source_root: a.leaf(),
            destination: b.leaf(),
            io: &PacedIo::shared(),
            cancel: &AtomicBool::new(false),
            validate_scope: &validate,
        }
        .execute(Some(fault));
        panic!("Cold append fault did not terminate the process");
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_actual_child_exit_recovers_all_nine_copy_boundaries() {
        for fault in FAULTS {
            let f = Fixture::new().unwrap();
            let plan = f.prepare_approved().unwrap();
            let review = f.copy_review(&plan).unwrap();
            let ticket = f.approvals.issue(review.clone()).unwrap();
            f.approvals
                .consume(&ticket, &review)
                .unwrap()
                .authorize(&review)
                .unwrap();
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "executor::append_fixture::tests::m4b_append_cold_child",
                    "--nocapture",
                ])
                .env("STILLHARBOUR_M4B_CHILD_ROOT", f.owned.path())
                .env("STILLHARBOUR_M4B_CHILD_TOKEN", &f.selection)
                .env("STILLHARBOUR_M4B_CHILD_DIGEST", &plan.digest)
                .env("STILLHARBOUR_M4B_CHILD_FAULT", format!("{fault:?}"))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(20);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() > deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("Cold append child timed out at {fault:?}");
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            assert_eq!(status.code(), Some(77), "{fault:?}");
            f.audit(&plan).unwrap();
            let journal = f.journal(&plan).unwrap();
            catalog_journal::append_fixture_history(&journal, &plan).unwrap();
            drop(journal);
            assert!(f.copy_approved(&plan, None).is_err());
            assert_eq!(
                f.continue_approved(&plan, None).unwrap(),
                CopyOutcome::CompletedVerified
            );
            f.audit(&plan).unwrap();
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_every_copy_boundary_retains_versions_and_requires_fresh_continuation() {
        for fault in FAULTS {
            let f = Fixture::new().unwrap();
            let plan = f.prepare_approved().unwrap();
            assert_eq!(
                f.copy_approved(&plan, Some(fault)).unwrap(),
                CopyOutcome::Interrupted,
                "{fault:?}"
            );
            f.audit(&plan).unwrap();
            let states = f.inspect(&plan).unwrap();
            if fault == CopyFault::AfterCopyChunk {
                assert_eq!(states[0], RecoveredItem::RetainedPartialStage);
            }
            let before = f.journal(&plan).unwrap().history_count().unwrap();
            assert!(f.copy_approved(&plan, None).is_err());
            assert_eq!(f.journal(&plan).unwrap().history_count().unwrap(), before);
            assert_eq!(
                f.continue_approved(&plan, None).unwrap(),
                CopyOutcome::CompletedVerified,
                "{fault:?}"
            );
            assert!(
                f.inspect(&plan)
                    .unwrap()
                    .iter()
                    .all(|s| *s == RecoveredItem::PublishedVerified)
            );
            f.audit(&plan).unwrap();
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_late_target_is_never_overwritten_or_adopted() {
        let f = Fixture::new().unwrap();
        let plan = f.prepare_approved().unwrap();
        let (_, dst) = f.paths().unwrap();
        let root = output_chain(&dst).unwrap();
        let file = &plan.files[0];
        let location = file.destination_location.as_ref().unwrap();
        let path = dst.join(&location.relative_parent).join(&file.target);
        f.boundary(&path).unwrap();
        std::fs::write(&path, b"late unique occupant").unwrap();
        assert!(f.copy_approved(&plan, None).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"late unique occupant");
        assert!(f.inspect(&plan).is_err());
        assert!(f.recovery_review(&plan).is_err());
        let stage = staging_parent(root.leaf(), file).unwrap();
        assert_eq!(
            native::read(&stage, &file.stage)
                .unwrap()
                .metadata()
                .unwrap()
                .len(),
            file.bytes
        );
        for (path, bytes) in f.originals.iter().chain(&f.neighbors) {
            assert_eq!(std::fs::read(path).unwrap(), *bytes);
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_corrupt_stage_is_retained_and_never_accepted_anyway() {
        let f = Fixture::new().unwrap();
        let plan = f.prepare_approved().unwrap();
        assert_eq!(
            f.copy_approved(&plan, Some(CopyFault::AfterFlush)).unwrap(),
            CopyOutcome::Interrupted
        );
        let (_, dst) = f.paths().unwrap();
        let file = &plan.files[0];
        let path = dst
            .join(&file.stage_location.as_ref().unwrap().relative_parent)
            .join(&file.stage);
        f.boundary(&path).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[0] ^= 1;
        std::fs::write(&path, &bytes).unwrap();
        let before = f.journal(&plan).unwrap().history_count().unwrap();
        assert!(f.continue_approved(&plan, None).is_err());
        assert_eq!(f.journal(&plan).unwrap().history_count().unwrap(), before);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert!(f.inspect(&plan).is_err());
        f.audit(&plan).unwrap();
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_disk_full_and_catalog_denial_do_not_report_success() {
        for fault in [
            CopyFault::InjectedDiskFullAfterChunk,
            CopyFault::ReadOnlyJournalAfterPublish,
        ] {
            let f = Fixture::new().unwrap();
            let plan = f.prepare_approved().unwrap();
            assert!(f.copy_approved(&plan, Some(fault)).is_err());
            f.audit(&plan).unwrap();
            assert_eq!(
                f.continue_approved(&plan, None).unwrap(),
                CopyOutcome::CompletedVerified
            );
            f.audit(&plan).unwrap();
        }
    }
    fn inventory(f: &Fixture) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(f: &Fixture, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
            f.boundary(path).unwrap();
            for entry in std::fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let p = entry.path();
                f.boundary(&p).unwrap();
                let kind = entry.file_type().unwrap();
                assert!(!kind.is_symlink());
                if kind.is_dir() {
                    result.insert(p.clone(), vec![]);
                    visit(f, &p, result);
                } else {
                    result.insert(p.clone(), std::fs::read(p).unwrap());
                }
            }
        }
        let mut result = BTreeMap::new();
        visit(f, &f.paths().unwrap().1, &mut result);
        result
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_preparation_interruptions_retain_intent_without_blind_replay() {
        for point in [
            "stage_directory",
            "target_directory",
            "target_probe",
            "probe_staged",
            "copy_anchor",
        ] {
            let f = Fixture::new().unwrap();
            PREPARATION_FAULT.with(|p| p.set(Some(point)));
            let result = f.prepare_approved();
            PREPARATION_FAULT.with(|p| p.set(None));
            assert!(result.is_err(), "{point}");
            let before = inventory(&f);
            assert!(f.prepare_approved().is_err());
            assert_eq!(inventory(&f), before);
            for (path, bytes) in f.originals.iter().chain(&f.neighbors) {
                assert_eq!(std::fs::read(path).unwrap(), *bytes);
            }
            let db = Connection::open(f.owned.path().join("data/preparation.sqlite")).unwrap();
            let count: i64 = db
                .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
                .unwrap();
            assert!(count > 0);
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4b_append_wrong_action_and_stale_recovery_approval_never_write() {
        let f = Fixture::new().unwrap();
        let plan = f.prepare_approved().unwrap();
        let expected = f.preparation_review().unwrap();
        let ticket = f.approvals.issue(expected.clone()).unwrap();
        let proof = f.approvals.consume(&ticket, &expected).unwrap();
        assert!(f.copy(&plan, proof, None).is_err());
        assert_eq!(f.journal(&plan).unwrap().history_count().unwrap(), 0);
        f.copy_approved(&plan, Some(CopyFault::AfterCopyChunk))
            .unwrap();
        let review = f.recovery_review(&plan).unwrap();
        let expected = review.approval(&plan, &f.selection).unwrap();
        let ticket = f.approvals.issue(expected.clone()).unwrap();
        let (_, dst) = f.paths().unwrap();
        let file = &plan.files[0];
        let path = dst
            .join(&file.stage_location.as_ref().unwrap().relative_parent)
            .join(&file.stage);
        f.boundary(&path).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[0] ^= 1;
        std::fs::write(&path, &bytes).unwrap();
        let before = f.journal(&plan).unwrap().history_count().unwrap();
        assert!(
            f.continue_copy(
                &plan,
                &review,
                f.approvals.consume(&ticket, &expected).unwrap(),
                None
            )
            .is_err()
        );
        assert_eq!(f.journal(&plan).unwrap().history_count().unwrap(), before);
        f.audit(&plan).unwrap();
    }
}
