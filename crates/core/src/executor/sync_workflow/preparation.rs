//! Reviewed native destination preparation. Every future name is held as an
//! exclusive empty probe until the whole native name set is proven distinct.
use super::*;
use std::collections::BTreeSet;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryFact {
    path: String,
    object: Object,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeFact {
    index: usize,
    target: String,
    object: Object,
    revision: String,
}
#[derive(Default)]
struct Facts {
    stage_intent: bool,
    stage: Option<Object>,
    dir_intents: BTreeSet<String>,
    dirs: BTreeMap<String, Object>,
    intents: BTreeMap<usize, String>,
    probes: BTreeMap<usize, ProbeFact>,
    moves: BTreeSet<usize>,
    staged: BTreeMap<usize, ProbeFact>,
    names_proven: bool,
}
fn selected(scope: &Scope) -> Vec<&Observed> {
    let mut files: Vec<_> = scope.sources.iter().filter(|f| f.copy).collect();
    files.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    files
}
fn target(scope: &Scope, observed: &Observed) -> Result<(String, String)> {
    let namespace = relative(&source::decode_path(&scope.namespace)?)?;
    let path = relative(&source::decode_path(&observed.path)?)?;
    Ok(path
        .rsplit_once('/')
        .map_or((namespace.clone(), path.clone()), |(p, n)| {
            (format!("{namespace}/{p}"), n.into())
        }))
}
pub(super) fn stage_name(id: &str) -> String {
    format!(".stillharbour-staging-{id}")
}
fn stage_leaf(id: &str, index: usize) -> String {
    format!("{id}-{index}.partial")
}
pub(super) fn validate_targets(scope: &Scope) -> Result<()> {
    let mut directories = BTreeSet::new();
    let source_handle = source::open_root(
        &source::decode_path(&scope.source)?,
        Some(&scope.source_identity),
    )?;
    for file in selected(scope) {
        let raw = source::decode_path(&file.path)?;
        let location = locations::Source::capture(&source_handle, &raw)?;
        let native = location.open(&source_handle)?;
        if filesystem::identity(&native)? != file.identity
            || source::revision(&native)? != file.revision
        {
            return Err(failed("A source changed during native target preflight."));
        }
        let (parent, _) = target(scope, file)?;
        let mut prefix = String::new();
        for name in parent.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(name);
            directories.insert(prefix.clone());
        }
    }
    if directories.len() > 1024 {
        return Err(invalid(
            "This append scope exceeds the 1024-directory native preparation bound. No names were created.",
        ));
    }
    Ok(())
}
fn value<T: Serialize>(fact: &T) -> Result<serde_json::Value> {
    serde_json::to_value(fact)
        .map_err(|e| failed("Could not encode native preparation fact.").caused_by(e))
}
fn parse<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T> {
    serde_json::from_value(value)
        .map_err(|e| failed("Invalid native preparation fact.").caused_by(e))
}
pub(super) fn canceled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(AppError::new(
            ErrorCode::Busy,
            "Native preparation canceled with all objects retained. Inspect recovery before continuing.",
        ))
    } else {
        Ok(())
    }
}

pub(super) fn capture_parents(root: &File, namespace: &str) -> Result<Vec<(String, Object)>> {
    let mut parent = root.try_clone()?;
    let mut prefix = String::new();
    let mut result = Vec::new();
    for name in namespace.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(name);
        match native::lookup(&parent, name)? {
            Some(entry) if entry.kind == native::EntryKind::Directory => {
                parent = native::read_directory(&parent, name)?;
                result.push((prefix.clone(), entry.object));
            }
            Some(_) => {
                return Err(invalid(
                    "An incompatible object occupies the destination namespace.",
                ));
            }
            None => break,
        }
    }
    Ok(result)
}
pub(super) fn validate_parents(destination: &Path, expected: &[(String, Object)]) -> Result<()> {
    let chain = output_chain(destination)?;
    for (path, object) in expected {
        let current = open_directory(chain.leaf(), path)?
            .ok_or_else(|| failed("A reviewed namespace ancestor disappeared."))?;
        if !object.matches(&current)? {
            return Err(failed(
                "A reviewed namespace ancestor changed native identity.",
            ));
        }
    }
    Ok(())
}
fn open_directory(root: &File, path: &str) -> Result<Option<File>> {
    let mut parent = root.try_clone()?;
    for name in path.split('/') {
        match native::lookup(&parent, name)? {
            None => return Ok(None),
            Some(entry) if entry.kind == native::EntryKind::Directory => {
                parent = native::read_directory(&parent, name)?
            }
            Some(_) => {
                return Err(failed(
                    "A namespace directory became an incompatible object.",
                ));
            }
        }
    }
    Ok(Some(parent))
}

