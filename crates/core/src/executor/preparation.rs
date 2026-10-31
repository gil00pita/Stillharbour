//! Native future-name evidence in a newly owned pending output namespace.
//! No normalization/lowercasing is used to authorize distinct target names.
use super::{CopyFixture, Journal, Object, failed, native};
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem,
    operations::MAX_PLAN_BYTES,
    organization::portable_relative_target,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_DIRECTORIES: usize = 1024;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Boundary {
    RootCreated,
    DirectoryCreated,
    LeafCreated,
    StageMoved,
}
#[cfg(test)]
thread_local! {static FAULT:std::cell::Cell<Option<Boundary>>=const{std::cell::Cell::new(None)};}
fn boundary(point: Boundary) -> Result<()> {
    #[cfg(test)]
    if FAULT.with(|fault| fault.get() == Some(point)) {
        return Err(AppError::new(
            ErrorCode::WorkerFailed,
            "Injected owned namespace interruption before the next journal completion.",
        ));
    }
    let _ = point;
    Ok(())
}
#[derive(Clone, Serialize)]
struct Target {
    id: String,
    path: String,
}
#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
enum State {
    Prepared,
    NativeConflict,
    BlockedAncestor,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    id: String,
    target_relative_path: String,
    state: State,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    policy: String,
    temporary_namespace: String,
    fingerprint: String,
    rows: Vec<Row>,
    directories: usize,
    retained_stages: usize,
    execution_available: bool,
}
struct Directory {
    path: Vec<String>,
    handle: File,
    object: Object,
}
#[derive(Serialize)]
struct Stage {
    target_id: String,
    parent: usize,
    original_leaf: String,
    stage_leaf: String,
    object: Object,
    revision: String,
}
struct Placeholder {
    target: usize,
    parent: usize,
    leaf: String,
    identity: filesystem::FileIdentity,
    revision: String,
}
struct Preparation {
    report: Report,
    directories: Vec<Directory>,
    stages: Vec<Stage>,
}
fn validate_members(
    parent: &File,
    namespace: &str,
    directories: &[Directory],
    stages: &[Stage],
) -> Result<()> {
    for (index, node) in directories.iter().enumerate() {
        let (parent, leaf) = if index == 0 {
            (parent, namespace)
        } else {
            let prefix = &node.path[..node.path.len() - 1];
            let owner = directories
                .iter()
                .find(|candidate| candidate.path == prefix)
                .ok_or_else(|| failed("Missing prepared-directory parent."))?;
            (&owner.handle, node.path.last().unwrap().as_str())
        };
        let entry = native::lookup(parent, leaf)?
            .ok_or_else(|| failed("A prepared directory is missing."))?;
        if entry.kind != native::EntryKind::Directory
            || entry.object != node.object
            || !node.object.matches(&node.handle)?
        {
            return Err(failed(
                "A prepared directory changed membership or identity.",
            ));
        }
    }
    for (index, node) in directories.iter().enumerate() {
        let mut expected = Vec::new();
        for child in directories.iter().skip(1) {
            if child.path.len() == node.path.len() + 1 && child.path.starts_with(&node.path) {
                expected.push((child.object.clone(), native::EntryKind::Directory));
            }
        }
        for stage in stages.iter().filter(|stage| stage.parent == index) {
            expected.push((stage.object.clone(), native::EntryKind::File));
        }
        native::verify_entries(&node.handle, &expected)?;
    }
    for stage in stages {
        let parent = &directories[stage.parent].handle;
        let file = native::read(parent, &stage.stage_leaf)?;
        if !stage.object.matches(&file)?
            || filesystem::identity(&file)?.links != 1
            || file.metadata()?.len() != 0
            || crate::library::native::revision(&file)? != stage.revision
            || native::lookup(parent, &stage.original_leaf)?.is_some()
        {
            return Err(failed("A prepared stage or reserved target name changed."));
        }
    }
    Ok(())
}
fn input(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
fn canceled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        return Err(AppError::new(
            ErrorCode::Busy,
            "Destination preparation was canceled. Retain its journaled pending namespace.",
        ));
    }
    Ok(())
}

