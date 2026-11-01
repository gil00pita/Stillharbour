//! Closed native experiment. Only this constructor's generated objects can be
//! moved. Retained local versions are NOT independent recovery or M4 evidence.
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
use rusqlite::{Connection, params};
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
    run: tempfile::TempDir,
    root_id: String,
    originals: BTreeMap<String, Original>,
    probes: Vec<super::Object>,
    plan: Plan,
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
        let mut members = [
            ("Case.JPG", "case.JPG"),
            ("Case.xmp", "case.xmp"),
            ("A.JPG", "B.JPG"),
            ("B.JPG", "A.JPG"),
            ("Undo.JPG", "Renamed.JPG"),
        ]
        .into_iter()
        .map(|(before, after)| Member {
            id: uuid(),
            before: before.into(),
            after: after.into(),
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
            let payload = format!("Stillharbour disposable rename member {index}\n").repeat(37);
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
        if !plan.issues.is_empty() {
            return Err(failed("Generated native rename planning conflicted."));
        }
        let result = Self {
            run,
            root_id,
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
        let mut inventory = self
            .probes
            .iter()
            .cloned()
            .map(|o| (o, native::EntryKind::File))
            .collect::<Vec<_>>();
        let mut revisions = BTreeMap::new();
        let io = PacedIo::shared();
        let cancel = AtomicBool::new(false);
        for (id, name) in locations {
            let original = &self.originals[id];
            let mut file = native::reopen_stage(parent, name, &original.identity)?;
            let mut version = native::reopen_stage(versions, id, &original.version)?;
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
            revisions.insert(id.clone(), source_native::revision(&file)?);
            inventory.push((super::Object::of(&file)?, native::EntryKind::File));
        }
        native::verify_entries(parent, &inventory)?;
        Ok(revisions)
    }
    fn execute(&self, plan: &Plan, fault: Option<(usize, bool)>) -> Result<bool> {
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
        let path = self.run.path().canonicalize()?;
        let directory = root(&path.join("originals"))?;
        let versions = root(&path.join("retained"))?;
        let _lease = Registry::shared().try_acquire(vec![
            Scope::new(root(&path.join("originals"))?, Access::Write),
            Scope::new(root(&path.join("retained"))?, Access::Read),
        ])?;
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
        while cursor < plan.steps.len() {
            let step = &plan.steps[cursor];
            let original = &self.originals[&step.member_id];
            if !intent {
                db.execute(
                    "INSERT INTO events(step,phase,revision) VALUES (?1,'intent',?2)",
                    params![cursor as i64, expected[&step.member_id]],
                )?;
            }
            if fault == Some((cursor, false)) {
                return Ok(false);
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
            if fault == Some((cursor, true)) {
                return Ok(false);
            }
            let mut current = self.locations(plan, parent, &db)?;
            current.insert(step.member_id.clone(), step.to.clone());
            let current_revisions = self.verify(parent, versions.leaf(), &current)?;
            db.execute(
                "INSERT INTO events(step,phase,revision) VALUES (?1,'done',?2)",
                params![cursor as i64, current_revisions[&step.member_id]],
            )?;
            expected.insert(
                step.member_id.clone(),
                current_revisions[&step.member_id].clone(),
            );
            cursor += 1;
            intent = false;
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
    let interrupted = !fixture.execute(&fixture.plan, Some((0, true)))?;
    let resumed = fixture.execute(&fixture.plan, None)?;
    if !interrupted || !resumed {
        return Err(failed("Native rename interruption was not reconciled."));
    }
    let reverse = fixture.reverse_plan()?;
    if !reverse.issues.is_empty() || !fixture.execute(&reverse, None)? {
        return Err(failed("Native fixture undo did not restore originals."));
    }
    Ok(
        serde_json::json!({"scope":"constructor-owned-generated-only","nativeOs":std::env::consts::OS,"members":fixture.plan.members.len(),"steps":fixture.plan.steps.len(),"interruptedAfterFirstRename":interrupted,"freshApprovedContinuation":resumed,"reverseSteps":reverse.steps.len(),"reverseRestoredOriginalNames":true,"completeBytesAndModifiedTimeVerified":true,"originalObjectsRetained":true,"localRetainedVersionsOnly":true,"independentRecoveryProven":false,"productionAuthorization":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(!fixture.execute(&reverse, Some((5, true))).unwrap());
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
                assert!(!fixture.execute(&fixture.plan, Some((step, after))).unwrap());
                assert!(fixture.execute(&fixture.plan, None).unwrap());
                let db = fixture.journal(&fixture.plan).unwrap();
                assert_eq!(Fixture::cursor(&db, &fixture.plan).unwrap(), (10, false));
                assert!(fixture.execute(&fixture.plan, None).unwrap());
                assert_eq!(Fixture::cursor(&db, &fixture.plan).unwrap(), (10, false));
            }
        }
    }
}
