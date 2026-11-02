//! Backend-only snapshot staging/restoration through the central copy engine.
//! Caller owns the native scope review/approval and catalog job index. This
//! adapter never removes sources, adopts an existing namespace or replays jobs.
use super::*;
use crate::backup::native_metadata::MetadataManifest;
use crate::library::native as source;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone)]
pub(crate) struct Root {
    owned: bool,
    pub path: PathBuf,
    pub identity: filesystem::FileIdentity,
    pub volume: String,
}
#[derive(Clone)]
pub(crate) struct FileSpec {
    pub metadata: Option<MetadataManifest>,
    pub relative: Vec<u8>,
    pub target: String,
    pub identity: filesystem::FileIdentity,
    pub revision: String,
    pub bytes: u64,
    pub digest: String,
}
impl Root {
    pub(crate) fn existing(
        path: PathBuf,
        identity: filesystem::FileIdentity,
        volume: String,
    ) -> Self {
        Self {
            path,
            identity,
            volume,
            owned: false,
        }
    }
    pub(crate) fn source(&self) -> Result<File> {
        let file = source::open_root(&self.path, Some(&self.identity))?;
        if source::volume_token(&file)? != self.volume {
            return Err(failed("Snapshot root volume changed."));
        }
        Ok(file)
    }
    fn output(&self) -> Result<DirectoryChain> {
        let chain = output_chain(&self.path)?;
        let actual = filesystem::identity(chain.leaf())?;
        if actual.volume != self.identity.volume
            || actual.file != self.identity.file
            || source::volume_token(chain.leaf())? != self.volume
        {
            return Err(failed("Snapshot output root changed identity/volume."));
        }
        Ok(chain)
    }
}
fn apply_metadata(
    root: &Root,
    relative: &Path,
    metadata: &MetadataManifest,
    expected: &filesystem::FileIdentity,
) -> Result<()> {
    if !root.owned {
        return Err(failed(
            "Native metadata restoration requires a newly owned output.",
        ));
    }
    #[cfg(windows)]
    {
        let chain = root.output()?;
        let mut parents = Vec::new();
        let components = relative.components().collect::<Vec<_>>();
        if components.is_empty()
            || components
                .iter()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(failed("Invalid newly owned metadata path."));
        }
        for component in &components[..components.len() - 1] {
            let parent = parents.last().unwrap_or_else(|| chain.leaf());
            let name = component
                .as_os_str()
                .to_str()
                .ok_or_else(|| failed("Unsupported owned metadata component."))?;
            parents.push(native::read_directory(parent, name)?);
        }
        let parent = parents.last().unwrap_or_else(|| chain.leaf());
        let file = filesystem::windows::owned_metadata_child(
            parent,
            components[components.len() - 1].as_os_str(),
        )?;
        if filesystem::identity(&file)? != *expected {
            return Err(failed(
                "New output identity changed before native metadata restoration.",
            ));
        }
        metadata.restore_new(&file)?;
        metadata.compare(&file)?;
    }
    #[cfg(not(windows))]
    {
        let file = source::open_asset(&root.path, &root.identity, &root.volume, relative)?;
        if filesystem::identity(&file)? != *expected {
            return Err(failed(
                "New output identity changed before native metadata restoration.",
            ));
        }
        metadata.restore_new(&file)?;
        metadata.compare(&file)?;
    }
    Ok(())
}
pub(crate) fn make_directory(parent: &Root, leaf: &str) -> Result<Root> {
    if !crate::organization::portable_filename(leaf) {
        return Err(failed("Unsafe snapshot namespace name."));
    }
    let chain = parent.output()?;
    let directory = native::create_directory(chain.leaf(), leaf)?;
    let path = parent.path.join(leaf);
    let current = source::open_root(&path, None)?;
    if Object::of(&current)? != Object::of(&directory)? {
        return Err(failed("Snapshot namespace membership changed."));
    }
    Ok(Root {
        owned: true,
        path,
        identity: filesystem::identity(&directory)?,
        volume: source::volume_token(&directory)?,
    })
}
/// New app-owned metadata only; caller records durable intent before this call.
pub(crate) fn write_new(parent: &Root, leaf: &str, bytes: &[u8]) -> Result<()> {
    if !parent.owned {
        return Err(failed(
            "Snapshot metadata writes require a newly created owned namespace.",
        ));
    }
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(failed("Snapshot metadata exceeds its bound."));
    }
    let chain = parent.output()?;
    let mut file = native::create_file(chain.leaf(), leaf)?;
    std::io::Write::write_all(&mut file, bytes)?;
    file.sync_all()?;
    let identity = filesystem::identity(&file)?;
    if !native::entry_matches(chain.leaf(), leaf, &identity)? {
        return Err(failed("Snapshot metadata membership changed."));
    }
    Ok(())
}
pub(crate) fn digest(file: &mut File, cancel: &AtomicBool) -> Result<String> {
    let version = source::revision(file)?;
    let identity = filesystem::identity(file)?;
    let mut hash = blake3::Hasher::new();
    let mut bytes = [0u8; CHUNK_BYTES];
    loop {
        match PacedIo::shared().read(file, &mut bytes, cancel)? {
            IoStep::Canceled => return Err(failed("Snapshot content verification canceled.")),
            IoStep::Completed(0) => break,
            IoStep::Completed(n) => {
                hash.update(&bytes[..n]);
            }
        }
    }
    if identity != filesystem::identity(file)? || source::revision(file)? != version {
        return Err(failed("Snapshot content changed during verification."));
    }
    Ok(hash.finalize().to_hex().to_string())
}
fn new_journal(path: &Path, plan: &CopyPlan) -> Result<Journal> {
    if serde_json::to_vec(plan)
        .map_err(|e| failed("Invalid snapshot copy plan.").caused_by(e))?
        .len()
        > 64 * 1024 * 1024
    {
        return Err(failed("The snapshot copy plan exceeds its bounded size."));
    }
    // Reserve the exact app-owned journal name before SQLite opens it. No
    // existing file can be adopted by this new-job-only bridge.
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};
        options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let reservation = options.open(path)?;
    let identity = filesystem::identity(&reservation)?;
    let journal = Journal::create(path, plan)?;
    let mut observe = std::fs::OpenOptions::new();
    observe.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };
        observe
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        observe.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let current = observe.open(path)?;
    if filesystem::identity(&current)? != identity {
        return Err(failed("Snapshot journal path changed membership."));
    }
    Ok(journal)
}
pub(crate) fn copy(
    source: &Root,
    destination: &Root,
    files: &[FileSpec],
    directories: &[String],
    journal_directory: &Root,
    cancel: &AtomicBool,
    validate: &dyn Fn() -> Result<()>,
) -> Result<()> {
    if !destination.owned {
        return Err(failed(
            "Snapshot publication requires a newly created protected namespace.",
        ));
    }
    if files.len() > 10_000 {
        return Err(failed("Snapshot copy exceeds its 10,000-file bound."));
    }
    let prospective = screen_targets(
        &files
            .iter()
            .map(|file| file.target.clone())
            .collect::<Vec<_>>(),
        directories,
    )?;
    validate()?;
    let a = source.source()?;
    let b = destination.output()?;
    let data = journal_directory.output()?;
    if Object::of(&a)? == Object::of(b.leaf())? {
        return Err(failed("Snapshot input and output roots overlap."));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let mut seed = CopyPlan {
        id: id.clone(),
        digest: String::new(),
        source: Object::of(&a)?,
        destination: Object::of(b.leaf())?,
        source_volume: source.volume.clone(),
        destination_volume: destination.volume.clone(),
        files: Vec::new(),
    };
    seed.digest = seed.digest()?;
    let preparation = new_journal(
        &journal_directory.path.join(format!("prepare-{id}.sqlite")),
        &seed,
    )?;
    preparation.append(None,"snapshot_scope",serde_json::json!({"source":source::encode_path(&source.path),"destination":source::encode_path(&destination.path),"files":files.iter().map(|f|(&f.relative,&f.target,&f.identity,&f.revision,f.bytes,&f.digest,&f.metadata)).collect::<Vec<_>>()}))?;
    let stage_name = format!(".stillharbour-staging-{id}");
    preparation.append(
        None,
        "directory_intent",
        serde_json::json!({"relative":stage_name}),
    )?;
    let stage = native::create_directory(b.leaf(), &stage_name)?;
    preparation.append(
        None,
        "directory_ready",
        serde_json::json!({"relative":stage_name,"object":Object::of(&stage)?}),
    )?;
    let mut parents = BTreeMap::new();
    parents.insert(String::new(), b.leaf().try_clone()?);
    let mut directory_paths = prospective
        .iter()
        .filter(|(_, kind)| **kind == crate::backup::comparison::ComparisonEntryKind::Directory)
        .map(|(path, _)| {
            path.to_str()
                .map(str::to_string)
                .ok_or_else(|| failed("Unsupported prospective native directory."))
        })
        .collect::<Result<Vec<_>>>()?;
    directory_paths.sort_by_key(|path| path.split('/').count());
    for directory in directory_paths {
        let mut parent = String::new();
        let components = directory.split('/').collect::<Vec<_>>();
        for component in components {
            let relative = if parent.is_empty() {
                (*component).to_string()
            } else {
                format!("{parent}/{component}")
            };
            if !parents.contains_key(&relative) {
                preparation.append(
                    None,
                    "directory_intent",
                    serde_json::json!({"relative":relative}),
                )?;
                let held = parents
                    .get(&parent)
                    .ok_or_else(|| failed("Snapshot parent preparation is incomplete."))?;
                let created = native::create_directory(held, component)?;
                preparation.append(
                    None,
                    "directory_ready",
                    serde_json::json!({"relative":relative,"object":Object::of(&created)?}),
                )?;
                parents.insert(relative.clone(), created);
            }
            parent = relative;
        }
    }
    let mut probes = Vec::new();
    let mut objects = std::collections::BTreeSet::new();
    for (index, file) in files.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Err(failed(
                "Snapshot name preparation canceled; identified probes are retained.",
            ));
        }
        let (parent, leaf) = file.target.rsplit_once('/').unwrap_or(("", &file.target));
        let parent = parents
            .get(parent)
            .ok_or_else(|| failed("Prospective native parent is missing."))?;
        preparation.append(
            None,
            "name_probe_intent",
            serde_json::json!({"index":index,"target":file.target}),
        )?;
        let probe = native::create_file(parent, leaf)?;
        probe.sync_all()?;
        let identity = filesystem::identity(&probe)?;
        if identity.links != 1 || !objects.insert((identity.volume, identity.file)) {
            return Err(failed("Native prospective names are not distinct objects."));
        }
        let revision = source::revision(&probe)?;
        preparation.append(
            None,
            "name_probe_ready",
            serde_json::json!({"index":index,"identity":identity,"revision":revision}),
        )?;
        probes.push((identity, revision));
    }
    for (index, file) in files.iter().enumerate() {
        let (parent, leaf) = file.target.rsplit_once('/').unwrap_or(("", &file.target));
        let parent = parents
            .get(parent)
            .ok_or_else(|| failed("Native prospective parent disappeared."))?;
        let probe = native::read(parent, leaf)?;
        if filesystem::identity(&probe)? != probes[index].0
            || source::revision(&probe)? != probes[index].1
            || probe.metadata()?.len() != 0
        {
            return Err(failed(
                "A prospective name changed before complete-set native proof.",
            ));
        }
    }
    preparation.append(
        None,
        "names_proven",
        serde_json::json!({"files":files.len(),"directories":directories.len()}),
    )?;
    let mut plan = seed.clone();
    for (index, file) in files.iter().enumerate() {
        let relative = source::decode_path(&file.relative)?;
        let location = locations::Source::capture(&a, &relative)?;
        let original = location.open(&a)?;
        if filesystem::identity(&original)? != file.identity
            || source::revision(&original)? != file.revision
            || original.metadata()?.len() != file.bytes
        {
            return Err(failed("A reviewed snapshot source version changed."));
        }
        let (parent, leaf) = file
            .target
            .rsplit_once('/')
            .map_or((None, file.target.as_str()), |(p, l)| (Some(p), l));
        let held = parents
            .get(parent.unwrap_or(""))
            .ok_or_else(|| failed("Native prospective parent disappeared."))?;
        let stage_leaf = format!("{}.partial", uuid::Uuid::new_v4());
        let expected = &probes[index].0;
        let probe = native::reopen_stage(held, leaf, expected)?;
        if probe.metadata()?.len() != 0 || source::revision(&probe)? != probes[index].1 {
            return Err(failed("A native name probe changed before staging."));
        }
        preparation.append(
            None,
            "probe_stage_intent",
            serde_json::json!({"index":index,"stage":stage_leaf,"identity":expected}),
        )?;
        let staged = native::publish_to(held, leaf, probe, &stage, &stage_leaf, expected)?;
        let prepared = PreparedStage {
            object: Object::of(&staged)?,
            revision: source::revision(&staged)?,
        };
        preparation.append(None,"probe_staged",serde_json::json!({"index":index,"stage":stage_leaf,"identity":filesystem::identity(&staged)?}))?;
        plan.files.push(CopyFile {
            source: "backend native snapshot source".into(),
            target: leaf.into(),
            stage: stage_leaf,
            object: Object::of(&original)?,
            revision: file.revision.clone(),
            bytes: file.bytes,
            source_location: Some(location),
            destination_location: parent
                .map(|p| locations::Destination::capture(b.leaf(), p))
                .transpose()?,
            stage_location: Some(locations::Destination::capture(b.leaf(), &stage_name)?),
            prepared_stage: Some(prepared),
        });
    }
    plan.digest = plan.digest()?;
    preparation.append(
        None,
        "copy_ready",
        serde_json::json!({"digest":plan.digest}),
    )?;
    let journal = new_journal(
        &journal_directory.path.join(format!("copy-{id}.sqlite")),
        &plan,
    )?;
    let scope = || {
        validate()?;
        source.source()?;
        destination.output()?;
        journal_directory.output()?;
        Ok(())
    };
    let outcome = CopyContext {
        plan: &plan,
        journal: &journal,
        source_root: &a,
        destination: b.leaf(),
        io: &PacedIo::shared(),
        cancel,
        validate_scope: &scope,
    }
    .execute(None)?;
    if outcome != CopyOutcome::CompletedVerified {
        return Err(failed(
            "Snapshot staging/restoration stopped with retained partial copies.",
        ));
    }
    for file in files {
        let relative = Path::new(&file.target);
        let mut current = source::open_asset(
            &destination.path,
            &destination.identity,
            &destination.volume,
            relative,
        )?;
        if digest(&mut current, cancel)? != file.digest {
            return Err(failed(
                "Snapshot output differs from the exact reviewed content digest.",
            ));
        }
        if let Some(metadata) = &file.metadata {
            preparation.append(None,"metadata_intent",serde_json::json!({"target":file.target,"identity":filesystem::identity(&current)?,"metadata_digest":metadata.digest()?}))?;
            let identity = filesystem::identity(&current)?;
            drop(current);
            apply_metadata(destination, relative, metadata, &identity)?;
            preparation.append(
                None,
                "metadata_verified",
                serde_json::json!({"target":file.target,"metadata_digest":metadata.digest()?}),
            )?;
        }
    }
    let current_data = journal_directory.output()?;
    if Object::of(data.leaf())? != Object::of(current_data.leaf())? {
        return Err(failed("Snapshot journal directory changed."));
    }
    validate()?;
    preparation.append(
        None,
        "copy_completed",
        serde_json::json!({"files":files.len(),"fullBytesVerified":true,"sourcesRetained":true}),
    )?;
    Ok(())
}
pub(crate) fn screen_targets(
    files: &[String],
    directories: &[String],
) -> Result<BTreeMap<PathBuf, crate::backup::comparison::ComparisonEntryKind>> {
    use crate::backup::comparison::ComparisonEntryKind as Kind;
    if files.len() > 10_000 || directories.len() > 10_000 {
        return Err(failed(
            "The complete prospective snapshot name set exceeds its bound.",
        ));
    }
    let mut paths = BTreeMap::new();
    for (path, kind) in directories
        .iter()
        .map(|p| (p, Kind::Directory))
        .chain(files.iter().map(|p| (p, Kind::File)))
    {
        if !crate::organization::portable_relative_target(path) {
            return Err(failed(
                "A prospective snapshot path exceeds the portable native naming policy.",
            ));
        }
        let path = PathBuf::from(path);
        if paths
            .insert(path.clone(), kind)
            .is_some_and(|prior| prior != kind || kind == Kind::File)
        {
            return Err(failed(
                "Duplicate or conflicting prospective snapshot object types.",
            ));
        }
        for parent in path
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
        {
            if paths.insert(parent.into(), Kind::Directory) == Some(Kind::File) {
                return Err(failed(
                    "A prospective snapshot file is an ancestor of another object.",
                ));
            }
        }
    }
    if !crate::backup::names::blocks(&paths, &BTreeMap::new()).is_empty() {
        return Err(failed(
            "Prospective snapshot file/directory names have a case, Unicode or ancestor/type conflict; no payload copy is authorized.",
        ));
    }
    Ok(paths)
}
/// Restic outputs remain inside a fresh owned workspace. Apply captured metadata
/// only after a complete byte/tree comparison, with durable per-file intents.
pub(crate) fn restore_metadata(
    output: &Root,
    files: &[FileSpec],
    journal_directory: &Root,
    cancel: &AtomicBool,
) -> Result<()> {
    if !output.owned || !journal_directory.owned || files.len() > 10_000 {
        return Err(failed(
            "Native metadata restoration requires a bounded newly owned namespace.",
        ));
    }
    let root = output.source()?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut plan = CopyPlan {
        id: id.clone(),
        digest: String::new(),
        source: Object::of(&root)?,
        destination: Object::of(&root)?,
        source_volume: output.volume.clone(),
        destination_volume: output.volume.clone(),
        files: Vec::new(),
    };
    plan.digest = plan.digest()?;
    let journal = new_journal(
        &journal_directory.path.join(format!("metadata-{id}.sqlite")),
        &plan,
    )?;
    for file in files {
        let Some(metadata) = &file.metadata else {
            continue;
        };
        let relative = Path::new(&file.target);
        let current = source::open_asset(&output.path, &output.identity, &output.volume, relative)?;
        let identity = filesystem::identity(&current)?;
        journal.append(None,"metadata_intent",serde_json::json!({"relative":file.target,"identity":identity,"full_digest":file.digest,"metadata_digest":metadata.digest()?}))?;
        drop(current);
        apply_metadata(output, relative, metadata, &identity)?;
        let mut current =
            source::open_asset(&output.path, &output.identity, &output.volume, relative)?;
        if current.metadata()?.len() != file.bytes || digest(&mut current, cancel)? != file.digest {
            return Err(failed(
                "A newly restored output changed bytes during native metadata restoration.",
            ));
        }
        metadata.compare(&current)?;
        let reopened =
            source::open_asset(&output.path, &output.identity, &output.volume, relative)?;
        if filesystem::identity(&reopened)? != identity {
            return Err(failed(
                "Native metadata restoration changed output membership.",
            ));
        }
        journal.append(
            None,
            "metadata_verified",
            serde_json::json!({"relative":file.target,"metadata_digest":metadata.digest()?}),
        )?;
    }
    output.source()?;
    journal_directory.output()?;
    Ok(())
}