fn prepare(
    parent: &File,
    targets: &[Target],
    journal: &Journal,
    cancel: &AtomicBool,
) -> Result<Preparation> {
    if targets.is_empty() || targets.len() > 15_600 {
        return Err(input("Prepare between 1 and 15,600 proposed outputs."));
    }
    let mut ids = BTreeSet::new();
    for target in targets {
        if target.id.len() != 64
            || !target.id.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !ids.insert(target.id.clone())
            || !portable_relative_target(&target.path)
            || target
                .path
                .split('/')
                .any(|part| part.to_ascii_lowercase().ends_with(".photoslibrary"))
        {
            return Err(input("Invalid or repeated proposed output identity/path."));
        }
    }
    let encoded = serde_json::to_vec(targets)
        .map_err(|error| input("Could not encode target-name scope.").caused_by(error))?;
    if encoded.len() > MAX_PLAN_BYTES {
        return Err(input("The target-name scope exceeds 8 MiB."));
    }
    canceled(cancel)?;
    let token = uuid::Uuid::new_v4().to_string();
    let temporary_namespace = format!(".stillharbour-preparing-{token}.partial");
    let parent_object = Object::of(parent)?;
    journal.append(None,"namespace_prepare_intent",serde_json::json!({"policy":"native-name-preparation-v1","parent":parent_object,"temporaryNamespace":temporary_namespace,"targets":targets}))?;
    let root = native::create_directory(parent, &temporary_namespace)?;
    boundary(Boundary::RootCreated)?;
    let root_object = Object::of(&root)?;
    if root_object.volume != parent_object.volume {
        return Err(failed("The new namespace has an unexpected volume."));
    }
    journal.append(
        None,
        "namespace_created",
        serde_json::json!({"object":root_object,"temporaryNamespace":temporary_namespace}),
    )?;
    let mut directories = vec![Directory {
        path: Vec::new(),
        handle: root,
        object: root_object,
    }];
    let mut known = BTreeMap::from([(Vec::<String>::new(), 0usize)]);
    let mut file_objects: BTreeMap<(u64, u64), usize> = BTreeMap::new();
    let mut placeholders = Vec::new();
    let mut conflicted = BTreeSet::new();
    let mut blocked_prefixes = BTreeSet::new();
    let mut rows = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        canceled(cancel)?;
        let parts: Vec<String> = target.path.split('/').map(str::to_owned).collect();
        let mut parent_index = 0;
        let mut path = Vec::new();
        let mut blocked = false;
        for part in &parts[..parts.len() - 1] {
            canceled(cancel)?;
            path.push(part.clone());
            if blocked_prefixes
                .iter()
                .any(|prefix: &Vec<String>| path.starts_with(prefix))
            {
                blocked = true;
                break;
            }
            if let Some(&existing) = known.get(&path) {
                parent_index = existing;
                continue;
            }
            if directories.len() >= MAX_DIRECTORIES {
                return Err(input(
                    "Native target preparation exceeds 1,024 directories. Use a smaller selection.",
                ));
            }
            journal.append(
                Some(index),
                "namespace_directory_intent",
                serde_json::json!({"parent":directories[parent_index].object,"path":path}),
            )?;
            match native::create_directory(&directories[parent_index].handle, part) {
                Ok(handle) => {
                    boundary(Boundary::DirectoryCreated)?;
                    let object = Object::of(&handle)?;
                    if object.volume != parent_object.volume {
                        return Err(failed("An owned target directory changed volume."));
                    }
                    journal.append(
                        Some(index),
                        "namespace_directory_created",
                        serde_json::json!({"path":path,"object":object}),
                    )?;
                    let node = directories.len();
                    known.insert(path.clone(), node);
                    directories.push(Directory {
                        path: path.clone(),
                        handle,
                        object,
                    });
                    parent_index = node;
                }
                Err(error) => {
                    let Some(entry) = native::lookup(&directories[parent_index].handle, part)?
                    else {
                        return Err(error);
                    };
                    if entry.kind == native::EntryKind::Directory
                        && let Some(other) =
                            directories.iter().find(|node| node.object == entry.object)
                    {
                        blocked_prefixes.insert(other.path.clone());
                        blocked_prefixes.insert(path.clone());
                    } else if entry.kind == native::EntryKind::File
                        && let Some(&other) =
                            file_objects.get(&(entry.object.volume, entry.object.file))
                    {
                        conflicted.insert(other);
                        blocked_prefixes.insert(path.clone());
                    } else {
                        return Err(failed(
                            "An unknown or unsupported entry occupies the new namespace. Retain all objects for review.",
                        ));
                    }
                    blocked = true;
                    break;
                }
            }
        }
        let mut row = Row {
            id: target.id.clone(),
            target_relative_path: target.path.clone(),
            state: if blocked {
                State::BlockedAncestor
            } else {
                State::Prepared
            },
        };
        if !blocked {
            let leaf = parts
                .last()
                .ok_or_else(|| input("Missing target name."))?
                .clone();
            journal.append(
                Some(index),
                "namespace_leaf_intent",
                serde_json::json!({"parent":directories[parent_index].object,"leaf":leaf}),
            )?;
            match native::create_file(&directories[parent_index].handle, &leaf) {
                Ok(file) => {
                    boundary(Boundary::LeafCreated)?;
                    let identity = filesystem::identity(&file)?;
                    if identity.links != 1
                        || identity.volume != parent_object.volume
                        || file.metadata()?.len() != 0
                    {
                        return Err(failed("The owned name probe has unexpected file evidence."));
                    }
                    let revision = crate::library::native::revision(&file)?;
                    journal.append(Some(index),"namespace_leaf_created",serde_json::json!({"object":Object::of(&file)?,"revision":revision,"parent":directories[parent_index].object,"leaf":leaf}))?;
                    file_objects.insert((identity.volume, identity.file), index);
                    placeholders.push(Placeholder {
                        target: index,
                        parent: parent_index,
                        leaf,
                        identity,
                        revision,
                    });
                }
                Err(error) => {
                    let Some(entry) = native::lookup(&directories[parent_index].handle, &leaf)?
                    else {
                        return Err(error);
                    };
                    if entry.kind == native::EntryKind::File
                        && let Some(&other) =
                            file_objects.get(&(entry.object.volume, entry.object.file))
                    {
                        conflicted.insert(other);
                        conflicted.insert(index);
                        row.state = State::NativeConflict;
                    } else if entry.kind == native::EntryKind::Directory
                        && let Some(other) =
                            directories.iter().find(|node| node.object == entry.object)
                    {
                        blocked_prefixes.insert(other.path.clone());
                        row.state = State::NativeConflict;
                    } else {
                        return Err(failed(
                            "An unknown or unsupported entry occupies a proposed output. No existing entry was adopted.",
                        ));
                    }
                }
            }
        }
        rows.push(row);
    }
    // Mark every participant, including requests processed before an alias.
    for (index, row) in rows.iter_mut().enumerate() {
        let parts: Vec<String> = row
            .target_relative_path
            .split('/')
            .map(str::to_owned)
            .collect();
        if conflicted.contains(&index) {
            row.state = State::NativeConflict;
        }
        if blocked_prefixes
            .iter()
            .any(|prefix| parts.starts_with(prefix))
        {
            row.state = State::BlockedAncestor;
        }
    }
    let mut stages = Vec::new();
    for placeholder in placeholders {
        canceled(cancel)?;
        let parent = &directories[placeholder.parent].handle;
        let stage_leaf = format!(".stillharbour-stage-{token}-{}.partial", placeholder.target);
        journal.append(Some(placeholder.target),"namespace_stage_intent",serde_json::json!({"parent":directories[placeholder.parent].object,"from":placeholder.leaf,"to":stage_leaf}))?;
        let file = native::reopen_stage(parent, &placeholder.leaf, &placeholder.identity)?;
        if file.metadata()?.len() != 0
            || crate::library::native::revision(&file)? != placeholder.revision
        {
            return Err(failed("An owned name probe changed before staging."));
        }
        let staged = native::publish_new(
            parent,
            &placeholder.leaf,
            file,
            &stage_leaf,
            &placeholder.identity,
        )?;
        boundary(Boundary::StageMoved)?;
        let stage = Stage {
            target_id: targets[placeholder.target].id.clone(),
            parent: placeholder.parent,
            original_leaf: placeholder.leaf,
            stage_leaf,
            object: Object::of(&staged)?,
            revision: crate::library::native::revision(&staged)?,
        };
        journal.append(
            Some(placeholder.target),
            "namespace_stage_created",
            serde_json::to_value(&stage).map_err(|error| {
                input("Could not retain prepared-stage evidence.").caused_by(error)
            })?,
        )?;
        stages.push(stage);
    }
    if !parent_object.matches(parent)? {
        return Err(failed("The destination parent changed during preparation."));
    }
    for directory in &directories {
        if !directory.object.matches(&directory.handle)? {
            return Err(failed("An owned directory changed during preparation."));
        }
    }
    canceled(cancel)?;
    validate_members(parent, &temporary_namespace, &directories, &stages)?;
    let evidence = serde_json::to_vec(&(
        "native-name-preparation-v1",
        &parent_object,
        &temporary_namespace,
        &rows,
        &stages,
        directories
            .iter()
            .map(|node| (&node.path, &node.object))
            .collect::<Vec<_>>(),
    ))
    .map_err(|error| input("Could not fingerprint prepared target evidence.").caused_by(error))?;
    if evidence.len() > MAX_PLAN_BYTES {
        return Err(input(
            "The prepared namespace exceeds its retained 8-MiB evidence bound.",
        ));
    }
    let report = Report {
        policy: "native-name-preparation-v1".into(),
        temporary_namespace,
        fingerprint: blake3::hash(&evidence).to_hex().to_string(),
        rows,
        directories: directories.len(),
        retained_stages: stages.len(),
        execution_available: false,
    };
    journal.append(None,"namespace_prepared",serde_json::json!({"report":report,"evidence":serde_json::from_slice::<serde_json::Value>(&evidence).map_err(|error|input("Could not retain prepared evidence.").caused_by(error))?}))?;
    Ok(Preparation {
        report,
        directories,
        stages,
    })
}