fn facts(workflow: &SyncWorkflow, id: &str, binding: &Binding) -> Result<Facts> {
    let files = selected(&binding.scope);
    let mut state = Facts::default();
    let namespace = relative(&source::decode_path(&binding.scope.namespace)?)?;
    for (item, phase, evidence) in workflow.events(id)? {
        if phase.starts_with("copy/") {
            continue;
        }
        if item.is_some() {
            return Err(failed(
                "Preparation event has an unexpected copy item index.",
            ));
        }
        match phase.as_str() {
            "prepare/stage_intent" => {
                if state.stage_intent || evidence != serde_json::json!({"path":stage_name(id)}) {
                    return Err(failed("Invalid/repeated native staging intent."));
                }
                state.stage_intent = true;
            }
            "prepare/stage_ready" => {
                let fact: DirectoryFact = parse(evidence)?;
                if !state.stage_intent || state.stage.is_some() || fact.path != stage_name(id) {
                    return Err(failed("Invalid native staging completion."));
                }
                state.stage = Some(fact.object);
            }
            "prepare/directory_intent" => {
                let path = evidence
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| failed("Missing native directory path."))?
                    .to_owned();
                if !state.stage_intent
                    || state.names_proven
                    || !safe_parent(&namespace, &path, &files, &binding.scope)?
                    || !state.dir_intents.insert(path)
                {
                    return Err(failed("Invalid native directory intent."));
                }
            }
            "prepare/directory_ready" => {
                let fact: DirectoryFact = parse(evidence)?;
                if state.names_proven
                    || !state.dir_intents.contains(&fact.path)
                    || state.dirs.insert(fact.path, fact.object).is_some()
                {
                    return Err(failed("Invalid native directory completion."));
                }
            }
            "prepare/probe_intent" => {
                let index = evidence
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .and_then(|v| usize::try_from(v).ok())
                    .ok_or_else(|| failed("Invalid probe index."))?;
                let observed = files
                    .get(index)
                    .ok_or_else(|| failed("Foreign native name probe."))?;
                let (parent, leaf) = target(&binding.scope, observed)?;
                let path = format!("{parent}/{leaf}");
                if state.names_proven
                    || evidence != serde_json::json!({"index":index,"target":path})
                    || state.intents.insert(index, path).is_some()
                {
                    return Err(failed("Invalid/repeated native name probe intent."));
                }
            }
            "prepare/probe_ready" => {
                let fact: ProbeFact = parse(evidence)?;
                if state.names_proven
                    || state.intents.get(&fact.index) != Some(&fact.target)
                    || state.probes.insert(fact.index, fact).is_some()
                {
                    return Err(failed("Invalid native name probe identity."));
                }
            }
            "prepare/names_proven" => {
                if state.names_proven
                    || state.probes.len() != files.len()
                    || evidence != serde_json::json!({"files":files.len()})
                {
                    return Err(failed("Incomplete/repeated whole native name proof."));
                }
                state.names_proven = true;
            }
            "prepare/probe_move_intent" => {
                let index = evidence
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .and_then(|v| usize::try_from(v).ok())
                    .ok_or_else(|| failed("Invalid staged probe index."))?;
                if !state.names_proven
                    || !state.probes.contains_key(&index)
                    || evidence != serde_json::json!({"index":index,"stage":stage_leaf(id,index)})
                    || !state.moves.insert(index)
                {
                    return Err(failed("Invalid native probe move intent."));
                }
            }
            "prepare/probe_staged" => {
                let fact: ProbeFact = parse(evidence)?;
                let original = state
                    .probes
                    .get(&fact.index)
                    .ok_or_else(|| failed("Staged probe has no original identity."))?;
                if !state.moves.contains(&fact.index)
                    || fact.target != stage_leaf(id, fact.index)
                    || fact.object != original.object
                    || state.staged.insert(fact.index, fact).is_some()
                {
                    return Err(failed("Invalid staged native probe completion."));
                }
            }
            _ => {
                return Err(failed(
                    "Unknown preparation journal phase; retain all objects.",
                ));
            }
        }
    }
    Ok(state)
}
fn safe_parent(namespace: &str, path: &str, files: &[&Observed], scope: &Scope) -> Result<bool> {
    if namespace == path || namespace.starts_with(&format!("{path}/")) {
        return Ok(true);
    }
    for file in files {
        let (parent, _) = target(scope, file)?;
        if parent == path || parent.starts_with(&format!("{path}/")) {
            return Ok(true);
        }
    }
    Ok(false)
}
fn ensure_directory(
    workflow: &SyncWorkflow,
    id: &str,
    binding: &Binding,
    root: &File,
    path: &str,
    facts: &mut Facts,
    cancel: &AtomicBool,
) -> Result<File> {
    let namespace = relative(&source::decode_path(&binding.scope.namespace)?)?;
    let mut parent = root.try_clone()?;
    let mut prefix = String::new();
    for name in path.split('/') {
        canceled(cancel)?;
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(name);
        let original = binding
            .parents
            .iter()
            .find(|(p, _)| p == &prefix)
            .map(|(_, o)| o.clone())
            .or_else(|| {
                let relative = if prefix == namespace {
                    Some("")
                } else {
                    prefix.strip_prefix(&format!("{namespace}/"))
                };
                relative.and_then(|relative| {
                    binding
                        .scope
                        .destination_directories
                        .iter()
                        .find(|(p, _)| {
                            source::decode_path(p).is_ok_and(|p| p == Path::new(relative))
                        })
                        .map(|(_, i)| Object {
                            volume: i.volume,
                            file: i.file,
                        })
                })
            });
        let expected = facts.dirs.get(&prefix).cloned().or(original);
        match native::lookup(&parent, name)? {
            Some(entry)
                if entry.kind == native::EntryKind::Directory
                    && expected.as_ref() == Some(&entry.object) =>
            {
                parent = native::read_directory(&parent, name)?;
            }
            Some(_) => {
                return Err(failed(
                    "An unreviewed/uncertain destination ancestor is retained. Recompare or inspect its ownership; it cannot be adopted automatically.",
                ));
            }
            None if expected.is_some() => {
                return Err(failed(
                    "A recorded native directory disappeared; it will not be recreated blindly.",
                ));
            }
            None => {
                if !facts.dir_intents.contains(&prefix) {
                    workflow.event(
                        id,
                        None,
                        "prepare/directory_intent",
                        serde_json::json!({"path":prefix}),
                    )?;
                    facts.dir_intents.insert(prefix.clone());
                }
                parent = native::create_directory(&parent, name)?;
                let object = Object::of(&parent)?;
                workflow.event(
                    id,
                    None,
                    "prepare/directory_ready",
                    value(&DirectoryFact {
                        path: prefix.clone(),
                        object: object.clone(),
                    })?,
                )?;
                facts.dirs.insert(prefix.clone(), object);
            }
        }
    }
    Ok(parent)
}

