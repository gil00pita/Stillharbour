//! Closed native experiment. Only this constructor's generated objects can be
//! moved. Retained local versions are NOT independent recovery or M4 evidence.
mod catalog;
mod controller;
mod quarantine;
mod session;
use super::{approval, failed, native, source_native};
use crate::{
    domain::Result,
    filesystem::{self, DirectoryChain, FileIdentity},
    jobs::{
        operation_locks::{Access, Registry, Scope},
        paced_io::{CHUNK_BYTES, IoStep, PacedIo},
    },
    operations::rename_graph::{self as graph, Facts, Member, NameFact, Plan},
};
pub use controller::GeneratedRenameController;
use rusqlite::{Connection, params};
pub use session::GeneratedRenameSession;
#[derive(Default)]
struct RunControl {
    stop: std::sync::atomic::AtomicBool,
    completed: std::sync::atomic::AtomicUsize,
    #[cfg(feature = "fixture-probes")]
    hold_first: bool,
    #[cfg(test)]
    after_step: Option<(usize, std::sync::Arc<std::sync::Barrier>)>,
}
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Seek, SeekFrom},
    sync::atomic::AtomicBool,
    time::SystemTime,
};

struct Original {
    identity: FileIdentity,
    version: FileIdentity,
    modified: SystemTime,
    bytes: u64,
    readonly: bool,
    created: Option<SystemTime>,
}
struct Fixture {
    run: Run,
    root_id: String,
    source_root: super::Object,
    version_root: super::Object,
    originals: BTreeMap<String, Original>,
    probes: Vec<super::Object>,
    plan: Plan,
}
// Private, constructor-owned scope retained across native movement and the
// catalog/job transaction. It cannot authorize another fixture or caller path.
struct ExecutionScope<'a> {
    fixture: &'a Fixture,
    lease: crate::jobs::operation_locks::Lease,
}
enum Run {
    Owned(tempfile::TempDir),
    #[cfg(test)]
    Child(std::path::PathBuf),
}
impl Run {
    fn path(&self) -> &std::path::Path {
        match self {
            Self::Owned(run) => run.path(),
            #[cfg(test)]
            Self::Child(path) => path,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    AfterIntent(usize),
    AfterMove(usize),
    DenyIntent(usize),
    DenyCompletion(usize),
    #[cfg(test)]
    ExitAfterIntent(usize),
    #[cfg(test)]
    ExitAfterMove(usize),
    #[cfg(test)]
    ExitAfterCompletion(usize),
}
const MEMBERS: [(&str, &str); 5] = [
    ("Case.JPG", "case.JPG"),
    ("Case.xmp", "case.xmp"),
    ("A.JPG", "B.JPG"),
    ("B.JPG", "A.JPG"),
    ("Undo.JPG", "Renamed.JPG"),
];
fn payload(index: usize) -> String {
    format!("Stillharbour disposable rename member {index}\n").repeat(37)
}
fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn root(path: &std::path::Path) -> Result<DirectoryChain> {
    #[cfg(windows)]
    {
        DirectoryChain::open_for_owned_publication(path)
    }
    #[cfg(not(windows))]
    {
        DirectoryChain::open(path)
    }
}
impl Fixture {
    fn new() -> Result<Self> {
        let fixture = Self::generated_members(&MEMBERS)?;
        if !fixture.plan.issues.is_empty() {
            return Err(failed("Generated native rename planning conflicted."));
        }
        Ok(fixture)
    }
    // All callers supply literal generated names. This is not an imported-plan
    // constructor or a grant to touch a supplied directory.
    fn generated_members(pairs: &[(&str, &str)]) -> Result<Self> {
        let run = tempfile::Builder::new()
            .prefix("stillharbour-rename-fixture-")
            .tempdir()?;
        let path = run.path().canonicalize()?;
        std::fs::create_dir(path.join("originals"))?;
        std::fs::create_dir(path.join("retained"))?;
        let directory = root(&path.join("originals"))?;
        let versions = root(&path.join("retained"))?;
        let parent = directory.leaf();
        let id = uuid();
        let root_id = uuid();
        let mut members = pairs
            .iter()
            .map(|(before, after)| Member {
                id: uuid(),
                before: (*before).into(),
                after: (*after).into(),
                revision: "0".repeat(64),
            })
            .collect::<Vec<_>>();
        // Observe native equivalence on the exact empty parent using exclusive
        // owned probes. Never guess case/Unicode behavior by lowercasing names.
        let names = members
            .iter()
            .flat_map(|m| {
                [
                    m.before.clone(),
                    m.after.clone(),
                    graph::stage_name(&id, &m.id),
                ]
            })
            .collect::<Vec<_>>();
        let mut classes: BTreeMap<String, (String, FileIdentity)> = BTreeMap::new();
        let mut representatives: Vec<(String, FileIdentity)> = Vec::new();
        for name in names {
            if classes.contains_key(&name) {
                continue;
            }
            let mut known = None;
            for (_, identity) in &representatives {
                if native::reopen_stage(parent, &name, identity).is_ok() {
                    known = Some(identity.clone());
                    break;
                }
            }
            let identity = if let Some(identity) = known {
                identity
            } else {
                let file = native::create_file(parent, &name)?;
                let identity = filesystem::identity(&file)?;
                drop(file);
                representatives.push((name.clone(), identity.clone()));
                identity
            };
            classes.insert(
                name,
                (format!("{}:{}", identity.volume, identity.file), identity),
            );
        }
        let mut facts = Facts {
            root_id: root_id.clone(),
            root_revision: source_native::revision(parent)?,
            available: true,
            targets: members
                .iter()
                .map(|m| {
                    (
                        m.id.clone(),
                        NameFact {
                            native_key: classes[&m.after].0.clone(),
                            occupant: members
                                .iter()
                                .find(|other| classes[&other.before].0 == classes[&m.after].0)
                                .map(|other| other.id.clone()),
                        },
                    )
                })
                .collect(),
            stages: members
                .iter()
                .map(|m| {
                    let name = graph::stage_name(&id, &m.id);
                    (
                        name.clone(),
                        NameFact {
                            native_key: classes[&name].0.clone(),
                            occupant: None,
                        },
                    )
                })
                .collect(),
        };
        let mut probes = Vec::new();
        for (index, (name, identity)) in representatives.into_iter().enumerate() {
            let file = native::reopen_stage(parent, &name, &identity)?;
            let file = native::publish_new(
                parent,
                &name,
                file,
                &format!(".stillharbour-probe-{index}"),
                &identity,
            )?;
            probes.push(super::Object::of(&file)?);
        }
        let io = PacedIo::shared();
        let cancel = AtomicBool::new(false);
        let mut originals = BTreeMap::new();
        for (index, member) in members.iter_mut().enumerate() {
            // Distinct deterministic generated bytes, including a companion.
            // Decoder/media support is deliberately outside this rename test.
            let payload = payload(index);
            let mut file = native::create_file(parent, &member.before)?;
            let mut version = native::create_file(versions.leaf(), &member.id)?;
            if io.write_all(&mut file, payload.as_bytes(), &cancel)? == IoStep::Canceled
                || io.write_all(&mut version, payload.as_bytes(), &cancel)? == IoStep::Canceled
            {
                return Err(failed("Fixture creation canceled."));
            }
            file.sync_all()?;
            version.sync_all()?;
            member.revision = source_native::revision(&file)?;
            originals.insert(
                member.id.clone(),
                Original {
                    identity: filesystem::identity(&file)?,
                    version: filesystem::identity(&version)?,
                    modified: file.metadata()?.modified()?,
                    bytes: payload.len() as u64,
                    readonly: file.metadata()?.permissions().readonly(),
                    created: file.metadata()?.created().ok(),
                },
            );
        }
        facts.root_revision = source_native::revision(parent)?;
        let plan = graph::build(id, members, facts)?;
        let result = Self {
            run: Run::Owned(run),
            root_id,
            source_root: super::Object::of(parent)?,
            version_root: super::Object::of(versions.leaf())?,
            originals,
            probes,
            plan,
        };
        result.journal(&result.plan)?;
        Ok(result)
    }
    fn journal(&self, plan: &Plan) -> Result<Connection> {
        let db = Connection::open(self.run.path().join(format!("{}.sqlite", plan.id)))?;
        db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS scope (digest TEXT NOT NULL); CREATE TABLE IF NOT EXISTS events (ordinal INTEGER PRIMARY KEY, step INTEGER NOT NULL, phase TEXT NOT NULL CHECK(phase IN ('intent','done')), revision TEXT NOT NULL);")?;
        let digest: Option<String> = db
            .query_row("SELECT digest FROM scope", [], |row| row.get(0))
            .optional()?;
        if let Some(digest) = digest {
            if digest != plan.digest {
                return Err(failed("Rename journal plan changed."));
            }
        } else {
            db.execute("INSERT INTO scope VALUES (?1)", [&plan.digest])?;
        }
        Ok(db)
    }
    fn cursor(db: &Connection, plan: &Plan) -> Result<(usize, bool)> {
        let mut statement = db.prepare("SELECT step,phase FROM events ORDER BY ordinal")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, u32>(0)? as usize, row.get::<_, String>(1)?))
        })?;
        let mut completed = 0;
        let mut intent = false;
        for row in rows {
            let (step, phase) = row?;
            if step != completed || step >= plan.steps.len() || (phase == "intent") == intent {
                return Err(failed("Rename journal sequence is invalid."));
            }
            if phase == "intent" {
                intent = true;
            } else if phase == "done" {
                completed += 1;
                intent = false;
            } else {
                return Err(failed("Unknown rename journal phase."));
            }
        }
        Ok((completed, intent))
    }
    fn locations(
        &self,
        plan: &Plan,
        parent: &File,
        db: &Connection,
    ) -> Result<BTreeMap<String, String>> {
        let (completed, intent) = Self::cursor(db, plan)?;
        let mut locations = plan
            .members
            .iter()
            .map(|m| (m.id.clone(), m.before.clone()))
            .collect::<BTreeMap<_, _>>();
        for step in &plan.steps[..completed] {
            locations.insert(step.member_id.clone(), step.to.clone());
        }
        if intent {
            let step = &plan.steps[completed];
            if native::reopen_stage(parent, &step.to, &self.originals[&step.member_id].identity)
                .is_ok()
            {
                locations.insert(step.member_id.clone(), step.to.clone());
            }
        }
        Ok(locations)
    }
    fn verify(
        &self,
        parent: &File,
        versions: &File,
        locations: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, String>> {
        if !self.source_root.matches(parent)? || !self.version_root.matches(versions)? {
            return Err(failed(
                "A retained fixture directory changed native identity.",
            ));
        }
        let mut inventory = self
            .probes
            .iter()
            .cloned()
            .map(|o| (o, native::EntryKind::File))
            .collect::<Vec<_>>();
        let mut revisions = BTreeMap::new();
        for (id, name) in locations {
            let (object, revision) = self.verify_member(parent, versions, id, name)?;
            revisions.insert(id.clone(), revision);
            inventory.push((object, native::EntryKind::File));
        }
        native::verify_entries(parent, &inventory)?;
        Ok(revisions)
    }
    fn verify_member(
        &self,
        parent: &File,
        versions: &File,
        id: &str,
        name: &str,
    ) -> Result<(super::Object, String)> {
        if !self.version_root.matches(versions)? {
            return Err(failed("Retained version directory identity changed."));
        }
        let io = PacedIo::shared();
        let cancel = AtomicBool::new(false);
        let original = &self.originals[id];
        let mut file = native::reopen_stage(parent, name, &original.identity)?;
        let mut version = native::read(versions, id)?;
        if filesystem::identity(&version)? != original.version {
            return Err(failed(
                "Retained fixture version changed identity or link count.",
            ));
        }
        let metadata = file.metadata()?;
        if metadata.len() != original.bytes
            || metadata.modified()? != original.modified
            || metadata.permissions().readonly() != original.readonly
            || metadata.created().ok() != original.created
        {
            return Err(failed("Rename fixture metadata changed."));
        }
        file.seek(SeekFrom::Start(0))?;
        version.seek(SeekFrom::Start(0))?;
        let mut left = [0; CHUNK_BYTES];
        let mut right = [0; CHUNK_BYTES];
        loop {
            let IoStep::Completed(a) = io.read(&mut file, &mut left, &cancel)? else {
                return Err(failed("Verification canceled."));
            };
            let IoStep::Completed(b) = io.read(&mut version, &mut right, &cancel)? else {
                return Err(failed("Verification canceled."));
            };
            if a != b || left[..a] != right[..b] {
                return Err(failed("Retained rename version differs."));
            }
            if a == 0 {
                break;
            }
        }
        Ok((super::Object::of(&file)?, source_native::revision(&file)?))
    }
    fn execute(&self, plan: &Plan, fault: Option<Fault>) -> Result<bool> {
        self.execute_controlled(plan, fault, None)
    }
    fn execute_controlled(
        &self,
        plan: &Plan,
        fault: Option<Fault>,
        control: Option<&RunControl>,
    ) -> Result<bool> {
        let scope = self.execution_scope()?;
        self.execute_in_scope(plan, fault, control, &scope)
    }
    fn execution_scope(&self) -> Result<ExecutionScope<'_>> {
        let path = self.run.path().canonicalize()?;
        let lease = Registry::shared().try_acquire(vec![
            Scope::new(root(&path.join("originals"))?, Access::Write),
            Scope::new(root(&path.join("retained"))?, Access::Read),
        ])?;
        Ok(ExecutionScope {
            fixture: self,
            lease,
        })
    }
    fn execute_in_scope(
        &self,
        plan: &Plan,
        fault: Option<Fault>,
        control: Option<&RunControl>,
        scope: &ExecutionScope<'_>,
    ) -> Result<bool> {
        if !std::ptr::eq(self, scope.fixture) {
            return Err(failed(
                "A held organization scope belongs to another fixture.",
            ));
        }
        if !plan.issues.is_empty() || plan.root_id != self.root_id {
            return Err(failed("Unresolved or foreign fixture rename scope."));
        }
        let canonical = graph::build(
            plan.id.clone(),
            plan.members.clone(),
            plan.name_facts.clone(),
        )?;
        let encode = |value: &Plan| {
            serde_json::to_vec(value)
                .map_err(|error| failed("Invalid retained fixture plan.").caused_by(error))
        };
        if encode(&canonical)? != encode(plan)? {
            return Err(failed("Retained fixture rename ordering changed."));
        }
        let directory = scope.lease.directory(0);
        let versions = scope.lease.directory(1);
        let parent = directory.leaf();
        let db = self.journal(plan)?;
        let locations = self.locations(plan, parent, &db)?;
        let revisions = self.verify(parent, versions.leaf(), &locations)?;
        let (mut cursor, mut intent) = Self::cursor(&db, plan)?;
        let mut expected = plan
            .members
            .iter()
            .map(|m| (m.id.clone(), m.revision.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut statement =
            db.prepare("SELECT step,revision FROM events WHERE phase='done' ORDER BY ordinal")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, u32>(0)? as usize, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (step, revision) = row?;
            expected.insert(plan.steps[step].member_id.clone(), revision);
        }
        for (member, revision) in &revisions {
            let reconciled_publication = intent
                && plan.steps[cursor].member_id == *member
                && locations[member] == plan.steps[cursor].to;
            if !reconciled_publication && expected.get(member) != Some(revision) {
                return Err(failed(
                    "Fixture rename source revision changed since its retained phase.",
                ));
            }
        }
        let digest = blake3::hash(
            &serde_json::to_vec(&(plan.digest.clone(), cursor, intent, revisions)).map_err(
                |error| failed("Could not bind fixture rename approval.").caused_by(error),
            )?,
        )
        .to_hex()
        .to_string();
        let approvals = approval::Approvals::new();
        let review = approval::Review::new(
            approval::Action::RenameFixture,
            plan.id.clone(),
            digest,
            self.root_id.clone(),
        )?;
        let ticket = approvals.issue(review.clone())?;
        approvals.consume(&ticket, &review)?.authorize(&review)?;
        if approvals.consume(&ticket, &review).is_ok() {
            return Err(failed("Rename approval replayed."));
        }
        if let Some(control) = control {
            control
                .completed
                .store(cursor, std::sync::atomic::Ordering::Release);
        }
        while cursor < plan.steps.len() {
            if control
                .is_some_and(|control| control.stop.load(std::sync::atomic::Ordering::Acquire))
            {
                return Ok(false);
            }
            let step = &plan.steps[cursor];
            let original = &self.originals[&step.member_id];
            if !intent {
                if fault == Some(Fault::DenyIntent(cursor)) {
                    db.pragma_update(None, "query_only", true)?;
                }
                db.execute(
                    "INSERT INTO events(step,phase,revision) VALUES (?1,'intent',?2)",
                    params![cursor as i64, expected[&step.member_id]],
                )?;
            }
            if fault == Some(Fault::AfterIntent(cursor)) {
                return Ok(false);
            }
            #[cfg(test)]
            if fault == Some(Fault::ExitAfterIntent(cursor)) {
                std::process::exit(79);
            }
            // After a lost completion, inspect exact original object at the
            // planned destination. No content-only adoption or blind replay.
            if native::reopen_stage(parent, &step.to, &original.identity).is_err() {
                let file = native::reopen_stage(parent, &step.from, &original.identity)?;
                if source_native::revision(&file)? != expected[&step.member_id] {
                    return Err(failed("Fixture source revision changed before its move."));
                }
                drop(native::publish_new(
                    parent,
                    &step.from,
                    file,
                    &step.to,
                    &original.identity,
                )?);
            }
            if fault == Some(Fault::AfterMove(cursor)) {
                return Ok(false);
            }
            #[cfg(test)]
            if fault == Some(Fault::ExitAfterMove(cursor)) {
                std::process::exit(79);
            }
            let mut current = self.locations(plan, parent, &db)?;
            current.insert(step.member_id.clone(), step.to.clone());
            let current_revisions = self.verify(parent, versions.leaf(), &current)?;
            if fault == Some(Fault::DenyCompletion(cursor)) {
                db.pragma_update(None, "query_only", true)?;
            }
            db.execute(
                "INSERT INTO events(step,phase,revision) VALUES (?1,'done',?2)",
                params![cursor as i64, current_revisions[&step.member_id]],
            )?;
            expected.insert(
                step.member_id.clone(),
                current_revisions[&step.member_id].clone(),
            );
            #[cfg(test)]
            if fault == Some(Fault::ExitAfterCompletion(cursor)) {
                std::process::exit(79);
            }
            cursor += 1;
            intent = false;
            if let Some(control) = control {
                control
                    .completed
                    .store(cursor, std::sync::atomic::Ordering::Release);
                #[cfg(feature = "fixture-probes")]
                if control.hold_first && cursor == 1 {
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
                    while !control.stop.load(std::sync::atomic::Ordering::Acquire)
                        && std::time::Instant::now() < deadline
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    // An expired diagnostic hold stops with every object/fact
                    // retained; it never silently resumes the remaining moves.
                    control
                        .stop
                        .store(true, std::sync::atomic::Ordering::Release);
                }
                #[cfg(test)]
                if let Some((step, gate)) = &control.after_step
                    && cursor == *step
                {
                    gate.wait();
                }
            }
        }
        self.verify(parent, versions.leaf(), &self.locations(plan, parent, &db)?)?;
        Ok(true)
    }
    fn reverse_plan(&mut self) -> Result<Plan> {
        let path = self.run.path().canonicalize()?;
        let directory = root(&path.join("originals"))?;
        let versions = root(&path.join("retained"))?;
        let _lease = Registry::shared().try_acquire(vec![
            Scope::new(root(&path.join("originals"))?, Access::Write),
            Scope::new(root(&path.join("retained"))?, Access::Read),
        ])?;
        let parent = directory.leaf();
        let db = self.journal(&self.plan)?;
        if Self::cursor(&db, &self.plan)? != (self.plan.steps.len(), false) {
            return Err(failed("Undo needs a completed retained fixture operation."));
        }
        let locations = self.locations(&self.plan, parent, &db)?;
        let revisions = self.verify(parent, versions.leaf(), &locations)?;
        let id = uuid();
        let mut representatives: Vec<(String, FileIdentity)> = Vec::new();
        let mut observe = |name: &str| -> Result<NameFact> {
            for (member_id, original) in &self.originals {
                if native::reopen_stage(parent, name, &original.identity).is_ok() {
                    return Ok(NameFact {
                        native_key: format!(
                            "{}:{}",
                            original.identity.volume, original.identity.file
                        ),
                        occupant: Some(member_id.clone()),
                    });
                }
            }
            for (_, identity) in &representatives {
                if native::reopen_stage(parent, name, identity).is_ok() {
                    return Ok(NameFact {
                        native_key: format!("{}:{}", identity.volume, identity.file),
                        occupant: None,
                    });
                }
            }
            if let Some(entry) = native::lookup(parent, name)? {
                return Ok(NameFact {
                    native_key: format!("{}:{}", entry.object.volume, entry.object.file),
                    occupant: Some("foreign-preserved-object".into()),
                });
            }
            let file = native::create_file(parent, name)?;
            let identity = filesystem::identity(&file)?;
            representatives.push((name.to_string(), identity.clone()));
            Ok(NameFact {
                native_key: format!("{}:{}", identity.volume, identity.file),
                occupant: None,
            })
        };
        let targets = self
            .plan
            .members
            .iter()
            .map(|member| Ok((member.id.clone(), observe(&member.before)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let stages = self
            .plan
            .members
            .iter()
            .filter(|m| m.before != m.after)
            .map(|member| {
                let name = graph::stage_name(&id, &member.id);
                Ok((name.clone(), observe(&name)?))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        for (name, identity) in representatives {
            let file = native::reopen_stage(parent, &name, &identity)?;
            let file = native::publish_new(
                parent,
                &name,
                file,
                &format!(".stillharbour-probe-{}", self.probes.len()),
                &identity,
            )?;
            self.probes.push(super::Object::of(&file)?);
        }
        graph::reverse(
            id,
            &self.plan,
            &revisions,
            Facts {
                root_id: self.root_id.clone(),
                root_revision: source_native::revision(parent)?,
                available: true,
                targets,
                stages,
            },
        )
    }
}
use rusqlite::OptionalExtension;

pub(super) fn probe() -> Result<serde_json::Value> {
    let mut fixture = Fixture::new()?;
    let interrupted = !fixture.execute(&fixture.plan, Some(Fault::AfterMove(0)))?;
    let resumed = fixture.execute(&fixture.plan, None)?;
    if !interrupted || !resumed {
        return Err(failed("Native rename interruption was not reconciled."));
    }
    let reverse = fixture.reverse_plan()?;
    if !reverse.issues.is_empty() || !fixture.execute(&reverse, None)? {
        return Err(failed("Native fixture undo did not restore originals."));
    }
    let unicode = unicode_probe()?;
    Ok(
        serde_json::json!({"scope":"constructor-owned-generated-only","nativeOs":std::env::consts::OS,"members":fixture.plan.members.len(),"steps":fixture.plan.steps.len(),"interruptedAfterFirstRename":interrupted,"freshApprovedContinuation":resumed,"reverseSteps":reverse.steps.len(),"reverseRestoredOriginalNames":true,"completeBytesAndModifiedTimeVerified":true,"originalObjectsRetained":true,"localRetainedVersionsOnly":true,"independentRecoveryProven":false,"productionAuthorization":false,"unicode":unicode}),
    )
}
fn unicode_probe() -> Result<serde_json::Value> {
    fn originals(fixture: &Fixture) -> Result<()> {
        let scope = fixture.execution_scope()?;
        let locations = fixture
            .plan
            .members
            .iter()
            .map(|m| (m.id.clone(), m.before.clone()))
            .collect();
        fixture.verify(
            scope.lease.directory(0).leaf(),
            scope.lease.directory(1).leaf(),
            &locations,
        )?;
        Ok(())
    }
    let mut rename = Fixture::generated_members(&[
        ("caf\u{e9}.JPG", "cafe\u{301}.JPG"),
        ("caf\u{e9}.xmp", "cafe\u{301}.xmp"),
    ])?;
    if !rename.plan.issues.is_empty()
        || rename.execute(&rename.plan, Some(Fault::AfterMove(0)))?
        || !rename.execute(&rename.plan, None)?
    {
        return Err(failed("Generated Unicode rename did not reconcile."));
    }
    let reverse = rename.reverse_plan()?;
    if !rename.execute(&reverse, None)? {
        return Err(failed("Generated Unicode undo did not complete."));
    }
    originals(&rename)?;

    let mut collision = Fixture::generated_members(&[
        ("First.JPG", "caf\u{e9}.JPG"),
        ("Second.JPG", "cafe\u{301}.JPG"),
    ])?;
    let keys = collision
        .plan
        .name_facts
        .targets
        .values()
        .map(|fact| &fact.native_key)
        .collect::<Vec<_>>();
    let equivalent = keys[0] == keys[1];
    if equivalent {
        if !collision.plan.issues.iter().any(|issue| {
            issue.kind == graph::Conflict::NativeNameCollision && issue.member_ids.len() == 2
        }) || !collision.plan.steps.is_empty()
            || collision.execute(&collision.plan, None).is_ok()
            || collision.journal(&collision.plan)?.query_row(
                "SELECT count(*) FROM events",
                [],
                |row| row.get::<_, i64>(0),
            )? != 0
        {
            return Err(failed(
                "Native equivalent Unicode targets were not blocked before movement.",
            ));
        }
    } else {
        if !collision.plan.issues.is_empty() || !collision.execute(&collision.plan, None)? {
            return Err(failed(
                "Distinct native Unicode targets did not remain distinct.",
            ));
        }
        let reverse = collision.reverse_plan()?;
        if !collision.execute(&reverse, None)? {
            return Err(failed("Distinct native Unicode targets did not undo."));
        }
    }
    originals(&collision)?;
    Ok(
        serde_json::json!({"canonicalTargetsEquivalent":equivalent,"collisionBlockedBeforeMove":equivalent,"distinctTargetsRoundTrip":!equivalent,"companionRenameContinuedAndUndone":true,"allOriginalObjectsBytesAndMetadataVerified":true,"sourceMembers":4,"productionAuthorization":false}),
    )
}
pub(super) fn relocation_probe() -> Result<serde_json::Value> {
    quarantine::probe()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    #[test]
    fn native_unicode_rename_and_collision_follow_observed_parent_semantics() {
        let receipt = unicode_probe().unwrap();
        assert_eq!(receipt["companionRenameContinuedAndUndone"], true);
        assert_eq!(receipt["allOriginalObjectsBytesAndMetadataVerified"], true);
        assert_eq!(receipt["sourceMembers"], 4);
        assert_eq!(receipt["productionAuthorization"], false);
        assert_ne!(
            receipt["collisionBlockedBeforeMove"],
            receipt["distinctTargetsRoundTrip"]
        );
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ChildScope {
        version: u32,
        forward: Plan,
        active: Plan,
        reverse: bool,
        probes: Vec<super::super::Object>,
        source_root: super::super::Object,
        version_root: super::super::Object,
    }
    fn bounded(file: File, limit: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        file.take(limit + 1).read_to_end(&mut bytes).unwrap();
        assert!(bytes.len() as u64 <= limit);
        bytes
    }
    #[test]
    fn native_rename_fixture_exit_child() {
        let Some(path) = std::env::var_os("STILLHARBOUR_RENAME_CHILD_ROOT") else {
            return;
        };
        let token = std::env::var("STILLHARBOUR_RENAME_CHILD_TOKEN").unwrap();
        uuid::Uuid::parse_str(&token).unwrap();
        let path = std::path::PathBuf::from(path).canonicalize().unwrap();
        assert!(path.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("stillharbour-rename-fixture-")
        );
        let container = root(&path).unwrap();
        assert_eq!(
            bounded(
                native::read(container.leaf(), "rename-child-owner").unwrap(),
                36
            ),
            token.as_bytes()
        );
        let scope: ChildScope = serde_json::from_slice(&bounded(
            native::read(container.leaf(), "rename-child-scope.json").unwrap(),
            128 * 1024,
        ))
        .unwrap();
        assert_eq!(scope.version, 1);
        assert_eq!(scope.forward.id, token);
        assert_eq!(scope.forward.members.len(), MEMBERS.len());
        assert_eq!(scope.active.members.len(), MEMBERS.len());
        assert_eq!(scope.active.root_id, scope.forward.root_id);
        assert!(scope.probes.len() <= 64);
        for plan in [&scope.forward, &scope.active] {
            let canonical = graph::build(
                plan.id.clone(),
                plan.members.clone(),
                plan.name_facts.clone(),
            )
            .unwrap();
            assert_eq!(
                serde_json::to_vec(&canonical).unwrap(),
                serde_json::to_vec(plan).unwrap()
            );
            assert!(plan.issues.is_empty());
        }
        let directory = root(&path.join("originals")).unwrap();
        let versions = root(&path.join("retained")).unwrap();
        assert!(scope.source_root.matches(directory.leaf()).unwrap());
        assert!(scope.version_root.matches(versions.leaf()).unwrap());
        assert_eq!(
            source_native::revision(directory.leaf()).unwrap(),
            scope.active.root_revision
        );
        let mut originals = BTreeMap::new();
        let mut names = std::collections::BTreeSet::new();
        for original in &scope.forward.members {
            let index = MEMBERS
                .iter()
                .position(|(before, after)| original.before == *before && original.after == *after)
                .unwrap();
            assert!(names.insert(index));
            let active = scope
                .active
                .members
                .iter()
                .find(|m| m.id == original.id)
                .unwrap();
            assert_eq!(
                active.before,
                if scope.reverse {
                    &original.after
                } else {
                    &original.before
                }
                .to_string()
            );
            assert_eq!(
                active.after,
                if scope.reverse {
                    &original.before
                } else {
                    &original.after
                }
                .to_string()
            );
            let file = native::read(directory.leaf(), &active.before).unwrap();
            assert_eq!(source_native::revision(&file).unwrap(), active.revision);
            let version = native::read(versions.leaf(), &active.id).unwrap();
            let metadata = file.metadata().unwrap();
            let identity = filesystem::identity(&file).unwrap();
            let retained_identity = filesystem::identity(&version).unwrap();
            let expected = payload(index);
            assert_eq!(bounded(file, expected.len() as u64), expected.as_bytes());
            assert_eq!(bounded(version, expected.len() as u64), expected.as_bytes());
            assert_eq!(identity.links, 1);
            assert_eq!(retained_identity.links, 1);
            originals.insert(
                active.id.clone(),
                Original {
                    identity,
                    version: retained_identity,
                    modified: metadata.modified().unwrap(),
                    bytes: expected.len() as u64,
                    readonly: metadata.permissions().readonly(),
                    created: metadata.created().ok(),
                },
            );
        }
        for (index, object) in scope.probes.iter().enumerate() {
            let file =
                native::read(directory.leaf(), &format!(".stillharbour-probe-{index}")).unwrap();
            assert!(object.matches(&file).unwrap());
            assert_eq!(file.metadata().unwrap().len(), 0);
            assert_eq!(filesystem::identity(&file).unwrap().links, 1);
        }
        let fixture = Fixture {
            run: Run::Child(path),
            root_id: scope.forward.root_id.clone(),
            source_root: scope.source_root,
            version_root: scope.version_root,
            originals,
            probes: scope.probes,
            plan: scope.forward,
        };
        let initial = scope
            .active
            .members
            .iter()
            .map(|m| (m.id.clone(), m.before.clone()))
            .collect();
        fixture
            .verify(directory.leaf(), versions.leaf(), &initial)
            .unwrap();
        let fault = std::env::var("STILLHARBOUR_RENAME_CHILD_FAULT").unwrap();
        let (phase, step) = fault.split_once(':').unwrap();
        let step: usize = step.parse().unwrap();
        assert!(step < scope.active.steps.len());
        let fault = match phase {
            "intent" => Fault::ExitAfterIntent(step),
            "move" => Fault::ExitAfterMove(step),
            "completion" => Fault::ExitAfterCompletion(step),
            _ => panic!("Unknown closed crash boundary"),
        };
        fixture.execute(&scope.active, Some(fault)).unwrap();
        panic!("The explicitly requested child exit did not occur");
    }
    #[test]
    fn native_rename_fixture_process_exit_reopens_actual_state_before_fresh_forward_or_reverse_approval()
     {
        for reverse in [false, true] {
            for step in [0, 5, 9] {
                for phase in ["intent", "move", "completion"] {
                    let mut fixture = Fixture::new().unwrap();
                    let active = if reverse {
                        assert!(fixture.execute(&fixture.plan, None).unwrap());
                        fixture.reverse_plan().unwrap()
                    } else {
                        fixture.plan.clone()
                    };
                    fixture.journal(&active).unwrap();
                    let scope = ChildScope {
                        version: 1,
                        forward: fixture.plan.clone(),
                        active: active.clone(),
                        reverse,
                        probes: fixture.probes.clone(),
                        source_root: fixture.source_root.clone(),
                        version_root: fixture.version_root.clone(),
                    };
                    for (name, bytes) in [
                        ("rename-child-owner", fixture.plan.id.as_bytes().to_vec()),
                        (
                            "rename-child-scope.json",
                            serde_json::to_vec(&scope).unwrap(),
                        ),
                    ] {
                        let mut file = std::fs::OpenOptions::new()
                            .create_new(true)
                            .write(true)
                            .open(fixture.run.path().join(name))
                            .unwrap();
                        file.write_all(&bytes).unwrap();
                        file.sync_all().unwrap();
                    }
                    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                        .args([
                            "--exact",
                            "executor::rename_fixture::tests::native_rename_fixture_exit_child",
                            "--nocapture",
                        ])
                        .env("STILLHARBOUR_RENAME_CHILD_ROOT", fixture.run.path())
                        .env("STILLHARBOUR_RENAME_CHILD_TOKEN", &fixture.plan.id)
                        .env("STILLHARBOUR_RENAME_CHILD_FAULT", format!("{phase}:{step}"))
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn()
                        .unwrap();
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
                    loop {
                        if child.try_wait().unwrap().is_some() {
                            break;
                        }
                        if std::time::Instant::now() >= deadline {
                            child.kill().unwrap();
                            child.wait().unwrap();
                            panic!("The owned rename child exceeded its 45-second bound");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    let result = child.wait_with_output().unwrap();
                    assert_eq!(
                        result.status.code(),
                        Some(79),
                        "reverse={reverse}, {phase}:{step}: {} {}",
                        String::from_utf8_lossy(&result.stdout),
                        String::from_utf8_lossy(&result.stderr)
                    );
                    let directory =
                        root(&fixture.run.path().canonicalize().unwrap().join("originals"))
                            .unwrap();
                    let versions =
                        root(&fixture.run.path().canonicalize().unwrap().join("retained")).unwrap();
                    let db = fixture.journal(&active).unwrap();
                    let expected = if phase == "completion" {
                        (step + 1, false)
                    } else {
                        (step, true)
                    };
                    assert_eq!(Fixture::cursor(&db, &active).unwrap(), expected);
                    let locations = fixture.locations(&active, directory.leaf(), &db).unwrap();
                    fixture
                        .verify(directory.leaf(), versions.leaf(), &locations)
                        .unwrap();
                    assert_eq!(Fixture::cursor(&db, &active).unwrap(), expected);
                    drop(db);
                    assert!(fixture.execute(&active, None).unwrap());
                    assert_eq!(
                        Fixture::cursor(&fixture.journal(&active).unwrap(), &active).unwrap(),
                        (10, false)
                    );
                }
            }
        }
    }
    #[test]
    fn native_rename_fixture_reused_directory_paths_never_replace_retained_root_identity() {
        for source in [false, true] {
            let fixture = Fixture::new().unwrap();
            let base = fixture.run.path().canonicalize().unwrap();
            let old = base.join(if source { "originals" } else { "retained" });
            let preserved = base.join("retained-unavailable-directory");
            assert!(old.starts_with(&base) && preserved.starts_with(&base));
            std::fs::rename(&old, &preserved).unwrap();
            std::fs::create_dir(&old).unwrap();
            assert!(fixture.execute(&fixture.plan, None).is_err());
            assert_eq!(
                Fixture::cursor(&fixture.journal(&fixture.plan).unwrap(), &fixture.plan).unwrap(),
                (0, false)
            );
            let original_path = if source {
                preserved.clone()
            } else {
                base.join("originals")
            };
            let version_path = if source {
                base.join("retained")
            } else {
                preserved.clone()
            };
            let directory = root(&original_path).unwrap();
            let versions = root(&version_path).unwrap();
            for member in &fixture.plan.members {
                let original = &fixture.originals[&member.id];
                assert!(
                    native::reopen_stage(directory.leaf(), &member.before, &original.identity)
                        .is_ok()
                );
                assert!(
                    native::reopen_stage(versions.leaf(), &member.id, &original.version).is_ok()
                );
            }
        }
    }
    #[test]
    fn native_rename_fixture_journal_denial_retains_intent_and_actual_objects_for_fresh_recovery() {
        for step in [0, 5, 9] {
            for completion in [false, true] {
                let fixture = Fixture::new().unwrap();
                let fault = if completion {
                    Fault::DenyCompletion(step)
                } else {
                    Fault::DenyIntent(step)
                };
                assert!(fixture.execute(&fixture.plan, Some(fault)).is_err());
                let directory =
                    root(&fixture.run.path().canonicalize().unwrap().join("originals")).unwrap();
                let versions =
                    root(&fixture.run.path().canonicalize().unwrap().join("retained")).unwrap();
                let db = fixture.journal(&fixture.plan).unwrap();
                assert_eq!(
                    Fixture::cursor(&db, &fixture.plan).unwrap(),
                    (step, completion)
                );
                let locations = fixture
                    .locations(&fixture.plan, directory.leaf(), &db)
                    .unwrap();
                fixture
                    .verify(directory.leaf(), versions.leaf(), &locations)
                    .unwrap();
                let current = &fixture.plan.steps[step];
                assert_eq!(
                    locations[&current.member_id],
                    if completion {
                        current.to.clone()
                    } else {
                        current.from.clone()
                    }
                );
                drop(db);
                assert!(fixture.execute(&fixture.plan, None).unwrap());
            }
        }
    }
    #[test]
    fn native_rename_fixture_changed_source_or_unavailable_version_never_gets_a_move_intent() {
        for change_source in [false, true] {
            let fixture = Fixture::new().unwrap();
            let path = fixture.run.path().canonicalize().unwrap();
            let directory = root(&path.join("originals")).unwrap();
            let versions = root(&path.join("retained")).unwrap();
            let member = &fixture.plan.members[0];
            let original = &fixture.originals[&member.id];
            if change_source {
                let mut file =
                    native::reopen_stage(directory.leaf(), &member.before, &original.identity)
                        .unwrap();
                assert_eq!(
                    PacedIo::shared()
                        .write_all(&mut file, b"changed", &AtomicBool::new(false))
                        .unwrap(),
                    IoStep::Completed(())
                );
                file.set_times(std::fs::FileTimes::new().set_modified(original.modified))
                    .unwrap();
                file.sync_all().unwrap();
            } else {
                let file =
                    native::reopen_stage(versions.leaf(), &member.id, &original.version).unwrap();
                drop(
                    native::publish_new(
                        versions.leaf(),
                        &member.id,
                        file,
                        "retained-unavailable-version",
                        &original.version,
                    )
                    .unwrap(),
                );
            }
            assert!(fixture.execute(&fixture.plan, None).is_err());
            assert_eq!(
                Fixture::cursor(&fixture.journal(&fixture.plan).unwrap(), &fixture.plan).unwrap(),
                (0, false)
            );
            for member in &fixture.plan.members {
                assert!(
                    native::reopen_stage(
                        directory.leaf(),
                        &member.before,
                        &fixture.originals[&member.id].identity
                    )
                    .is_ok()
                );
            }
            if !change_source {
                assert!(
                    native::reopen_stage(
                        versions.leaf(),
                        "retained-unavailable-version",
                        &original.version
                    )
                    .is_ok()
                );
            }
        }
    }
    #[test]
    fn native_rename_fixture_undo_preserves_a_new_occupant_of_an_old_name() {
        let mut fixture = Fixture::new().unwrap();
        assert!(fixture.execute(&fixture.plan, None).unwrap());
        let reverse = fixture.reverse_plan().unwrap();
        let directory =
            root(&fixture.run.path().canonicalize().unwrap().join("originals")).unwrap();
        let foreign = native::create_file(directory.leaf(), "Undo.JPG").unwrap();
        let foreign_identity = filesystem::identity(&foreign).unwrap();
        drop(foreign);
        assert!(fixture.execute(&reverse, None).is_err());
        assert_eq!(
            Fixture::cursor(&fixture.journal(&reverse).unwrap(), &reverse).unwrap(),
            (0, false)
        );
        assert!(native::reopen_stage(directory.leaf(), "Undo.JPG", &foreign_identity).is_ok());
        for member in &fixture.plan.members {
            assert!(
                native::reopen_stage(
                    directory.leaf(),
                    &member.after,
                    &fixture.originals[&member.id].identity
                )
                .is_ok()
            );
        }
    }
    #[test]
    fn native_rename_fixture_undo_restores_names_and_reconciles_interruption() {
        let mut fixture = Fixture::new().unwrap();
        assert!(fixture.execute(&fixture.plan, None).unwrap());
        let reverse = fixture.reverse_plan().unwrap();
        assert!(reverse.issues.is_empty());
        assert!(
            !fixture
                .execute(&reverse, Some(Fault::AfterMove(5)))
                .unwrap()
        );
        assert!(fixture.execute(&reverse, None).unwrap());
        let directory =
            root(&fixture.run.path().canonicalize().unwrap().join("originals")).unwrap();
        for member in &fixture.plan.members {
            assert!(
                native::reopen_stage(
                    directory.leaf(),
                    &member.before,
                    &fixture.originals[&member.id].identity
                )
                .is_ok()
            );
        }
    }
    #[test]
    fn native_rename_fixture_reconciles_each_intent_and_publication_without_replaying() {
        for step in 0..10 {
            for after in [false, true] {
                let fixture = Fixture::new().unwrap();
                let fault = if after {
                    Fault::AfterMove(step)
                } else {
                    Fault::AfterIntent(step)
                };
                assert!(!fixture.execute(&fixture.plan, Some(fault)).unwrap());
                assert!(fixture.execute(&fixture.plan, None).unwrap());
                let db = fixture.journal(&fixture.plan).unwrap();
                assert_eq!(Fixture::cursor(&db, &fixture.plan).unwrap(), (10, false));
                assert!(fixture.execute(&fixture.plan, None).unwrap());
                assert_eq!(Fixture::cursor(&db, &fixture.plan).unwrap(), (10, false));
            }
        }
    }
}
