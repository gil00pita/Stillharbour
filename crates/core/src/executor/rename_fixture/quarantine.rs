//! Generated-file relocation/quarantine infrastructure, not duplicate-removal
//! authority. It has no production constructor, expiry, purge or source unlink.
use super::super::Object;
use super::*;

struct Quarantine {
    fixture: Fixture,
    root: Object,
    volume: String,
    selected: Vec<String>,
}
struct Held {
    source: DirectoryChain,
    quarantine: DirectoryChain,
    versions: DirectoryChain,
}
struct Observed {
    positions: BTreeMap<String, bool>,
    revisions: BTreeMap<String, String>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RecordMember {
    id: String,
    original_leaf: String,
    quarantine_leaf: String,
    revision: String,
    object: Object,
    retained_version: Object,
    bytes: String,
    modified: SystemTime,
    created: Option<SystemTime>,
    readonly: bool,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    policy: String,
    id: String,
    role: String,
    volume: String,
    source: Object,
    quarantine: Object,
    versions: Object,
    members: Vec<RecordMember>,
    scope_revisions: BTreeMap<String, String>,
}
struct Progress {
    completed: usize,
    pending: bool,
    expected: BTreeMap<String, String>,
}
struct Execution<'a> {
    db: &'a Connection,
    held: &'a Held,
    id: &'a str,
    restore: bool,
    fault: Option<Fault>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    AfterIntent(usize),
    AfterMove(usize),
    DenyCompletion(usize),
    DenyContinuation,
}
impl Quarantine {
    fn new() -> Result<Self> {
        let fixture = Fixture::new()?;
        let container = root(&fixture.run.path().canonicalize()?)?;
        let directory = native::create_directory(container.leaf(), "quarantine")?;
        let selected = ["Case.JPG", "Case.xmp"]
            .into_iter()
            .map(|name| {
                fixture
                    .plan
                    .members
                    .iter()
                    .find(|m| m.before == name)
                    .map(|m| m.id.clone())
                    .ok_or_else(|| failed("Missing generated companion member."))
            })
            .collect::<Result<Vec<_>>>()?;
        let result = Self {
            root: Object::of(&directory)?,
            volume: source_native::volume_token(&directory)?,
            fixture,
            selected,
        };
        result.db()?;
        Ok(result)
    }
    fn leaf(id: &str) -> String {
        format!("{id}.retained")
    }
    fn db(&self) -> Result<Connection> {
        let db = Connection::open(self.fixture.run.path().join("relocation.sqlite"))?;
        db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS plans(id TEXT PRIMARY KEY,role TEXT UNIQUE NOT NULL CHECK(role IN ('quarantine','restore')),payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=65536));
            CREATE TABLE IF NOT EXISTS events(ordinal INTEGER PRIMARY KEY,plan_id TEXT NOT NULL REFERENCES plans(id),step INTEGER NOT NULL CHECK(step BETWEEN 0 AND 1),phase TEXT NOT NULL CHECK(phase IN ('intent','done')),revision TEXT NOT NULL,UNIQUE(plan_id,step,phase));
            CREATE TRIGGER IF NOT EXISTS plan_update_refused BEFORE UPDATE ON plans BEGIN SELECT RAISE(ABORT,'Retain immutable relocation plans'); END;
            CREATE TRIGGER IF NOT EXISTS plan_delete_refused BEFORE DELETE ON plans BEGIN SELECT RAISE(ABORT,'Retain relocation plans'); END;
            CREATE TRIGGER IF NOT EXISTS event_update_refused BEFORE UPDATE ON events BEGIN SELECT RAISE(ABORT,'Retain relocation facts'); END;
            CREATE TRIGGER IF NOT EXISTS event_delete_refused BEFORE DELETE ON events BEGIN SELECT RAISE(ABORT,'Retain relocation facts'); END;
            CREATE TRIGGER IF NOT EXISTS event_order BEFORE INSERT ON events WHEN
              (SELECT count(*) FROM events WHERE plan_id=NEW.plan_id) != NEW.step*2 + CASE NEW.phase WHEN 'done' THEN 1 ELSE 0 END
              BEGIN SELECT RAISE(ABORT,'Invalid relocation phase order'); END;
            CREATE TABLE IF NOT EXISTS continuations(id TEXT PRIMARY KEY,plan_id TEXT NOT NULL REFERENCES plans(id),payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=65536));
            CREATE TRIGGER IF NOT EXISTS continuation_update_refused BEFORE UPDATE ON continuations BEGIN SELECT RAISE(ABORT,'Retain continuation observations, never approval'); END;
            CREATE TRIGGER IF NOT EXISTS continuation_delete_refused BEFORE DELETE ON continuations BEGIN SELECT RAISE(ABORT,'Retain continuation observations'); END;")?;
        Ok(db)
    }
    fn open(&self) -> Result<Held> {
        let path = self.fixture.run.path().canonicalize()?;
        let held = Held {
            source: root(&path.join("originals"))?,
            quarantine: root(&path.join("quarantine"))?,
            versions: root(&path.join("retained"))?,
        };
        if !self.fixture.source_root.matches(held.source.leaf())?
            || !self.root.matches(held.quarantine.leaf())?
            || !self.fixture.version_root.matches(held.versions.leaf())?
        {
            return Err(failed("A relocation fixture root changed identity."));
        }
        if source_native::volume_token(held.source.leaf())? != self.volume
            || source_native::volume_token(held.quarantine.leaf())? != self.volume
            || source_native::volume_token(held.versions.leaf())? != self.volume
        {
            return Err(failed("Fixture relocation requires one native volume."));
        }
        Ok(held)
    }
    fn lease(&self, access: Access) -> Result<crate::jobs::operation_locks::Lease> {
        let path = self.fixture.run.path().canonicalize()?;
        Registry::shared().try_acquire(vec![
            Scope::new(root(&path.join("originals"))?, access),
            Scope::new(root(&path.join("quarantine"))?, access),
            Scope::new(root(&path.join("retained"))?, Access::Read),
        ])
    }
    fn inspect(&self, held: &Held) -> Result<Observed> {
        if !self.fixture.source_root.matches(held.source.leaf())?
            || !self.root.matches(held.quarantine.leaf())?
        {
            return Err(failed("Relocation root binding changed."));
        }
        let mut source = self
            .fixture
            .probes
            .iter()
            .cloned()
            .map(|o| (o, native::EntryKind::File))
            .collect::<Vec<_>>();
        let mut quarantine = Vec::new();
        let mut positions = BTreeMap::new();
        let mut revisions = BTreeMap::new();
        for member in &self.fixture.plan.members {
            let original = &self.fixture.originals[&member.id];
            let in_source =
                native::reopen_stage(held.source.leaf(), &member.before, &original.identity)
                    .is_ok();
            let leaf = Self::leaf(&member.id);
            let in_quarantine = self.selected.contains(&member.id)
                && native::reopen_stage(held.quarantine.leaf(), &leaf, &original.identity).is_ok();
            if in_source == in_quarantine {
                return Err(failed(
                    "Original location could not be verified uniquely; retain all evidence.",
                ));
            }
            let (parent, name) = if in_quarantine {
                (held.quarantine.leaf(), leaf.as_str())
            } else {
                (held.source.leaf(), member.before.as_str())
            };
            let (object, revision) =
                self.fixture
                    .verify_member(parent, held.versions.leaf(), &member.id, name)?;
            if in_quarantine {
                quarantine.push((object, native::EntryKind::File));
            } else {
                source.push((object, native::EntryKind::File));
            }
            positions.insert(member.id.clone(), in_quarantine);
            revisions.insert(member.id.clone(), revision);
        }
        native::verify_entries(held.source.leaf(), &source)?;
        native::verify_entries(held.quarantine.leaf(), &quarantine)?;
        Ok(Observed {
            positions,
            revisions,
        })
    }
    fn record(
        &self,
        id: String,
        restore: bool,
        revisions: BTreeMap<String, String>,
    ) -> Result<Record> {
        if revisions.len() != self.fixture.originals.len()
            || self
                .fixture
                .originals
                .keys()
                .any(|id| !revisions.contains_key(id))
            || revisions
                .values()
                .any(|r| r.len() != 64 || !r.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(failed("Incomplete or malformed relocation revision scope."));
        }
        crate::operations::validate_id(&id)?;
        let members = self
            .selected
            .iter()
            .map(|member_id| {
                let member = self
                    .fixture
                    .plan
                    .members
                    .iter()
                    .find(|m| m.id == *member_id)
                    .expect("constructor member");
                let original = &self.fixture.originals[member_id];
                RecordMember {
                    id: member_id.clone(),
                    original_leaf: member.before.clone(),
                    quarantine_leaf: Self::leaf(member_id),
                    revision: revisions[member_id].clone(),
                    object: Object {
                        volume: original.identity.volume,
                        file: original.identity.file,
                    },
                    retained_version: Object {
                        volume: original.version.volume,
                        file: original.version.file,
                    },
                    bytes: original.bytes.to_string(),
                    modified: original.modified,
                    created: original.created,
                    readonly: original.readonly,
                }
            })
            .collect();
        Ok(Record {
            policy: "generated-relocation-v2".into(),
            id,
            role: if restore { "restore" } else { "quarantine" }.into(),
            volume: self.volume.clone(),
            source: self.fixture.source_root.clone(),
            quarantine: self.root.clone(),
            versions: self.fixture.version_root.clone(),
            members,
            scope_revisions: revisions,
        })
    }
    fn encode(record: &Record) -> Result<String> {
        serde_json::to_string(record)
            .map_err(|error| failed("Cannot encode fixture relocation scope.").caused_by(error))
    }
    fn retained(&self, db: &Connection, restore: bool) -> Result<Record> {
        let role = if restore { "restore" } else { "quarantine" };
        let (id,payload):(String,Option<String>) = db.query_row("SELECT id,CASE WHEN length(CAST(payload AS BLOB))<=65536 THEN payload END FROM plans WHERE role=?1",[role],|row| Ok((row.get(0)?,row.get(1)?)))?;
        let record: Record = serde_json::from_str(
            &payload.ok_or_else(|| failed("Retained relocation scope exceeds its bound."))?,
        )
        .map_err(|error| {
            failed("Invalid retained relocation scope; no authority is imported.").caused_by(error)
        })?;
        let expected = self.record(id, restore, record.scope_revisions.clone())?;
        if Self::encode(&record)? != Self::encode(&expected)? {
            return Err(failed(
                "Retained relocation scope differs from its exact owned objects/mappings.",
            ));
        }
        Ok(record)
    }
    fn progress(db: &Connection, record: &Record) -> Result<Progress> {
        let mut query = db.prepare(
            "SELECT step,phase,revision FROM events WHERE plan_id=?1 ORDER BY ordinal LIMIT 5",
        )?;
        let rows = query.query_map([&record.id], |row| {
            Ok((
                row.get::<_, u32>(0)? as usize,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut result = Progress {
            completed: 0,
            pending: false,
            expected: record.scope_revisions.clone(),
        };
        for row in rows {
            let (step, phase, revision) = row?;
            if step != result.completed
                || step >= record.members.len()
                || revision.len() != 64
                || !revision.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(failed("Invalid retained relocation phase."));
            }
            let member = &record.members[step];
            match phase.as_str() {
                "intent" if !result.pending && revision == result.expected[&member.id] => {
                    result.pending = true
                }
                "done" if result.pending => {
                    result.pending = false;
                    result.completed += 1;
                    result.expected.insert(member.id.clone(), revision);
                }
                _ => {
                    return Err(failed(
                        "Retained relocation phase order/preconditions changed.",
                    ));
                }
            }
        }
        Ok(result)
    }
    fn resume(&self, restore: bool, fault: Option<Fault>) -> Result<bool> {
        let _lease = self.lease(Access::Write)?;
        let held = self.open()?;
        let observed = self.inspect(&held)?;
        let db = self.db()?;
        let record = self.retained(&db, restore)?;
        let progress = Self::progress(&db, &record)?;
        for (step, member) in record.members.iter().enumerate() {
            let pending = step == progress.completed && progress.pending;
            let expected_position = if step < progress.completed {
                !restore
            } else {
                restore
            };
            if !pending && observed.positions[&member.id] != expected_position {
                return Err(failed(
                    "Actual relocation positions do not match the retained phase.",
                ));
            }
        }
        for (id, revision) in &observed.revisions {
            let reconciled = progress.pending
                && record.members[progress.completed].id == *id
                && observed.positions[id] != restore;
            if !reconciled && progress.expected.get(id) != Some(revision) {
                return Err(failed("Relocation scope changed since its retained phase."));
            }
        }
        let attempts: i64 = db.query_row(
            "SELECT count(*) FROM continuations WHERE plan_id=?1",
            [&record.id],
            |row| row.get(0),
        )?;
        if attempts >= 64 {
            return Err(failed(
                "This fixture relocation has reached its explicit continuation bound.",
            ));
        }
        let id = uuid();
        let payload = serde_json::to_string(&serde_json::json!({"policy":"generated-relocation-continuation-v1","id":id,"parentId":record.id,"recordDigest":blake3::hash(Self::encode(&record)?.as_bytes()).to_hex().to_string(),"completed":progress.completed,"pending":progress.pending,"positions":observed.positions,"revisions":observed.revisions,"attempt":attempts+1}))
            .map_err(|error| failed("Cannot bind fresh relocation observations.").caused_by(error))?;
        let review = approval::Review::new(
            approval::Action::ContinueRelocationFixture,
            id.clone(),
            blake3::hash(payload.as_bytes()).to_hex().to_string(),
            self.fixture.root_id.clone(),
        )?;
        let approvals = approval::Approvals::new();
        let ticket = approvals.issue(review.clone())?;
        approvals.consume(&ticket, &review)?.authorize(&review)?;
        if fault == Some(Fault::DenyContinuation) {
            db.pragma_update(None, "query_only", true)?;
        }
        db.execute(
            "INSERT INTO continuations(id,plan_id,payload) VALUES (?1,?2,?3)",
            params![id, record.id, payload],
        )?;
        self.run_members(
            Execution {
                db: &db,
                held: &held,
                id: &record.id,
                restore,
                fault,
            },
            Progress {
                expected: observed.revisions,
                ..progress
            },
        )
    }
    fn execute(&self, restore: bool, fault: Option<Fault>) -> Result<bool> {
        let _lease = self.lease(Access::Write)?;
        let held = self.open()?;
        let observed = self.inspect(&held)?;
        if self
            .selected
            .iter()
            .any(|id| observed.positions[id] != restore)
        {
            return Err(failed(
                "Relocation needs the exact complete reviewed starting positions.",
            ));
        }
        let db = self.db()?;
        let role = if restore { "restore" } else { "quarantine" };
        let existing: i64 =
            db.query_row("SELECT count(*) FROM plans WHERE role=?1", [role], |row| {
                row.get(0)
            })?;
        if existing != 0 {
            return Err(failed(
                "Retained relocation needs inspection; never blindly replay its moves.",
            ));
        }
        if restore {
            let completed:i64 = db.query_row("SELECT count(*) FROM events e JOIN plans p ON p.id=e.plan_id WHERE p.role='quarantine' AND e.phase='done'",[],|row|row.get(0))?;
            if completed != 2 {
                return Err(failed(
                    "Restoration needs the completed retained quarantine plan.",
                ));
            }
        }
        let id = uuid();
        // This is a new immutable reverse plan when restoring, not reuse of the
        // quarantine approval. All native paths remain sealed in this factory.
        let payload =
            Self::encode(&self.record(id.clone(), restore, observed.revisions.clone())?)?;
        let digest = blake3::hash(payload.as_bytes()).to_hex().to_string();
        let review = approval::Review::new(
            approval::Action::RelocateFixture,
            id.clone(),
            digest,
            self.fixture.root_id.clone(),
        )?;
        let approvals = approval::Approvals::new();
        let ticket = approvals.issue(review.clone())?;
        approvals.consume(&ticket, &review)?.authorize(&review)?;
        db.execute(
            "INSERT INTO plans(id,role,payload) VALUES (?1,?2,?3)",
            params![id, role, payload],
        )?;
        self.run_members(
            Execution {
                db: &db,
                held: &held,
                id: &id,
                restore,
                fault,
            },
            Progress {
                completed: 0,
                pending: false,
                expected: observed.revisions,
            },
        )
    }
    fn run_members(&self, execution: Execution<'_>, mut progress: Progress) -> Result<bool> {
        let Execution {
            db,
            held,
            id,
            restore,
            fault,
        } = execution;
        let mut expected = progress.expected;
        for (step, member_id) in self.selected.iter().enumerate().skip(progress.completed) {
            let member = self
                .fixture
                .plan
                .members
                .iter()
                .find(|m| m.id == *member_id)
                .expect("constructor member");
            let original = &self.fixture.originals[member_id];
            let leaf = Self::leaf(member_id);
            let (from_parent, from, to_parent, to) = if restore {
                (
                    held.quarantine.leaf(),
                    leaf.as_str(),
                    held.source.leaf(),
                    member.before.as_str(),
                )
            } else {
                (
                    held.source.leaf(),
                    member.before.as_str(),
                    held.quarantine.leaf(),
                    leaf.as_str(),
                )
            };
            let current = self.inspect(held)?;
            if current.revisions != expected {
                return Err(failed(
                    "Relocation evidence changed before its next member.",
                ));
            }
            if !progress.pending {
                db.execute(
                    "INSERT INTO events(plan_id,step,phase,revision) VALUES (?1,?2,'intent',?3)",
                    params![id, step as i64, expected[member_id]],
                )?;
            }
            if fault == Some(Fault::AfterIntent(step)) {
                return Ok(false);
            }
            if current.positions[member_id] == restore {
                let file = native::reopen_stage(from_parent, from, &original.identity)?;
                if source_native::revision(&file)? != expected[member_id] {
                    return Err(failed("Relocation source changed after its intent."));
                }
                drop(native::relocate_new(
                    from_parent,
                    from,
                    file,
                    to_parent,
                    to,
                    &original.identity,
                )?);
            } else if !progress.pending {
                return Err(failed("An unrecorded move is not replay authority."));
            }
            if fault == Some(Fault::AfterMove(step)) {
                return Ok(false);
            }
            let verified = self.inspect(held)?;
            if fault == Some(Fault::DenyCompletion(step)) {
                db.pragma_update(None, "query_only", true)?;
            }
            db.execute(
                "INSERT INTO events(plan_id,step,phase,revision) VALUES (?1,?2,'done',?3)",
                params![id, step as i64, verified.revisions[member_id]],
            )?;
            expected = verified.revisions;
            progress.pending = false;
        }
        let final_state = self.inspect(held)?;
        if self
            .selected
            .iter()
            .any(|id| final_state.positions[id] == restore)
        {
            return Err(failed(
                "Relocation final positions did not match the reviewed plan.",
            ));
        }
        Ok(true)
    }
}

pub(super) fn probe() -> Result<serde_json::Value> {
    let fixture = Quarantine::new()?;
    if fixture.execute(false, Some(Fault::AfterMove(0)))?
        || !fixture.resume(false, None)?
        || fixture.execute(true, Some(Fault::AfterMove(0)))?
        || !fixture.resume(true, None)?
    {
        return Err(failed("Generated quarantine restoration did not finish."));
    }
    let _lease = fixture.lease(Access::Read)?;
    let observed = fixture.inspect(&fixture.open()?)?;
    if observed.positions.values().any(|position| *position) {
        return Err(failed("Generated originals were not restored."));
    }
    let db = fixture.db()?;
    let plans: i64 = db.query_row("SELECT count(*) FROM plans", [], |row| row.get(0))?;
    let events: i64 = db.query_row("SELECT count(*) FROM events", [], |row| row.get(0))?;
    let continuations: i64 =
        db.query_row("SELECT count(*) FROM continuations", [], |row| row.get(0))?;
    Ok(
        serde_json::json!({"scope":"generated-relocation-infrastructure-only","nativeOs":std::env::consts::OS,"quarantinedMembers":2,"companionsRestoredTogether":true,"originalObjectsAndBytesVerified":5,"planRecords":plans,"journalEvents":events,"freshContinuationRecords":continuations,"independentRecoveryProven":false,"duplicateRemovalAuthorized":false,"productionAuthorization":false,"automaticExpiry":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relocation_fixture_continuation_refuses_foreign_or_unknown_retained_scope_before_approval() {
        for variant in 0..3 {
            let fixture = Quarantine::new().unwrap();
            let observed = fixture.inspect(&fixture.open().unwrap()).unwrap();
            let id = uuid();
            let record = fixture
                .record(id.clone(), false, observed.revisions)
                .unwrap();
            let mut payload = serde_json::to_value(&record).unwrap();
            match variant {
                0 => payload["members"][0]["originalLeaf"] = serde_json::json!("foreign.JPG"),
                1 => payload["source"]["file"] = serde_json::json!(0),
                _ => payload["importedApproval"] = serde_json::json!("never-authority"),
            }
            let db = fixture.db().unwrap();
            db.execute(
                "INSERT INTO plans(id,role,payload) VALUES (?1,'quarantine',?2)",
                params![id, payload.to_string()],
            )
            .unwrap();
            assert!(fixture.resume(false, None).is_err());
            assert_eq!(
                db.query_row("SELECT count(*) FROM continuations", [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                db.query_row("SELECT count(*) FROM events", [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert!(
                fixture
                    .inspect(&fixture.open().unwrap())
                    .unwrap()
                    .positions
                    .values()
                    .all(|q| !q)
            );
        }
    }
    #[test]
    fn relocation_fixture_continuation_reconciles_both_directions_at_every_retained_boundary() {
        for restore in [false, true] {
            for step in [0, 1] {
                for variant in 0..3 {
                    let fixture = Quarantine::new().unwrap();
                    if restore {
                        assert!(fixture.execute(false, None).unwrap());
                    }
                    let fault = match variant {
                        0 => Fault::AfterIntent(step),
                        1 => Fault::AfterMove(step),
                        _ => Fault::DenyCompletion(step),
                    };
                    let result = fixture.execute(restore, Some(fault));
                    if variant == 2 {
                        assert!(result.is_err());
                    } else {
                        assert!(!result.unwrap());
                    }
                    assert!(fixture.resume(restore, None).unwrap());
                    let db = fixture.db().unwrap();
                    let record = fixture.retained(&db, restore).unwrap();
                    assert_eq!(Quarantine::progress(&db, &record).unwrap().completed, 2);
                    assert_eq!(
                        db.query_row(
                            "SELECT count(*) FROM events WHERE plan_id=?1",
                            [&record.id],
                            |row| row.get::<_, i64>(0)
                        )
                        .unwrap(),
                        4
                    );
                    assert_eq!(
                        db.query_row(
                            "SELECT count(*) FROM continuations WHERE plan_id=?1",
                            [&record.id],
                            |row| row.get::<_, i64>(0)
                        )
                        .unwrap(),
                        1
                    );
                }
            }
        }
    }
    #[test]
    fn relocation_fixture_continuation_can_stop_again_and_a_failed_attempt_record_moves_nothing() {
        let fixture = Quarantine::new().unwrap();
        assert!(!fixture.execute(false, Some(Fault::AfterMove(0))).unwrap());
        assert!(
            fixture
                .resume(false, Some(Fault::DenyContinuation))
                .is_err()
        );
        let held = fixture.open().unwrap();
        let observed = fixture.inspect(&held).unwrap();
        assert_eq!(observed.positions.values().filter(|q| **q).count(), 1);
        assert!(!fixture.resume(false, Some(Fault::AfterMove(1))).unwrap());
        assert!(fixture.resume(false, None).unwrap());
        let db = fixture.db().unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM continuations", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert!(
            db.execute("UPDATE continuations SET payload='{}'", [])
                .is_err()
        );
        assert!(db.execute("DELETE FROM continuations", []).is_err());
    }
    #[test]
    fn relocation_fixture_journal_scope_and_facts_cannot_be_rewritten_or_deleted() {
        let fixture = Quarantine::new().unwrap();
        assert!(fixture.execute(false, None).unwrap());
        let db = fixture.db().unwrap();
        assert!(db.execute("UPDATE plans SET payload='{}'", []).is_err());
        assert!(db.execute("DELETE FROM plans", []).is_err());
        assert!(db.execute("UPDATE events SET phase='intent'", []).is_err());
        assert!(db.execute("DELETE FROM events", []).is_err());
        assert_eq!(
            db.query_row("SELECT count(*) FROM events", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            4
        );
        drop(db);
        assert!(fixture.execute(true, None).unwrap());
    }
    #[test]
    fn relocation_fixture_roundtrip_retains_originals_and_reverse_scope() {
        let receipt = probe().unwrap();
        assert_eq!(receipt["planRecords"], 2);
        assert_eq!(receipt["journalEvents"], 8);
        assert_eq!(receipt["productionAuthorization"], false);
    }
    #[test]
    fn relocation_fixture_interruptions_retain_actual_objects_and_refuse_blind_replay() {
        for step in [0, 1] {
            for variant in 0..3 {
                let fixture = Quarantine::new().unwrap();
                let fault = match variant {
                    0 => Fault::AfterIntent(step),
                    1 => Fault::AfterMove(step),
                    _ => Fault::DenyCompletion(step),
                };
                let result = fixture.execute(false, Some(fault));
                if variant == 2 {
                    assert!(result.is_err());
                } else {
                    assert!(!result.unwrap());
                }
                let _lease = fixture.lease(Access::Read).unwrap();
                let observed = fixture.inspect(&fixture.open().unwrap()).unwrap();
                let count = observed
                    .positions
                    .values()
                    .filter(|position| **position)
                    .count();
                assert_eq!(count, step + usize::from(variant != 0));
                drop(_lease);
                assert!(fixture.execute(false, None).is_err());
            }
        }
    }
    #[test]
    fn relocation_fixture_restore_preserves_a_new_original_path_occupant() {
        let fixture = Quarantine::new().unwrap();
        assert!(fixture.execute(false, None).unwrap());
        let held = fixture.open().unwrap();
        let foreign = native::create_file(held.source.leaf(), "Case.JPG").unwrap();
        let identity = filesystem::identity(&foreign).unwrap();
        drop(foreign);
        assert!(fixture.execute(true, None).is_err());
        assert!(native::reopen_stage(held.source.leaf(), "Case.JPG", &identity).is_ok());
        for id in &fixture.selected {
            assert!(
                native::reopen_stage(
                    held.quarantine.leaf(),
                    &Quarantine::leaf(id),
                    &fixture.fixture.originals[id].identity
                )
                .is_ok()
            );
        }
        assert_eq!(
            fixture
                .db()
                .unwrap()
                .query_row(
                    "SELECT count(*) FROM plans WHERE role='restore'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}