fn targets(paths: &[&str]) -> Vec<Target> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| Target {
            id: blake3::hash(format!("{index}:{path}").as_bytes())
                .to_hex()
                .to_string(),
            path: (*path).into(),
        })
        .collect()
}
fn prepared_copy_plan(fixture: &CopyFixture, prepared: &Preparation) -> Result<super::CopyPlan> {
    let mut files = Vec::new();
    for source in &fixture.plan.files {
        let path = format!("2026/Trip/{}", source.target);
        let row = prepared
            .report
            .rows
            .iter()
            .find(|row| row.target_relative_path == path && row.state == State::Prepared)
            .ok_or_else(|| failed("Missing prepared file row."))?;
        let stage = prepared
            .stages
            .iter()
            .find(|stage| stage.target_id == row.id)
            .ok_or_else(|| failed("Missing identified empty stage."))?;
        let relative_parent = prepared.directories[stage.parent].path.join("/");
        let mut file = source.clone();
        file.target = stage.original_leaf.clone();
        file.stage = stage.stage_leaf.clone();
        file.destination_location = if relative_parent.is_empty() {
            None
        } else {
            Some(super::locations::Destination::capture(
                &prepared.directories[0].handle,
                &relative_parent,
            )?)
        };
        file.prepared_stage = Some(super::PreparedStage {
            object: stage.object.clone(),
            revision: stage.revision.clone(),
        });
        files.push(file);
    }
    let mut plan = super::CopyPlan {
        id: uuid::Uuid::new_v4().to_string(),
        digest: String::new(),
        source: fixture.plan.source.clone(),
        destination: prepared.directories[0].object.clone(),
        source_volume: fixture.plan.source_volume.clone(),
        destination_volume: fixture.plan.destination_volume.clone(),
        files,
    };
    plan.digest = plan.digest()?;
    Ok(plan)
}