pub(super) fn prepare(
    workflow: &SyncWorkflow,
    library: &Library,
    id: &str,
    binding: &Binding,
    cancel: &AtomicBool,
) -> Result<CopyPlan> {
    let mut facts = facts(workflow, id, binding)?;
    let src = source::decode_path(&binding.scope.source)?;
    let dst = source::decode_path(&binding.scope.destination)?;
    let source_handle = source::open_root(&src, Some(&binding.scope.source_identity))?;
    let destination = output_chain(&dst)?;
    let stage_path = stage_name(id);
    if !facts.stage_intent {
        workflow.event(
            id,
            None,
            "prepare/stage_intent",
            serde_json::json!({"path":stage_path}),
        )?;
        facts.stage_intent = true;
    }
    let stage = match (
        &facts.stage,
        native::lookup(destination.leaf(), &stage_path)?,
    ) {
        (Some(expected), Some(entry))
            if entry.kind == native::EntryKind::Directory && entry.object == *expected =>
        {
            native::read_directory(destination.leaf(), &stage_path)?
        }
        (None, None) => {
            let stage = native::create_directory(destination.leaf(), &stage_path)?;
            #[cfg(test)]
            workflow.boundary(Fault::StageCreated)?;
            let object = Object::of(&stage)?;
            workflow.event(
                id,
                None,
                "prepare/stage_ready",
                value(&DirectoryFact {
                    path: stage_path.clone(),
                    object: object.clone(),
                })?,
            )?;
            facts.stage = Some(object);
            stage
        }
        _ => {
            return Err(failed(
                "Staging directory ownership is uncertain or changed; retain it and inspect rather than adopt/recreate it.",
            ));
        }
    };
    let selected = selected(&binding.scope);
    if selected.is_empty() || selected.len() > 10_000 {
        return Err(invalid(
            "Native preparation needs 1–10,000 reviewed new files.",
        ));
    }
    // Keep all exclusive probes in their actual future parents until the whole
    // set has been proved, including adapter-specific equivalence/collisions.
    for (index, observed) in selected.iter().enumerate() {
        canceled(cancel)?;
        library.validate_backup_sync_scope(&binding.scope)?;
        let (relative_parent, leaf) = target(&binding.scope, observed)?;
        let parent = ensure_directory(
            workflow,
            id,
            binding,
            destination.leaf(),
            &relative_parent,
            &mut facts,
            cancel,
        )?;
        if facts.probes.contains_key(&index) {
            continue;
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = facts.intents.entry(index) {
            workflow.event(
                id,
                None,
                "prepare/probe_intent",
                serde_json::json!({"index":index,"target":format!("{relative_parent}/{leaf}")}),
            )?;
            entry.insert(format!("{relative_parent}/{leaf}"));
        }
        if native::lookup(&parent, &leaf)?.is_some()
            || native::lookup(&stage, &stage_leaf(id, index))?.is_some()
        {
            return Err(failed(
                "An unrecorded target/stage occupant is retained; native preparation cannot infer its ownership.",
            ));
        }
        let probe = native::create_file(&parent, &leaf)?;
        #[cfg(test)]
        workflow.boundary(Fault::ProbeCreated)?;
        let fact = ProbeFact {
            index,
            target: format!("{relative_parent}/{leaf}"),
            object: Object::of(&probe)?,
            revision: source::revision(&probe)?,
        };
        workflow.event(id, None, "prepare/probe_ready", value(&fact)?)?;
        facts.probes.insert(index, fact);
    }
    if !facts.names_proven {
        for (index, observed) in selected.iter().enumerate() {
            let (parent, leaf) = target(&binding.scope, observed)?;
            let parent = open_directory(destination.leaf(), &parent)?
                .ok_or_else(|| failed("Prepared parent disappeared."))?;
            let expected = facts
                .probes
                .get(&index)
                .ok_or_else(|| failed("Incomplete native name proof."))?;
            let file = native::read(&parent, &leaf)?;
            if !expected.object.matches(&file)?
                || file.metadata()?.len() != 0
                || source::revision(&file)? != expected.revision
            {
                return Err(failed(
                    "A prospective name probe changed before complete-set proof.",
                ));
            }
        }
        workflow.event(
            id,
            None,
            "prepare/names_proven",
            serde_json::json!({"files":selected.len()}),
        )?;
        facts.names_proven = true;
        #[cfg(test)]
        workflow.boundary(Fault::NamesProven)?;
    }
    let mut files = Vec::with_capacity(selected.len());
    for (index, observed) in selected.into_iter().enumerate() {
        canceled(cancel)?;
        library.validate_backup_sync_scope(&binding.scope)?;
        let raw = source::decode_path(&observed.path)?;
        let (relative_parent, leaf) = target(&binding.scope, observed)?;
        let parent = open_directory(destination.leaf(), &relative_parent)?
            .ok_or_else(|| failed("Prepared target parent disappeared."))?;
        let original = facts
            .probes
            .get(&index)
            .ok_or_else(|| failed("A native name probe has no identity."))?
            .clone();
        let stage_leaf = stage_leaf(id, index);
        let staged = if let Some(fact) = facts.staged.get(&index) {
            let file = native::reopen_stage(
                &stage,
                &stage_leaf,
                &filesystem::FileIdentity {
                    volume: fact.object.volume,
                    file: fact.object.file,
                    links: 1,
                },
            )?;
            if file.metadata()?.len() != 0
                || source::revision(&file)? != fact.revision
                || native::lookup(&parent, &leaf)?.is_some()
            {
                return Err(failed("An identified prepared stage/target changed."));
            }
            file
        } else {
            if !facts.moves.contains(&index) {
                workflow.event(
                    id,
                    None,
                    "prepare/probe_move_intent",
                    serde_json::json!({"index":index,"stage":stage_leaf}),
                )?;
                facts.moves.insert(index);
            }
            let target = native::lookup(&parent, &leaf)?;
            let staged = native::lookup(&stage, &stage_leaf)?;
            let file = match (target, staged) {
                (Some(entry), None)
                    if entry.kind == native::EntryKind::File && entry.object == original.object =>
                {
                    let probe = native::reopen_stage(
                        &parent,
                        &leaf,
                        &filesystem::FileIdentity {
                            volume: original.object.volume,
                            file: original.object.file,
                            links: 1,
                        },
                    )?;
                    if probe.metadata()?.len() != 0
                        || source::revision(&probe)? != original.revision
                    {
                        return Err(failed("A native name probe changed before staging."));
                    }
                    let moved = native::publish_to(
                        &parent,
                        &leaf,
                        probe,
                        &stage,
                        &stage_leaf,
                        &filesystem::FileIdentity {
                            volume: original.object.volume,
                            file: original.object.file,
                            links: 1,
                        },
                    )?;
                    #[cfg(test)]
                    workflow.boundary(Fault::ProbeMoved)?;
                    moved
                }
                (None, Some(entry))
                    if entry.kind == native::EntryKind::File && entry.object == original.object =>
                {
                    native::reopen_stage(
                        &stage,
                        &stage_leaf,
                        &filesystem::FileIdentity {
                            volume: original.object.volume,
                            file: original.object.file,
                            links: 1,
                        },
                    )?
                }
                _ => {
                    return Err(failed(
                        "The native probe move is uncertain; retain both locations and inspect ownership.",
                    ));
                }
            };
            if file.metadata()?.len() != 0 {
                return Err(failed(
                    "A staged native name probe contains unexpected data.",
                ));
            }
            let fact = ProbeFact {
                index,
                target: stage_leaf.clone(),
                object: Object::of(&file)?,
                revision: source::revision(&file)?,
            };
            workflow.event(id, None, "prepare/probe_staged", value(&fact)?)?;
            facts.staged.insert(index, fact);
            file
        };
        let location = locations::Source::capture(&source_handle, &raw)?;
        let source_file = location.open(&source_handle)?;
        if filesystem::identity(&source_file)? != observed.identity
            || source::revision(&source_file)? != observed.revision
            || source_file.metadata()?.len() != observed.bytes
        {
            return Err(failed(
                "A reviewed append source changed during preparation.",
            ));
        }
        files.push(CopyFile {
            source: raw.to_string_lossy().into_owned(),
            target: leaf,
            stage: stage_leaf,
            object: Object {
                volume: observed.identity.volume,
                file: observed.identity.file,
            },
            revision: observed.revision.clone(),
            bytes: observed.bytes,
            source_location: Some(location),
            destination_location: Some(locations::Destination::capture(
                destination.leaf(),
                &relative_parent,
            )?),
            stage_location: Some(locations::Destination::capture(
                destination.leaf(),
                &stage_path,
            )?),
            prepared_stage: Some(PreparedStage {
                object: Object::of(&staged)?,
                revision: source::revision(&staged)?,
            }),
        });
    }
    library.validate_backup_sync_scope(&binding.scope)?;
    let mut plan = CopyPlan {
        id: id.into(),
        digest: String::new(),
        source: Object::of(&source_handle)?,
        destination: Object::of(destination.leaf())?,
        source_volume: binding.scope.source_volume.clone(),
        destination_volume: binding.scope.destination_volume.clone(),
        files,
    };
    plan.digest = plan.digest()?;
    Ok(plan)
}

pub(super) fn validate_plan(
    workflow: &SyncWorkflow,
    id: &str,
    binding: &Binding,
    plan: &CopyPlan,
) -> Result<()> {
    let src = source::decode_path(&binding.scope.source)?;
    let dst = source::decode_path(&binding.scope.destination)?;
    let a = source::open_root(&src, Some(&binding.scope.source_identity))?;
    let b = output_chain(&dst)?;
    let facts = facts(workflow, id, binding)?;
    let selected = selected(&binding.scope);
    if plan.id != id
        || plan.digest()? != plan.digest
        || !plan.source.matches(&a)?
        || !plan.destination.matches(b.leaf())?
        || plan.source_volume != binding.scope.source_volume
        || plan.destination_volume != binding.scope.destination_volume
        || selected.len() != plan.files.len()
        || !facts.names_proven
        || facts.staged.len() != selected.len()
    {
        return Err(failed(
            "Copy plan differs from its complete approved/native-prepared append scope.",
        ));
    }
    let staging = locations::Destination::capture(b.leaf(), &stage_name(id))?;
    for (index, (observed, file)) in selected.into_iter().zip(&plan.files).enumerate() {
        let raw = source::decode_path(&observed.path)?;
        let (parent, leaf) = target(&binding.scope, observed)?;
        let location = locations::Source::capture(&a, &raw)?;
        let destination = locations::Destination::capture(b.leaf(), &parent)?;
        let fact = facts
            .staged
            .get(&index)
            .ok_or_else(|| failed("Missing staged name-proof identity."))?;
        if file.object
            != (Object {
                volume: observed.identity.volume,
                file: observed.identity.file,
            })
            || file.revision != observed.revision
            || file.bytes != observed.bytes
            || file.target != leaf
            || file.stage != stage_leaf(id, index)
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
                != Some(encode(&staging)?.as_str())
            || file
                .prepared_stage
                .as_ref()
                .is_none_or(|p| p.object != fact.object || p.revision != fact.revision)
        {
            return Err(failed(
                "A retained append locator/action differs from the approved exact native name proof.",
            ));
        }
    }
    Ok(())
}

pub(super) fn review_incomplete(
    workflow: &SyncWorkflow,
    library: &Library,
    id: &str,
    binding: Binding,
) -> Result<SyncReview> {
    library.validate_backup_sync_scope(&binding.scope)?;
    let facts = facts(workflow, id, &binding)?;
    let dst = source::decode_path(&binding.scope.destination)?;
    let chain = output_chain(&dst)?;
    let mut uncertain = false;
    if facts.stage_intent
        && facts.stage.is_none()
        && native::lookup(chain.leaf(), &stage_name(id))?.is_some()
    {
        uncertain = true;
    }
    for path in &facts.dir_intents {
        if !facts.dirs.contains_key(path) && open_directory(chain.leaf(), path)?.is_some() {
            uncertain = true;
        }
    }
    for (index, path) in &facts.intents {
        if !facts.probes.contains_key(index)
            && let Some((parent, leaf)) = path.rsplit_once('/')
            && let Some(parent) = open_directory(chain.leaf(), parent)?
            && native::lookup(&parent, leaf)?.is_some()
        {
            uncertain = true;
        }
    }
    let mut view = binding.view.clone();
    view.id = uuid::Uuid::new_v4().to_string();
    view.job_id = Some(id.into());
    view.action = SyncAction::PrepareNames;
    view.digest = blake3::hash(encode(&(&binding, &workflow.events(id)?))?.as_bytes())
        .to_hex()
        .to_string();
    view.execution_available = SyncWorkflow::capabilities()?.execution_available && !uncertain;
    view.reason = if uncertain {
        "Preparation has an intent without an identified completion and an unexpected occupant. Keep all objects. Automatic adoption/removal is forbidden; inspect the namespace and create a new comparison/plan after a specific remedy.".into()
    } else {
        "Preparation journal and native ownership can be revalidated. This fresh approval resumes name preparation only, retaining every partial object; copying still needs its own later approval.".into()
    };
    workflow.retain_review(Review {
        binding,
        view,
        stage: Stage::Prepare,
        created: Instant::now(),
    })
}
