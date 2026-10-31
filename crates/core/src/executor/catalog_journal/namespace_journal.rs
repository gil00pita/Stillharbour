//! Retained preparation facts, deliberately separate from copy approval/history.
use super::*;
use crate::executor::preparation::{self, Target};
use std::sync::atomic::AtomicBool;

const ANCHOR_PHASE: &str = "namespace_preparation_plan_v1";
const FACT_PHASE: &str = "namespace_preparation_fact_v1";
const MAX_FACT: usize = crate::operations::MAX_PLAN_BYTES;
// The anchor is deliberately smaller than the generic preparation input limit.
const MAX_FACT_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    binding: Binding,
    token: String,
    targets: Vec<Target>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fact {
    ordinal: u64,
    previous: String,
    item: Option<usize>,
    phase: String,
    evidence: serde_json::Value,
}
#[derive(Clone, Default)]
struct State {
    count: u64,
    bytes: usize,
    hash: String,
    completed: bool,
}
impl State {
    fn apply(&mut self, fact: &Fact, text: &str, scope: &Scope) -> Result<()> {
        if self.completed
            || fact.ordinal != self.count + 1
            || fact.previous != self.hash
            || fact.ordinal > (scope.targets.len() * 6 + 1024 * 2 + 3) as u64
            || fact.item.is_some_and(|item| item >= scope.targets.len())
            || text.len() > MAX_FACT
            || self.bytes.saturating_add(text.len()) > MAX_FACT_BYTES
        {
            return Err(failed(
                "Invalid or oversized namespace preparation history.",
            ));
        }
        let namespace = format!(".stillharbour-preparing-{}.partial", scope.token);
        match (fact.ordinal, fact.phase.as_str()) {
            (1, "namespace_prepare_intent") => {
                if fact.item.is_some()
                    || fact.evidence
                        != serde_json::json!({"policy":"native-name-preparation-v1","parent":scope.binding.plan.destination,"temporaryNamespace":namespace,"targets":scope.targets})
                {
                    return Err(failed("Namespace intent differs from its retained scope."));
                }
            }
            (2, "namespace_created") => {
                if fact.item.is_some()
                    || fact.evidence["temporaryNamespace"] != namespace
                    || fact.evidence["object"]["volume"] != scope.binding.plan.destination.volume
                {
                    return Err(failed(
                        "Namespace creation differs from its retained scope.",
                    ));
                }
            }
            (3.., "namespace_prepared") => {
                preparation::validate_record(
                    &fact.evidence,
                    &scope.binding.plan.destination,
                    &namespace,
                    &scope.targets,
                )?;
                let report = &fact.evidence["report"];
                let rows = report["rows"]
                    .as_array()
                    .ok_or_else(|| failed("Missing prepared namespace rows."))?;
                if fact.item.is_some()
                    || report["policy"] != "native-name-preparation-v1"
                    || report["temporaryNamespace"] != namespace
                    || report["executionAvailable"] != false
                    || !report["fingerprint"].as_str().is_some_and(hex)
                    || rows.len() != scope.targets.len()
                    || rows.iter().zip(&scope.targets).any(|(row, target)| {
                        row["id"] != target.id || row["targetRelativePath"] != target.path
                    })
                    || !fact.evidence["evidence"].is_array()
                {
                    return Err(failed(
                        "Prepared namespace differs from its retained manifest.",
                    ));
                }
                self.completed = true;
            }
            (
                3..,
                "namespace_directory_intent"
                | "namespace_directory_created"
                | "namespace_leaf_intent"
                | "namespace_leaf_created"
                | "namespace_stage_intent"
                | "namespace_stage_created",
            ) if fact.item.is_some() => {}
            _ => return Err(failed("Unexpected namespace preparation phase.")),
        }
        self.count += 1;
        self.bytes += text.len();
        self.hash = blake3::hash(text.as_bytes()).to_hex().to_string();
        Ok(())
    }
}
struct NamespaceJournal {
    catalog: Arc<Catalog>,
    scope: Scope,
    encoded: String,
    anchor: i64,
    state: Mutex<State>,
}
impl NamespaceJournal {
    fn bind(fixture: &Fixture, token: String) -> Result<Self> {
        crate::operations::validate_id(&token)?;
        let owner = fixture.owner.clone();
        let manifest = fixture.manifest.clone();
        let plan = fixture.plan.clone();
        let (scope, encoded, anchor, state) = fixture.library.catalog.call(move |db| {
            let tx = db.transaction()?;
            let draft = operation_store::load(&tx, &owner)?;
            let manifest = copy_manifest::load(&tx, &owner, &manifest)?;
            let scope = Scope {
                targets: manifest.files.iter().map(|file| Target { id: file.view.id.clone(), path: file.view.target_relative_path.clone() }).collect(),
                binding: Binding { policy: "catalog-copy-stream-v1".into(), catalog_id: draft.catalog_id, owner_id: owner, owner_digest: draft.view.digest, manifest_id: manifest.view.id, manifest_digest: manifest.view.digest, plan },
                token,
            };
            validate_binding(&tx, &scope.binding)?;
            let encoded = serde_json::to_string(&scope).map_err(|error| failed("Could not encode namespace scope.").caused_by(error))?;
            if encoded.len() > MAX_RECORD { return Err(failed("Namespace scope exceeds 64 KiB. Use a smaller selection.")); }
            let previous = {
                let mut query = tx.prepare("SELECT sequence,CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT 2")?;
                let mut rows = query.query(params![scope.binding.owner_id,scope.token,ANCHOR_PHASE])?;
                let previous = if let Some(row) = rows.next()? { Some((row.get::<_,i64>(0)?,row.get::<_,Option<String>>(1)?)) } else { None };
                if rows.next()?.is_some() { return Err(failed("Duplicate namespace preparation anchors.")); }
                previous
            };
            let anchor = if let Some((sequence, previous)) = previous {
                if previous.as_ref() != Some(&encoded) { return Err(failed("Namespace scope changed or exceeds its bound.")); }
                sequence
            } else {
                tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![scope.binding.owner_id,scope.token,ANCHOR_PHASE,encoded,crate::library::store::now()])?;
                tx.last_insert_rowid()
            };
            let mut state = State::default();
            {
                let mut query = tx.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=8388608 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT ?4")?;
                let mut rows = query.query(params![scope.binding.owner_id,scope.token,FACT_PHASE,(scope.targets.len()*6+1024*2+4) as i64])?;
                while let Some(row) = rows.next()? {
                    let text: Option<String> = row.get(0)?;
                    let text = text.ok_or_else(|| failed("Retained preparation fact exceeds 8 MiB."))?;
                    state.apply(&decode(&text,MAX_FACT)?,&text,&scope)?;
                }
            }
            tx.commit()?;
            Ok((scope,encoded,anchor,state))
        })?;
        Ok(Self {
            catalog: fixture.library.catalog.clone(),
            scope,
            encoded,
            anchor,
            state: Mutex::new(state),
        })
    }
}
impl EventJournal for NamespaceJournal {
    fn append(&self, item: Option<usize>, phase: &str, evidence: serde_json::Value) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| failed("Namespace history is unavailable."))?;
        let fact = Fact {
            ordinal: state.count + 1,
            previous: state.hash.clone(),
            item,
            phase: phase.into(),
            evidence,
        };
        let text = serde_json::to_string(&fact)
            .map_err(|error| failed("Could not encode preparation fact.").caused_by(error))?;
        let mut proposed = state.clone();
        proposed.apply(&fact, &text, &self.scope)?;
        let anchor = self.anchor;
        let encoded = self.encoded.clone();
        let scope = self.scope.clone();
        let count = state.count;
        self.catalog.call(move |db| {
            let tx = db.transaction()?;
            validate_binding(&tx,&scope.binding)?;
            let unchanged: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_journal WHERE sequence=?1 AND plan_id=?2 AND item_id=?3 AND phase=?4 AND evidence=?5) AND (SELECT count(*) FROM operation_journal WHERE plan_id=?2 AND item_id=?3 AND phase=?4)=1",params![anchor,scope.binding.owner_id,scope.token,ANCHOR_PHASE,encoded],|row|row.get(0))?;
            let existing: i64 = tx.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3",params![scope.binding.owner_id,scope.token,FACT_PHASE],|row|row.get(0))?;
            if !unchanged || u64::try_from(existing).ok()!=Some(count) { return Err(failed("Namespace preparation has another writer or changed anchor. Inspect retained objects.")); }
            tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![scope.binding.owner_id,scope.token,FACT_PHASE,text,crate::library::store::now()])?;
            tx.commit()?;
            Ok(())
        })?;
        *state = proposed;
        Ok(())
    }
}
pub(super) fn fixture_probe() -> Result<serde_json::Value> {
    use crate::jobs::operation_locks::{Access, Registry, Scope as NativeScope};
    let fixture = Fixture::with_nested(true)?;
    let (source, destination) = fixture.scope()?;
    let _lease = Registry::shared().try_acquire(vec![
        NativeScope::new(source, Access::Read),
        NativeScope::new(destination, Access::Write),
    ])?;
    let token = uuid::Uuid::new_v4().to_string();
    let journal = NamespaceJournal::bind(&fixture, token.clone())?;
    let prepared = preparation::prepare_named(
        _lease.directory(1).leaf(),
        &journal.scope.targets,
        &journal,
        &AtomicBool::new(false),
        &token,
    )?;
    let report = serde_json::to_value(&prepared.report)
        .map_err(|error| failed("Could not encode namespace receipt.").caused_by(error))?;
    drop(journal);
    let reopened = NamespaceJournal::bind(&fixture, token)?;
    let state = reopened
        .state
        .lock()
        .map_err(|_| failed("Namespace history is unavailable."))?;
    if !state.completed || !fixture.preserved()? || report["retainedStages"] != 2 {
        return Err(failed(
            "Catalog namespace fixture did not retain its completed evidence.",
        ));
    }
    let count = state.count;
    drop(state);
    let retry_blocked = preparation::prepare_named(
        _lease.directory(1).leaf(),
        &reopened.scope.targets,
        &reopened,
        &AtomicBool::new(false),
        &reopened.scope.token,
    )
    .is_err();
    if !retry_blocked
        || reopened
            .state
            .lock()
            .map_err(|_| failed("Namespace history is unavailable."))?
            .count
            != count
    {
        return Err(failed("Completed namespace preparation was replayed."));
    }
    Ok(
        serde_json::json!({"report":report,"retainedFacts":count,"reopenedCompleted":true,"replayBlocked":true,"sourceAndNeighborPreserved":true,"executionApproval":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_namespace_retains_actual_preparation_and_refuses_replay() {
        let receipt = fixture_probe().unwrap();
        assert_eq!(receipt["retainedFacts"], 13);
        assert_eq!(receipt["reopenedCompleted"], true);
        assert_eq!(receipt["executionApproval"], false);
    }

    #[test]
    fn namespace_journal_denial_and_revoked_root_create_nothing() {
        for revoke in [false, true] {
            let fixture = Fixture::new().unwrap();
            let token = uuid::Uuid::new_v4().to_string();
            let journal = NamespaceJournal::bind(&fixture, token.clone()).unwrap();
            fixture.library.catalog.call(move |db| {
                if revoke { db.execute("UPDATE roots SET mode='referenced'",[])?; }
                else { db.execute_batch("CREATE TRIGGER reject_namespace_fact BEFORE INSERT ON operation_journal WHEN NEW.phase='namespace_preparation_fact_v1' BEGIN SELECT RAISE(ABORT,'fixture journal denial'); END;")?; }
                Ok(())
            }).unwrap();
            let (_, output) = fixture.scope().unwrap();
            assert!(
                preparation::prepare_named(
                    output.leaf(),
                    &journal.scope.targets,
                    &journal,
                    &AtomicBool::new(false),
                    &token
                )
                .is_err()
            );
            assert!(
                !fixture
                    .destination
                    .join(format!(".stillharbour-preparing-{token}.partial"))
                    .exists()
            );
            assert_eq!(journal.state.lock().unwrap().count, 0);
            assert!(fixture.preserved().unwrap());
        }
    }

    #[test]
    fn interrupted_namespace_retains_object_and_blocks_stale_writer() {
        struct RejectCreation<'a>(&'a NamespaceJournal);
        impl EventJournal for RejectCreation<'_> {
            fn append(
                &self,
                item: Option<usize>,
                phase: &str,
                evidence: serde_json::Value,
            ) -> Result<()> {
                if phase == "namespace_created" {
                    return Err(failed("Injected journal rejection after native creation."));
                }
                self.0.append(item, phase, evidence)
            }
        }
        let fixture = Fixture::new().unwrap();
        let token = uuid::Uuid::new_v4().to_string();
        let journal = NamespaceJournal::bind(&fixture, token.clone()).unwrap();
        let stale = NamespaceJournal::bind(&fixture, token.clone()).unwrap();
        let (_, output) = fixture.scope().unwrap();
        assert!(
            preparation::prepare_named(
                output.leaf(),
                &journal.scope.targets,
                &RejectCreation(&journal),
                &AtomicBool::new(false),
                &token
            )
            .is_err()
        );
        let path = fixture
            .destination
            .join(format!(".stillharbour-preparing-{token}.partial"));
        let root = crate::library::native::open_root(&path, None).unwrap();
        let identity = Object::of(&root).unwrap();
        assert!(
            preparation::prepare_named(
                output.leaf(),
                &stale.scope.targets,
                &stale,
                &AtomicBool::new(false),
                &token
            )
            .is_err()
        );
        let reopened = NamespaceJournal::bind(&fixture, token.clone()).unwrap();
        assert_eq!(reopened.state.lock().unwrap().count, 1);
        assert!(!reopened.state.lock().unwrap().completed);
        assert!(
            preparation::prepare_named(
                output.leaf(),
                &reopened.scope.targets,
                &reopened,
                &AtomicBool::new(false),
                &token
            )
            .is_err()
        );
        assert!(
            identity
                .matches(&crate::library::native::open_root(&path, None).unwrap())
                .unwrap()
        );
        assert!(fixture.preserved().unwrap());
        // Hash chaining is corruption detection, never imported approval.
        let owner = fixture.owner.clone();
        fixture.library.catalog.call(move |db| {
            db.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,'namespace_preparation_fact_v1',?3,'fixture')",params![owner,token,serde_json::to_string(&Fact{ordinal:2,previous:"foreign".into(),item:None,phase:"namespace_created".into(),evidence:serde_json::json!({})}).unwrap()])?;
            Ok(())
        }).unwrap();
        assert!(NamespaceJournal::bind(&fixture, reopened.scope.token.clone()).is_err());
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn duplicate_namespace_anchor_after_binding_blocks_before_native_creation() {
        let fixture = Fixture::new().unwrap();
        let token = uuid::Uuid::new_v4().to_string();
        let journal = NamespaceJournal::bind(&fixture, token.clone()).unwrap();
        let anchor = journal.anchor;
        fixture.library.catalog.call(move |db| {
            db.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) SELECT plan_id,item_id,phase,evidence,created_at FROM operation_journal WHERE sequence=?",[anchor])?;
            Ok(())
        }).unwrap();
        let (_, output) = fixture.scope().unwrap();
        assert!(
            preparation::prepare_named(
                output.leaf(),
                &journal.scope.targets,
                &journal,
                &AtomicBool::new(false),
                &token
            )
            .is_err()
        );
        assert!(
            !fixture
                .destination
                .join(format!(".stillharbour-preparing-{token}.partial"))
                .exists()
        );
        assert!(NamespaceJournal::bind(&fixture, token).is_err());
        assert!(fixture.preserved().unwrap());
    }
}
