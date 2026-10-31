//! Retained preparation facts, deliberately separate from copy approval/history.
use super::*;
use crate::executor::preparation::{self, Target};
use std::sync::atomic::AtomicBool;

const ANCHOR_PHASE: &str = "namespace_preparation_plan_v2";
const FACT_PHASE: &str = "namespace_preparation_fact_v2";
const MAX_FACT: usize = crate::operations::MAX_PLAN_BYTES;
// The anchor is deliberately smaller than the generic preparation input limit.
const MAX_FACT_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reference {
    token: String,
    scope_digest: String,
    fingerprint: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    sources: crate::executor::sources::Snapshot,
    token: String,
    namespace: String,
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
        preparation::validate_namespace(&scope.token, &scope.namespace)?;
        let namespace = scope.namespace.as_str();
        match (fact.ordinal, fact.phase.as_str()) {
            (1, "namespace_prepare_intent") => {
                if fact.item.is_some()
                    || fact.evidence
                        != serde_json::json!({"policy":"native-name-preparation-v1","parent":scope.sources.destination,"temporaryNamespace":namespace,"targets":scope.targets})
                {
                    return Err(failed("Namespace intent differs from its retained scope."));
                }
            }
            (2, "namespace_created") => {
                if fact.item.is_some()
                    || fact.evidence["temporaryNamespace"] != namespace
                    || fact.evidence["object"]["volume"] != scope.sources.destination.volume
                {
                    return Err(failed(
                        "Namespace creation differs from its retained scope.",
                    ));
                }
            }
            (3.., "namespace_prepared") => {
                preparation::validate_record(
                    &fact.evidence,
                    &scope.sources.destination,
                    namespace,
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
    #[cfg(test)]
    fn bind(fixture: &Fixture, token: String) -> Result<Self> {
        Self::bind_kind(fixture, token, false)
    }
    fn bind_kind(fixture: &Fixture, token: String, organized_output: bool) -> Result<Self> {
        crate::operations::validate_id(&token)?;
        let owner = fixture.owner.clone();
        let selected = token.clone();
        let previous=fixture.library.catalog.call(move|db| {
            use rusqlite::OptionalExtension;
            let previous:Option<Option<String>>=db.query_row("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT 1",params![owner,selected,ANCHOR_PHASE],|row|row.get(0)).optional()?;
            previous.map(|text|decode::<Scope>(&text.ok_or_else(||failed("Namespace scope exceeds 64 KiB."))?,MAX_RECORD).map(|scope|scope.sources)).transpose()
        })?;
        let sources = if let Some(previous) = previous {
            previous
        } else {
            crate::executor::sources::Snapshot::build(
                &fixture.library.catalog,
                fixture.owner.clone(),
                fixture.manifest.clone(),
            )?
        };
        let (scope, encoded, anchor, state) = fixture.library.catalog.call(move |db| {
            let tx = db.transaction()?;
            let scope = Scope {
                targets: sources.files.iter().map(|file| Target { id: file.id.clone(), path: file.target.clone() }).collect(),
                sources,
                namespace:if organized_output {format!("Stillharbour organized {token}")} else {format!(".stillharbour-preparing-{token}.partial")},
                token,
            };
            scope.sources.validate(&tx)?;
            let encoded = serde_json::to_string(&scope).map_err(|error| failed("Could not encode namespace scope.").caused_by(error))?;
            if encoded.len() > MAX_RECORD { return Err(failed("Namespace scope exceeds 64 KiB. Use a smaller selection.")); }
            let previous = {
                let mut query = tx.prepare("SELECT sequence,CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT 2")?;
                let mut rows = query.query(params![scope.sources.owner_id,scope.token,ANCHOR_PHASE])?;
                let previous = if let Some(row) = rows.next()? { Some((row.get::<_,i64>(0)?,row.get::<_,Option<String>>(1)?)) } else { None };
                if rows.next()?.is_some() { return Err(failed("Duplicate namespace preparation anchors.")); }
                previous
            };
            let anchor = if let Some((sequence, previous)) = previous {
                if previous.as_ref() != Some(&encoded) { return Err(failed("Namespace scope changed or exceeds its bound.")); }
                sequence
            } else {
                tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![scope.sources.owner_id,scope.token,ANCHOR_PHASE,encoded,crate::library::store::now()])?;
                tx.last_insert_rowid()
            };
            let mut state = State::default();
            {
                let mut query = tx.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=8388608 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT ?4")?;
                let mut rows = query.query(params![scope.sources.owner_id,scope.token,FACT_PHASE,(scope.targets.len()*6+1024*2+4) as i64])?;
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
            scope.sources.validate(&tx)?;
            let unchanged: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_journal WHERE sequence=?1 AND plan_id=?2 AND item_id=?3 AND phase=?4 AND evidence=?5) AND (SELECT count(*) FROM operation_journal WHERE plan_id=?2 AND item_id=?3 AND phase=?4)=1",params![anchor,scope.sources.owner_id,scope.token,ANCHOR_PHASE,encoded],|row|row.get(0))?;
            let existing: i64 = tx.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3",params![scope.sources.owner_id,scope.token,FACT_PHASE],|row|row.get(0))?;
            if !unchanged || u64::try_from(existing).ok()!=Some(count) { return Err(failed("Namespace preparation has another writer or changed anchor. Inspect retained objects.")); }
            tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![scope.sources.owner_id,scope.token,FACT_PHASE,text,crate::library::store::now()])?;
            tx.commit()?;
            Ok(())
        })?;
        *state = proposed;
        Ok(())
    }
}
fn load_preparation(
    db: &Connection,
    owner: &str,
    reference: &Reference,
) -> Result<(Scope, serde_json::Value)> {
    crate::operations::validate_id(&reference.token)?;
    if !hex(&reference.scope_digest) || !hex(&reference.fingerprint) {
        return Err(failed("Invalid prepared namespace reference."));
    }
    let mut query=db.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT 2")?;
    let mut rows = query.query(params![owner, reference.token, ANCHOR_PHASE])?;
    let text: Option<String> = rows
        .next()?
        .ok_or_else(|| failed("Prepared namespace anchor is missing."))?
        .get(0)?;
    let text = text.ok_or_else(|| failed("Prepared namespace anchor exceeds 64 KiB."))?;
    if rows.next()?.is_some()
        || blake3::hash(text.as_bytes()).to_hex().as_str() != reference.scope_digest
    {
        return Err(failed(
            "Prepared namespace anchor changed or is duplicated.",
        ));
    }
    let scope: Scope = decode(&text, MAX_RECORD)?;
    preparation::validate_namespace(&scope.token, &scope.namespace)?;
    if scope.token != reference.token || scope.sources.owner_id != owner {
        return Err(failed("Foreign or recursive prepared namespace binding."));
    }
    scope.sources.validate(db)?;
    let manifest = copy_manifest::load(db, owner, &scope.sources.manifest_id)?;
    let targets: Vec<Target> = manifest
        .files
        .iter()
        .map(|file| Target {
            id: file.view.id.clone(),
            path: file.view.target_relative_path.clone(),
        })
        .collect();
    if scope.targets != targets {
        return Err(failed(
            "Namespace target IDs/paths differ from the retained manifest.",
        ));
    }
    let mut state = State::default();
    let mut completion = None;
    let mut query=db.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=8388608 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT ?4")?;
    let mut rows = query.query(params![
        owner,
        reference.token,
        FACT_PHASE,
        (scope.targets.len() * 6 + 1024 * 2 + 4) as i64
    ])?;
    while let Some(row) = rows.next()? {
        let text: Option<String> = row.get(0)?;
        let text = text.ok_or_else(|| failed("Prepared namespace fact exceeds 8 MiB."))?;
        let fact: Fact = decode(&text, MAX_FACT)?;
        state.apply(&fact, &text, &scope)?;
        if fact.phase == "namespace_prepared" {
            completion = Some(fact.evidence);
        }
    }
    let completion = completion
        .filter(|_| state.completed)
        .ok_or_else(|| failed("Namespace preparation is incomplete. Inspect retained objects."))?;
    if completion["report"]["fingerprint"] != reference.fingerprint {
        return Err(failed("Prepared namespace fingerprint changed."));
    }
    Ok((scope, completion))
}
pub(super) fn validate_copy_binding(db: &Connection, binding: &Binding) -> Result<()> {
    let reference = binding
        .namespace
        .as_ref()
        .ok_or_else(|| failed("Missing prepared namespace reference."))?;
    let (scope, completion) = load_preparation(db, &binding.owner_id, reference)?;
    let base = &scope.sources;
    if binding.policy != "catalog-copy-stream-v1"
        || binding.catalog_id != base.catalog_id
        || binding.owner_id != base.owner_id
        || binding.owner_digest != base.owner_digest
        || binding.manifest_id != base.manifest_id
        || binding.manifest_digest != base.manifest_digest
    {
        return Err(failed(
            "Prepared copy scope differs from its catalog/manifest binding.",
        ));
    }
    let namespace = scope.namespace.as_str();
    let expected = preparation::project_copy_plan(
        &completion,
        base,
        namespace,
        &scope.targets,
        &binding.plan.id,
    )?;
    if serde_json::to_value(expected).ok() != serde_json::to_value(&binding.plan).ok() {
        return Err(failed(
            "Prepared copy differs from its exact retained stage/parent/source evidence.",
        ));
    }
    Ok(())
}
pub(super) fn check_copy_stream_admission(db: &Connection, binding: &Binding) -> Result<()> {
    let mut query=db.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND phase=?2 ORDER BY sequence LIMIT 65")?;
    let mut rows = query.query(params![binding.owner_id, PLAN_PHASE])?;
    let mut count = 0;
    let mut existing = false;
    while let Some(row) = rows.next()? {
        count += 1;
        let text: Option<String> = row.get(0)?;
        let previous: Binding = decode(
            &text.ok_or_else(|| failed("Retained copy anchor exceeds 64 KiB."))?,
            MAX_RECORD,
        )?;
        existing |= previous.plan.id == binding.plan.id;
        if let (Some(left), Some(right)) = (&binding.namespace, &previous.namespace)
            && left.token == right.token
            && previous.plan.id != binding.plan.id
        {
            return Err(failed(
                "This prepared namespace already belongs to another copy stream. Open its recovery inspection.",
            ));
        }
    }
    if count > 64 || (count == 64 && !existing) {
        return Err(failed(
            "This draft has reached its 64 retained copy-stream limit.",
        ));
    }
    Ok(())
}
pub(super) fn fixture_probe() -> Result<serde_json::Value> {
    let mut receipt = fixture_case(None)?;
    receipt["interruptedAfterPublish"] =
        fixture_case(Some(crate::executor::CopyFault::AfterPublish))?;
    Ok(receipt)
}
fn fixture_case(fault: Option<crate::executor::CopyFault>) -> Result<serde_json::Value> {
    fixture_case_for(Fixture::with_nested(true)?, fault)
}
fn fixture_case_for(
    fixture: Fixture,
    fault: Option<crate::executor::CopyFault>,
) -> Result<serde_json::Value> {
    use crate::jobs::operation_locks::{Access, Registry, Scope as NativeScope};
    let token = uuid::Uuid::new_v4().to_string();
    let journal = NamespaceJournal::bind_kind(&fixture, token.clone(), true)?;
    let (source, destination) = fixture.scope()?;
    let _lease = Registry::shared().try_acquire(vec![
        NativeScope::new(source, Access::Read),
        NativeScope::new(destination, Access::Write),
    ])?;
    let prepared = preparation::prepare_namespace(
        _lease.directory(1).leaf(),
        &journal.scope.targets,
        &journal,
        &AtomicBool::new(false),
        &token,
        &journal.scope.namespace,
    )?;
    let report = serde_json::to_value(&prepared.report)
        .map_err(|error| failed("Could not encode namespace receipt.").caused_by(error))?;
    drop(journal);
    let reopened = NamespaceJournal::bind_kind(&fixture, token, true)?;
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
    let retry_blocked = preparation::prepare_namespace(
        _lease.directory(1).leaf(),
        &reopened.scope.targets,
        &reopened,
        &AtomicBool::new(false),
        &reopened.scope.token,
        &reopened.scope.namespace,
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
    let reference = Reference {
        token: reopened.scope.token.clone(),
        scope_digest: blake3::hash(reopened.encoded.as_bytes())
            .to_hex()
            .to_string(),
        fingerprint: report["fingerprint"]
            .as_str()
            .ok_or_else(|| failed("Missing namespace fingerprint."))?
            .into(),
    };
    let owner = fixture.owner.clone();
    let supplied = reference.clone();
    let stream = uuid::Uuid::new_v4().to_string();
    let plan = fixture.library.catalog.call(move |db| {
        let (scope, completion) = load_preparation(db, &owner, &supplied)?;
        preparation::project_copy_plan(
            &completion,
            &scope.sources,
            &scope.namespace,
            &scope.targets,
            &stream,
        )
    })?;
    let copy = CatalogJournal::bind_with_namespace(
        fixture.library.catalog.clone(),
        fixture.owner.clone(),
        fixture.manifest.clone(),
        plan.clone(),
        Some(reference.clone()),
    )?;
    let root = crate::executor::output_chain(
        &fixture.destination.join(
            report["temporaryNamespace"]
                .as_str()
                .ok_or_else(|| failed("Missing namespace name."))?,
        ),
    )?;
    let validate = || {
        fixture.scope()?;
        let entry = crate::executor::native::lookup(
            _lease.directory(1).leaf(),
            report["temporaryNamespace"]
                .as_str()
                .ok_or_else(|| failed("Missing namespace name."))?,
        )?
        .ok_or_else(|| failed("Prepared namespace disappeared."))?;
        if entry.object != plan.destination
            || entry.kind != crate::executor::native::EntryKind::Directory
        {
            return Err(failed("Prepared namespace changed membership."));
        }
        Ok(())
    };
    let cancel = AtomicBool::new(false);
    let io = crate::jobs::paced_io::PacedIo::shared();
    let before = plan
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            crate::executor::inspect_item(
                file,
                index,
                &copy,
                _lease.directory(0).leaf(),
                root.leaf(),
                &io,
                &cancel,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    if before
        .iter()
        .any(|state| *state != crate::executor::RecoveredItem::PreparedEmptyStage)
    {
        return Err(failed(
            "Prepared catalog copy did not start with identified empty stages.",
        ));
    }
    let context = crate::executor::CopyContext {
        plan: &plan,
        journal: &copy,
        source_root: _lease.directory(0).leaf(),
        destination: root.leaf(),
        io: &io,
        cancel: &cancel,
        validate_scope: &validate,
    };
    let outcome = context.execute(fault)?;
    drop(copy);
    let copy = CatalogJournal::bind_with_namespace(
        fixture.library.catalog.clone(),
        fixture.owner.clone(),
        fixture.manifest.clone(),
        plan.clone(),
        Some(reference.clone()),
    )?;
    let after = plan
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            crate::executor::inspect_item(
                file,
                index,
                &copy,
                _lease.directory(0).leaf(),
                root.leaf(),
                &io,
                &cancel,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let published = if fault.is_none() { plan.files.len() } else { 1 };
    let expected_outcome = if fault.is_none() {
        crate::executor::CopyOutcome::CompletedVerified
    } else {
        crate::executor::CopyOutcome::Interrupted
    };
    if outcome != expected_outcome
        || after.iter().enumerate().any(|(index, state)| {
            *state
                != if index < published {
                    crate::executor::RecoveredItem::PublishedVerified
                } else {
                    crate::executor::RecoveredItem::PreparedEmptyStage
                }
        })
        || !fixture.preserved()?
    {
        return Err(failed(
            "Prepared catalog copy failed complete publication/readback.",
        ));
    }
    let mut same_objects = 0;
    for file in plan.files.iter().take(published) {
        let parent = crate::executor::destination_parent(root.leaf(), file)?;
        let output = crate::executor::native::read(&parent, &file.target)?;
        if !file
            .prepared_stage
            .as_ref()
            .ok_or_else(|| failed("Missing prepared stage identity."))?
            .object
            .matches(&output)?
        {
            return Err(failed(
                "Published output replaced its prepared object identity.",
            ));
        }
        same_objects += 1;
    }
    let replay_blocked = crate::executor::CopyContext {
        plan: &plan,
        journal: &copy,
        source_root: _lease.directory(0).leaf(),
        destination: root.leaf(),
        io: &io,
        cancel: &cancel,
        validate_scope: &validate,
    }
    .execute(None)
    .is_err_and(|error| error.code == ErrorCode::Conflict);
    if !replay_blocked {
        return Err(failed("Prepared catalog copy allowed blind replay."));
    }
    let mut second = plan.clone();
    second.id = uuid::Uuid::new_v4().to_string();
    second.digest = second.digest()?;
    let duplicate_blocked = CatalogJournal::bind_with_namespace(
        fixture.library.catalog.clone(),
        fixture.owner.clone(),
        fixture.manifest.clone(),
        second,
        Some(reference),
    )
    .is_err();
    if !duplicate_blocked {
        return Err(failed("Prepared namespace admitted another copy stream."));
    }
    Ok(
        serde_json::json!({"report":report,"retainedFacts":count,"reopenedCompleted":true,"replayBlocked":true,"sourceAndNeighborPreserved":true,"executionApproval":false,"preparedCopy":{"outcome":outcome,"before":before,"after":after,"journalEvents":copy.history_count()?,"secondStreamBlocked":duplicate_blocked,"samePreparedObjectsPublished":same_objects,"replayBlocked":replay_blocked}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_source_planning_copies_future_target_parents_without_creating_them_during_preview() {
        use crate::operations::{
            BundleConfirmationInput, BundleRelation, CopyManifestInput, PlanDraftInput,
        };
        use crate::organization::{MissingDatePolicy, OrganizationTemplate};
        let mut fixture = Fixture::with_nested(true).unwrap();
        let owner = fixture.owner.clone();
        let source = fixture
            .library
            .catalog
            .call(move |db| Ok(operation_store::load(db, &owner)?.sources[0].clone()))
            .unwrap();
        let template = OrganizationTemplate {
            pattern: "future/nested/{original_stem}.{ext}".into(),
            event: String::new(),
            missing_date: MissingDatePolicy::Block,
        };
        let preview = fixture
            .library
            .organization_preview(vec![source.id.clone()], template.clone())
            .unwrap();
        let root = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let draft = fixture
            .library
            .organization_plan_create(
                PlanDraftInput {
                    asset_ids: vec![source.id.clone()],
                    template,
                    preview_fingerprint: preview.fingerprint,
                    template_reference: None,
                },
                fixture.destination.clone(),
                (
                    crate::filesystem::identity(&root).unwrap(),
                    fixture.plan.destination_volume.clone(),
                ),
            )
            .unwrap();
        drop(root);
        let inventory = fixture
            .library
            .organization_bundle_inventory(source.id.clone(), source.revision.clone())
            .unwrap();
        let decision = fixture
            .library
            .organization_bundle_confirm(BundleConfirmationInput {
                plan_id: draft.plan.id.clone(),
                asset_id: source.id,
                source_revision: source.revision,
                inventory_fingerprint: inventory.fingerprint,
                companion_ids: inventory
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.selectable && entry.relation != BundleRelation::Additional
                    })
                    .map(|entry| entry.id.clone())
                    .collect(),
                reviewed_all_entries: true,
            })
            .unwrap();
        let manifest = fixture
            .library
            .organization_copy_manifest(CopyManifestInput {
                plan_id: draft.plan.id.clone(),
                bundle_confirmation_ids: vec![decision.id],
            })
            .unwrap();
        fixture.owner = draft.plan.id;
        fixture.manifest = manifest.id;
        let snapshot = crate::executor::sources::Snapshot::build(
            &fixture.library.catalog,
            fixture.owner.clone(),
            fixture.manifest.clone(),
        )
        .unwrap();
        assert!(
            snapshot
                .files
                .iter()
                .all(|file| file.target.starts_with("future/nested/"))
        );
        assert!(!fixture.destination.join("future").exists());
        assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 2);
        assert!(fixture.preserved().unwrap());
        let receipt = fixture_case_for(fixture, None).unwrap();
        assert_eq!(receipt["retainedFacts"], 15);
        assert_eq!(receipt["preparedCopy"]["samePreparedObjectsPublished"], 2);
        assert_eq!(
            receipt["preparedCopy"]["after"],
            serde_json::json!(["published_verified", "published_verified"])
        );
        assert!(
            receipt["report"]["temporaryNamespace"]
                .as_str()
                .unwrap()
                .starts_with("Stillharbour organized ")
        );
    }
    #[test]
    fn source_snapshot_refuses_active_destination_writers_and_changed_members_without_outputs() {
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let fixture = Fixture::with_nested(true).unwrap();
        let (_, destination) = fixture.scope().unwrap();
        let writer = Registry::shared()
            .try_acquire(vec![Scope::new(destination, Access::Write)])
            .unwrap();
        let blocked = crate::executor::sources::Snapshot::build(
            &fixture.library.catalog,
            fixture.owner.clone(),
            fixture.manifest.clone(),
        )
        .err()
        .unwrap();
        assert_eq!(blocked.code, ErrorCode::Busy);
        assert!(fixture.preserved().unwrap());
        drop(writer);
        let path = fixture.source.join("source-folder/Photo.JPG");
        let mut changed = std::fs::read(&path).unwrap();
        changed[0] ^= 0xff;
        std::fs::write(&path, &changed).unwrap();
        assert!(
            crate::executor::sources::Snapshot::build(
                &fixture.library.catalog,
                fixture.owner.clone(),
                fixture.manifest.clone()
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), changed);
        assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 2);
        assert_eq!(
            std::fs::read(fixture.destination.join("destination-only.bin")).unwrap(),
            NEIGHBOR_BYTES
        );
    }
    #[test]
    fn catalog_namespace_retains_actual_preparation_and_refuses_replay() {
        let receipt = fixture_probe().unwrap();
        assert_eq!(receipt["retainedFacts"], 13);
        assert_eq!(receipt["reopenedCompleted"], true);
        assert_eq!(receipt["executionApproval"], false);
        assert_eq!(receipt["preparedCopy"]["journalEvents"], 13);
        assert_eq!(receipt["preparedCopy"]["samePreparedObjectsPublished"], 2);
        assert_eq!(receipt["preparedCopy"]["secondStreamBlocked"], true);
        assert_eq!(
            receipt["interruptedAfterPublish"]["preparedCopy"]["journalEvents"],
            5
        );
        assert_eq!(
            receipt["interruptedAfterPublish"]["preparedCopy"]["samePreparedObjectsPublished"],
            1
        );
        assert_eq!(
            receipt["interruptedAfterPublish"]["preparedCopy"]["after"],
            serde_json::json!(["published_verified", "prepared_empty_stage"])
        );
    }

    #[test]
    fn prepared_catalog_binding_rejects_changed_evidence_before_copy_history() {
        let fixture = Fixture::with_nested(true).unwrap();
        let journal = NamespaceJournal::bind(&fixture, uuid::Uuid::new_v4().to_string()).unwrap();
        let (_, output) = fixture.scope().unwrap();
        let prepared = preparation::prepare_named(
            output.leaf(),
            &journal.scope.targets,
            &journal,
            &AtomicBool::new(false),
            &journal.scope.token,
        )
        .unwrap();
        let report = serde_json::to_value(&prepared.report).unwrap();
        let reference = Reference {
            token: journal.scope.token.clone(),
            scope_digest: blake3::hash(journal.encoded.as_bytes())
                .to_hex()
                .to_string(),
            fingerprint: report["fingerprint"].as_str().unwrap().into(),
        };
        let owner = fixture.owner.clone();
        let supplied = reference.clone();
        let plan = fixture
            .library
            .catalog
            .call(move |db| {
                let (scope, completion) = load_preparation(db, &owner, &supplied)?;
                preparation::project_copy_plan(
                    &completion,
                    &scope.sources,
                    &format!(".stillharbour-preparing-{}.partial", supplied.token),
                    &scope.targets,
                    &uuid::Uuid::new_v4().to_string(),
                )
            })
            .unwrap();
        for variant in 0..12 {
            let mut changed = plan.clone();
            let mut supplied = reference.clone();
            match variant {
                0 => changed.files[0].stage = ".stillharbour-foreign.partial".into(),
                1 => changed.files[0].target = "unexpected.JPG".into(),
                2 => changed.files[0].revision = "a".repeat(64),
                3 => changed.destination.file += 1,
                4 => {
                    changed.files[0]
                        .prepared_stage
                        .as_mut()
                        .unwrap()
                        .object
                        .file += 1
                }
                5 => {
                    changed.files[0]
                        .destination_location
                        .as_mut()
                        .unwrap()
                        .parents[0]
                        .file += 1
                }
                6 => changed.files[0].source_location.as_mut().unwrap().parents[0].file += 1,
                7 => supplied.scope_digest = "0".repeat(64),
                8 => supplied.fingerprint = "0".repeat(64),
                9 => supplied.token = uuid::Uuid::new_v4().to_string(),
                10 => changed.files[0].prepared_stage = None,
                _ => changed.files.swap(0, 1),
            }
            changed.digest = changed.digest().unwrap();
            assert!(
                CatalogJournal::bind_with_namespace(
                    fixture.library.catalog.clone(),
                    fixture.owner.clone(),
                    fixture.manifest.clone(),
                    changed,
                    Some(supplied)
                )
                .is_err(),
                "variant {variant}"
            );
            assert!(fixture.preserved().unwrap());
        }
        let owner = fixture.owner.clone();
        let count=fixture.library.catalog.call(move|db| Ok(db.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=? AND phase='copy_execution_plan_v1'",[owner],|row|row.get::<_,i64>(0))?)).unwrap();
        assert_eq!(count, 0);
        let valid = CatalogJournal::bind_with_namespace(
            fixture.library.catalog.clone(),
            fixture.owner.clone(),
            fixture.manifest.clone(),
            plan.clone(),
            Some(reference.clone()),
        )
        .unwrap();
        assert_eq!(valid.history_count().unwrap(), 0);
        fixture
            .library
            .catalog
            .call(|db| {
                db.execute("UPDATE roots SET mode='referenced'", [])?;
                Ok(())
            })
            .unwrap();
        assert!(
            CatalogJournal::bind_with_namespace(
                fixture.library.catalog.clone(),
                fixture.owner.clone(),
                fixture.manifest.clone(),
                plan,
                Some(reference)
            )
            .is_err()
        );
        assert!(fixture.preserved().unwrap());
    }

    #[test]
    fn namespace_journal_denial_and_revoked_root_create_nothing() {
        for revoke in [false, true] {
            let fixture = Fixture::new().unwrap();
            let token = uuid::Uuid::new_v4().to_string();
            let journal = NamespaceJournal::bind(&fixture, token.clone()).unwrap();
            fixture.library.catalog.call(move |db| {
                if revoke { db.execute("UPDATE roots SET mode='referenced'",[])?; }
                else { db.execute_batch("CREATE TRIGGER reject_namespace_fact BEFORE INSERT ON operation_journal WHEN NEW.phase='namespace_preparation_fact_v2' BEGIN SELECT RAISE(ABORT,'fixture journal denial'); END;")?; }
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
            db.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,'namespace_preparation_fact_v2',?3,'fixture')",params![owner,token,serde_json::to_string(&Fact{ordinal:2,previous:"foreign".into(),item:None,phase:"namespace_created".into(),evidence:serde_json::json!({})}).unwrap()])?;
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