pub(super) fn fixture_probe() -> Result<serde_json::Value> {
    let fixture = CopyFixture::new()?;
    let (source, output) = fixture.scope()?;
    use crate::jobs::operation_locks::{Access, Registry, Scope};
    let scope_lease = Registry::shared().try_acquire(vec![
        Scope::new(source, Access::Read),
        Scope::new(output, Access::Write),
    ])?;
    let output = scope_lease.directory(1);
    let journal = Journal::open(&fixture.run.path().join("operation.sqlite"), &fixture.plan)?;
    let prepared = prepare(
        output.leaf(),
        &targets(&[
            "2026/Trip/Photo.JPG",
            "2026/Trip/Photo.xmp",
            "2026/Trip/unknown.bin",
        ]),
        &journal,
        &AtomicBool::new(false),
    )?;
    for stage in &prepared.stages {
        let file = native::read(
            &prepared.directories[stage.parent].handle,
            &stage.stage_leaf,
        )?;
        if !stage.object.matches(&file)?
            || file.metadata()?.len() != 0
            || native::lookup(
                &prepared.directories[stage.parent].handle,
                &stage.original_leaf,
            )?
            .is_some()
        {
            return Err(failed("Prepared-name fixture evidence changed."));
        }
    }
    if !fixture.sources_preserved()? {
        return Err(failed("The name-preparation fixture source changed."));
    }
    let plan = prepared_copy_plan(&fixture, &prepared)?;
    // Preparation facts and execution events have distinct immutable journals.
    // Neither old intent nor an empty file acts as execution/replay authority.
    let copy = Journal::create(&fixture.run.path().join("prepared-copy.sqlite"), &plan)?;
    let cancel = AtomicBool::new(false);
    let before = plan
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            super::inspect_item(
                file,
                index,
                &copy,
                scope_lease.directory(0).leaf(),
                &prepared.directories[0].handle,
                &fixture.io,
                &cancel,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    if before
        .iter()
        .any(|state| *state != super::RecoveredItem::PreparedEmptyStage)
    {
        return Err(failed(
            "Prepared stages were not identified as empty unstarted objects.",
        ));
    }
    let validate = || {
        fixture.scope()?;
        let root = native::lookup(output.leaf(), &prepared.report.temporary_namespace)?
            .ok_or_else(|| failed("Prepared namespace disappeared."))?;
        if root.kind != native::EntryKind::Directory
            || root.object != prepared.directories[0].object
        {
            return Err(failed("Prepared namespace changed membership."));
        }
        Ok(())
    };
    let outcome = super::CopyContext {
        plan: &plan,
        journal: &copy,
        source_root: scope_lease.directory(0).leaf(),
        destination: &prepared.directories[0].handle,
        io: &fixture.io,
        cancel: &cancel,
        validate_scope: &validate,
    }
    .execute(None)?;
    if outcome != super::CopyOutcome::CompletedVerified {
        return Err(failed("Prepared-stage fixture did not complete."));
    }
    let after = plan
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            super::inspect_item(
                file,
                index,
                &copy,
                scope_lease.directory(0).leaf(),
                &prepared.directories[0].handle,
                &fixture.io,
                &cancel,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    if after
        .iter()
        .any(|state| *state != super::RecoveredItem::PublishedVerified)
        || !fixture.sources_preserved()?
    {
        return Err(failed(
            "Prepared-stage copy failed readback or source preservation.",
        ));
    }
    let mut final_ids = Vec::new();
    for file in &plan.files {
        let parent = super::destination_parent(&prepared.directories[0].handle, file)?;
        let output = native::read(&parent, &file.target)?;
        let initial = file
            .prepared_stage
            .as_ref()
            .ok_or_else(|| failed("Missing prepared object proof."))?;
        if !initial.object.matches(&output)? {
            return Err(failed("Prepared stage was replaced rather than reused."));
        }
        final_ids.push(initial.object.clone());
    }
    Ok(
        serde_json::json!({"scope":"generated-temporary-only","sourcesPreserved":true,"report":prepared.report,"copyFromPreparedStages":{"outcome":outcome,"before":before,"after":after,"samePreparedObjectsPublished":final_ids.len(),"sourcesPreserved":true,"destinationNeighborPreserved":true}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepared_stage_changes_are_never_overwritten_recreated_or_adopted() {
        use std::io::Write;
        for change in [
            "nonempty",
            "changed_empty",
            "replaced",
            "missing",
            "hardlink",
        ] {
            let fixture = CopyFixture::new().unwrap();
            let (source, output) = fixture.scope().unwrap();
            let journal =
                Journal::open(&fixture.run.path().join("operation.sqlite"), &fixture.plan).unwrap();
            let prepared = prepare(
                output.leaf(),
                &targets(&[
                    "2026/Trip/Photo.JPG",
                    "2026/Trip/Photo.xmp",
                    "2026/Trip/unknown.bin",
                ]),
                &journal,
                &AtomicBool::new(false),
            )
            .unwrap();
            let plan = prepared_copy_plan(&fixture, &prepared).unwrap();
            let item = &plan.files[0];
            let stage = &prepared.stages[0];
            let parent = &prepared.directories[stage.parent].handle;
            let proof = item.prepared_stage.as_ref().unwrap();
            let stage_path = fixture
                .destination
                .join("organized")
                .join(&prepared.report.temporary_namespace)
                .join("2026/Trip")
                .join(&stage.stage_leaf);
            match change {
                "nonempty" | "changed_empty" => {
                    let mut file =
                        native::reopen_stage(parent, &stage.stage_leaf, &proof.identity()).unwrap();
                    let old_time = file.metadata().unwrap().modified().unwrap();
                    file.write_all(b"unapproved stage data").unwrap();
                    file.sync_all().unwrap();
                    if change == "changed_empty" {
                        file.set_len(0).unwrap();
                        file.set_times(std::fs::FileTimes::new().set_modified(old_time))
                            .unwrap();
                        file.sync_all().unwrap();
                    }
                }
                "replaced" | "missing" => {
                    let file =
                        native::reopen_stage(parent, &stage.stage_leaf, &proof.identity()).unwrap();
                    drop(
                        native::publish_new(
                            parent,
                            &stage.stage_leaf,
                            file,
                            "retained-stage.partial",
                            &proof.identity(),
                        )
                        .unwrap(),
                    );
                    if change == "replaced" {
                        drop(native::create_file(parent, &stage.stage_leaf).unwrap());
                    }
                }
                "hardlink" => std::fs::hard_link(
                    &stage_path,
                    stage_path.with_file_name("extra-stage-link.partial"),
                )
                .unwrap(),
                _ => unreachable!(),
            }
            let copy =
                Journal::create(&fixture.run.path().join("prepared-copy.sqlite"), &plan).unwrap();
            let cancel = AtomicBool::new(false);
            let validate = || fixture.scope().map(|_| ());
            let context = super::super::CopyContext {
                plan: &plan,
                journal: &copy,
                source_root: source.leaf(),
                destination: &prepared.directories[0].handle,
                io: &fixture.io,
                cancel: &cancel,
                validate_scope: &validate,
            };
            assert!(
                context.execute(None).is_err(),
                "Changed prepared stage was accepted: {change}"
            );
            assert!(
                super::super::inspect_item(
                    item,
                    0,
                    &copy,
                    source.leaf(),
                    &prepared.directories[0].handle,
                    &fixture.io,
                    &cancel
                )
                .is_err()
            );
            assert!(
                native::lookup(parent, &stage.original_leaf)
                    .unwrap()
                    .is_none()
            );
            assert!(fixture.sources_preserved().unwrap());
            if change == "nonempty" {
                assert_eq!(
                    std::fs::read(&stage_path).unwrap(),
                    b"unapproved stage data"
                );
            }
            if change == "missing" {
                assert!(!stage_path.exists());
                assert!(stage_path.with_file_name("retained-stage.partial").exists());
            }
            if change == "hardlink" {
                assert!(
                    stage_path
                        .with_file_name("extra-stage-link.partial")
                        .exists()
                );
            }
        }
    }
    struct ResetFault;
    impl Drop for ResetFault {
        fn drop(&mut self) {
            FAULT.with(|fault| fault.set(None));
        }
    }
    #[test]
    fn interruptions_between_native_mutation_and_journal_completion_retain_pending_scope() {
        for point in [
            Boundary::RootCreated,
            Boundary::DirectoryCreated,
            Boundary::LeafCreated,
            Boundary::StageMoved,
        ] {
            let fixture = CopyFixture::new().unwrap();
            let (_, output) = fixture.scope().unwrap();
            let journal =
                Journal::open(&fixture.run.path().join("operation.sqlite"), &fixture.plan).unwrap();
            FAULT.with(|fault| fault.set(Some(point)));
            let reset = ResetFault;
            let result = prepare(
                output.leaf(),
                &targets(&["2026/Photo.JPG"]),
                &journal,
                &AtomicBool::new(false),
            );
            drop(reset);
            assert!(matches!(
                result,
                Err(AppError {
                    code: ErrorCode::WorkerFailed,
                    ..
                })
            ));
            let completed: i64 = journal
                .0
                .query_row(
                    "SELECT count(*) FROM events WHERE phase='namespace_prepared'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(completed, 0);
            assert_eq!(
                std::fs::read_dir(fixture.destination.join("organized"))
                    .unwrap()
                    .count(),
                1
            );
            assert!(fixture.sources_preserved().unwrap());
        }
    }
    #[test]
    fn oversized_scope_and_denied_journal_never_create_unrecorded_namespaces() {
        let fixture = CopyFixture::new().unwrap();
        let (_, output) = fixture.scope().unwrap();
        let journal =
            Journal::open(&fixture.run.path().join("operation.sqlite"), &fixture.plan).unwrap();
        let path = [
            "a".repeat(250),
            "b".repeat(250),
            "c".repeat(250),
            "d".repeat(250),
        ]
        .join("/");
        let large: Vec<_> = (0..9_000)
            .map(|index| Target {
                id: blake3::hash(index.to_string().as_bytes())
                    .to_hex()
                    .to_string(),
                path: path.clone(),
            })
            .collect();
        assert!(prepare(output.leaf(), &large, &journal, &AtomicBool::new(false)).is_err());
        journal.0.pragma_update(None, "query_only", true).unwrap();
        assert!(matches!(
            prepare(
                output.leaf(),
                &targets(&["Photo.JPG"]),
                &journal,
                &AtomicBool::new(false)
            ),
            Err(AppError {
                code: ErrorCode::Catalog,
                ..
            })
        ));
        assert_eq!(
            std::fs::read_dir(fixture.destination.join("organized"))
                .unwrap()
                .count(),
            0
        );
        assert!(fixture.sources_preserved().unwrap());
    }
    fn probe(paths: &[&str]) -> (CopyFixture, Preparation) {
        let fixture = CopyFixture::new().unwrap();
        let (_, output) = fixture.scope().unwrap();
        let journal =
            Journal::open(&fixture.run.path().join("operation.sqlite"), &fixture.plan).unwrap();
        let result = prepare(
            output.leaf(),
            &targets(paths),
            &journal,
            &AtomicBool::new(false),
        )
        .unwrap();
        drop(output);
        (fixture, result)
    }
    #[test]
    fn native_preparation_checks_actual_future_names_and_retains_only_empty_owned_stages() {
        let receipt = fixture_probe().unwrap();
        assert_eq!(receipt["sourcesPreserved"], true);
        assert_eq!(receipt["report"]["retainedStages"], 3);
        assert_eq!(receipt["report"]["executionAvailable"], false);
    }
    #[test]
    fn native_case_unicode_and_file_directory_conflicts_mark_all_participants() {
        let (fixture, prepared) = probe(&["Case.JPG", "case.JPG", "café.xmp", "cafe\u{301}.xmp"]);
        for pair in prepared.report.rows.chunks(2) {
            assert_eq!(
                pair[0].state == State::Prepared,
                pair[1].state == State::Prepared
            );
        }
        assert!(fixture.sources_preserved().unwrap());
        for paths in [
            ["trip/photo.jpg", "trip/photo.jpg/child.bin"],
            ["trip/photo.jpg/child.bin", "trip/photo.jpg"],
        ] {
            let (fixture, prepared) = probe(&paths);
            assert!(
                prepared
                    .report
                    .rows
                    .iter()
                    .all(|row| row.state != State::Prepared)
            );
            assert!(fixture.sources_preserved().unwrap());
        }
    }
    #[test]
    fn fresh_member_validation_rejects_changed_stages_and_new_target_occupants() {
        for occupy_target in [false, true] {
            let (fixture, prepared) = probe(&["trip/Photo.JPG"]);
            let (_, output) = fixture.scope().unwrap();
            let stage = &prepared.stages[0];
            let parent = &prepared.directories[stage.parent].handle;
            if occupy_target {
                let _neighbor = native::create_file(parent, &stage.original_leaf).unwrap();
            } else {
                use std::io::Write;
                let id = filesystem::identity(&native::read(parent, &stage.stage_leaf).unwrap())
                    .unwrap();
                let mut file = native::reopen_stage(parent, &stage.stage_leaf, &id).unwrap();
                file.write_all(b"unexpected").unwrap();
                file.sync_all().unwrap();
            }
            assert!(
                validate_members(
                    output.leaf(),
                    &prepared.report.temporary_namespace,
                    &prepared.directories,
                    &prepared.stages,
                )
                .is_err()
            );
            assert!(fixture.sources_preserved().unwrap());
        }
    }
    #[test]
    fn complete_pending_inventory_rejects_unknown_files_and_directories_without_adopting_them() {
        for directory in [false, true] {
            let (fixture, prepared) = probe(&["trip/Photo.JPG"]);
            let (_, output) = fixture.scope().unwrap();
            let parent = &prepared.directories[0].handle;
            let unknown = if directory {
                native::create_directory(parent, "unexpected").unwrap()
            } else {
                native::create_file(parent, "unexpected").unwrap()
            };
            let identity = Object::of(&unknown).unwrap();
            assert!(
                validate_members(
                    output.leaf(),
                    &prepared.report.temporary_namespace,
                    &prepared.directories,
                    &prepared.stages,
                )
                .is_err()
            );
            assert!(
                native::lookup(parent, "unexpected")
                    .unwrap()
                    .unwrap()
                    .object
                    == identity
            );
            assert!(fixture.sources_preserved().unwrap());
        }
    }
    #[test]
    fn invalid_and_pre_canceled_preparation_never_creates_a_namespace() {
        let fixture = CopyFixture::new().unwrap();
        let (_, output) = fixture.scope().unwrap();
        let journal =
            Journal::open(&fixture.run.path().join("operation.sqlite"), &fixture.plan).unwrap();
        for paths in [["../outside"], ["Package.photoslibrary/photo.jpg"]] {
            assert!(
                prepare(
                    output.leaf(),
                    &targets(&paths),
                    &journal,
                    &AtomicBool::new(false)
                )
                .is_err()
            );
        }
        assert!(
            prepare(
                output.leaf(),
                &targets(&["Photo.JPG"]),
                &journal,
                &AtomicBool::new(true)
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read_dir(fixture.destination.join("organized"))
                .unwrap()
                .count(),
            0
        );
        assert!(fixture.sources_preserved().unwrap());
    }
}
