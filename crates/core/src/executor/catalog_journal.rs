//! Catalog-actor adapter for the central engine. Currently constructor-owned
//! fixture capability only; retained records are never execution approvals.
#[cfg(all(feature = "fixture-probes", feature = "copy-workflow"))]
pub(in crate::executor) mod cross_volume_fixture;
pub(in crate::executor) mod namespace_journal;
use super::{CopyJournal, CopyPlan, EventJournal, Object, failed};
use crate::{
    catalog::Catalog,
    domain::Result,
    filesystem::FileIdentity,
    library::{copy_manifest, operation_store},
    organization::portable_filename,
};
#[cfg(any(test, feature = "fixture-probes"))]
use crate::{domain::ErrorCode, filesystem};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

const PLAN_PHASE: &str = "copy_execution_plan_v1";
const EVENT_PHASE: &str = "copy_execution_event_v1";
const MAX_RECORD: usize = 65_536;
const MAX_EVENT: usize = 8192;
/// Decode a new catalog-backed stream using the same strict state machine.
/// Callers bind the immutable plan/job before supplying contiguous copy events;
/// this routine never issues or reconstructs an approval.
#[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
pub(super) fn decode_copy_events(
    plan: &CopyPlan,
    events: Vec<(u64, Option<usize>, String, serde_json::Value)>,
) -> Result<(Vec<super::resume::Cursor>, bool, String)> {
    let bound = plan
        .files
        .len()
        .checked_mul(6)
        .and_then(|n| n.checked_add(3))
        .ok_or_else(|| failed("The copy stream exceeds its event bound."))?;
    if events.len() > bound || plan.digest()? != plan.digest {
        return Err(failed("The copy stream differs from its immutable plan."));
    }
    let mut history = History::new(plan);
    let mut hash = blake3::Hasher::new();
    hash.update(plan.digest.as_bytes());
    for (ordinal, item, phase, evidence) in events {
        if phase.len() > 128
            || serde_json::to_vec(&evidence)
                .map_err(|error| failed("Invalid copy event evidence.").caused_by(error))?
                .len()
                > MAX_EVENT
        {
            return Err(failed("A copy event exceeds its evidence bound."));
        }
        let event = Event {
            ordinal,
            stream_digest: plan.digest.clone(),
            item,
            phase,
            evidence,
        };
        history.apply(&event, plan)?;
        let encoded = serde_json::to_vec(&event)
            .map_err(|error| failed("Could not fingerprint copy events.").caused_by(error))?;
        hash.update(&(encoded.len() as u64).to_le_bytes());
        hash.update(&encoded);
    }
    Ok((
        history
            .items
            .iter()
            .map(|item| super::resume::Cursor {
                phase: item.phase,
                object: item.object.clone(),
                digest: item.digest.clone(),
            })
            .collect(),
        history.terminal,
        hash.finalize().to_hex().to_string(),
    ))
}
/// Reuse the production event decoder for the constructor-owned append lab.
#[cfg(any(test, feature = "fixture-probes"))]
pub(super) fn append_fixture_history(
    journal: &super::Journal,
    plan: &CopyPlan,
) -> Result<(Vec<super::resume::Cursor>, bool, String)> {
    let mut history = History::new(plan);
    let mut hash = blake3::Hasher::new();
    hash.update(plan.digest.as_bytes());
    let mut query = journal.0.prepare("SELECT sequence,item,phase,CASE WHEN length(CAST(evidence AS BLOB))<=8192 THEN evidence END FROM events ORDER BY sequence LIMIT ?")?;
    let mut rows = query.query([plan.files.len() as i64 * 6 + 4])?;
    while let Some(row) = rows.next()? {
        let text: Option<String> = row.get(3)?;
        let text = text.ok_or_else(|| failed("Append event exceeds its bound."))?;
        let event = Event {
            ordinal: u64::try_from(row.get::<_, i64>(0)?)
                .map_err(|_| failed("Invalid append journal ordinal."))?,
            stream_digest: plan.digest.clone(),
            item: row
                .get::<_, Option<i64>>(1)?
                .map(usize::try_from)
                .transpose()
                .map_err(|_| failed("Invalid append journal item."))?,
            phase: row.get(2)?,
            evidence: decode(&text, MAX_EVENT)?,
        };
        history.apply(&event, plan)?;
        let encoded = serde_json::to_vec(&event)
            .map_err(|e| failed("Could not fingerprint append events.").caused_by(e))?;
        hash.update(&(encoded.len() as u64).to_le_bytes());
        hash.update(&encoded);
    }
    Ok((
        history
            .items
            .iter()
            .map(|item| super::resume::Cursor {
                phase: item.phase,
                object: item.object.clone(),
                digest: item.digest.clone(),
            })
            .collect(),
        history.terminal,
        hash.finalize().to_hex().to_string(),
    ))
}
#[cfg(test)]
#[path = "catalog_journal/import_bundle_tests.rs"]
mod import_bundle_tests;
#[cfg(test)]
#[path = "catalog_journal/import_partition_tests.rs"]
mod import_partition_tests;
#[cfg(test)]
#[path = "catalog_journal/managed_destination_tests.rs"]
mod managed_destination_tests;
#[cfg(test)]
#[path = "catalog_journal/root_partition_tests.rs"]
mod root_partition_tests;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    policy: String,
    catalog_id: String,
    owner_id: String,
    owner_digest: String,
    manifest_id: String,
    manifest_digest: String,
    plan: CopyPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    namespace: Option<namespace_journal::Reference>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    ordinal: u64,
    stream_digest: String,
    item: Option<usize>,
    phase: String,
    evidence: serde_json::Value,
}
#[derive(Clone, Default)]
struct Item {
    phase: u8,
    object: Option<Object>,
    digest: Option<String>,
    copy_revision: Option<String>,
}
#[derive(Clone)]
struct History {
    count: u64,
    items: Vec<Item>,
    terminal: bool,
}
fn hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn decode<T: serde::de::DeserializeOwned>(text: &str, max: usize) -> Result<T> {
    if text.len() > max {
        return Err(failed("Retained copy journal evidence exceeds its bound."));
    }
    serde_json::from_str(text)
        .map_err(|error| failed("Invalid retained copy journal evidence.").caused_by(error))
}
impl History {
    fn new(plan: &CopyPlan) -> Self {
        Self {
            count: 0,
            items: vec![Item::default(); plan.files.len()],
            terminal: false,
        }
    }
    fn apply(&mut self, event: &Event, plan: &CopyPlan) -> Result<()> {
        if self.terminal
            || event.ordinal != self.count + 1
            || event.stream_digest != plan.digest
            || event.ordinal > plan.files.len() as u64 * 6 + 3
        {
            return Err(failed("Invalid copy journal order or stream binding."));
        }
        if let Some(index) = event.item {
            let file = plan
                .files
                .get(index)
                .ok_or_else(|| failed("Unknown journal file index."))?;
            let item = &mut self.items[index];
            let expected = match event.phase.as_str() {
                "item_intent" => 0,
                "stage_created" => 1,
                "stage_flushed" => 2,
                "stage_verified" => 3,
                "publish_intent" => 4,
                "item_completed_verified" => 5,
                _ => return Err(failed("Unknown file journal phase.")),
            };
            if item.phase != expected {
                return Err(failed(
                    "Copy journal skipped or repeated a required file phase.",
                ));
            }
            match event.phase.as_str() {
                "item_intent" => {
                    if event.evidence
                        != serde_json::json!({"stage":file.stage,"target":file.target,"sourceRevision":file.revision})
                    {
                        return Err(failed("Copy intent differs from its immutable plan."));
                    }
                }
                "stage_created" => {
                    let object: Object = serde_json::from_value(event.evidence.clone())
                        .map_err(|error| failed("Invalid stage identity.").caused_by(error))?;
                    if object.volume != plan.destination.volume
                        || serde_json::to_value(&object).ok() != Some(event.evidence.clone())
                    {
                        return Err(failed("Unexpected stage volume or identity evidence."));
                    }
                    item.object = Some(object);
                }
                "stage_flushed" => {
                    if event.evidence != serde_json::json!({"bytes":file.bytes.to_string()}) {
                        return Err(failed("Flushed bytes differ from the reviewed size."));
                    }
                }
                "stage_verified" => {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields, rename_all = "camelCase")]
                    struct Verified {
                        bytes: String,
                        digest: String,
                        copy_revision: String,
                    }
                    let verified: Verified = serde_json::from_value(event.evidence.clone())
                        .map_err(|error| {
                            failed("Invalid verification evidence.").caused_by(error)
                        })?;
                    if verified.bytes != file.bytes.to_string()
                        || !hex(&verified.digest)
                        || !hex(&verified.copy_revision)
                    {
                        return Err(failed("Invalid byte verification scope."));
                    }
                    item.digest = Some(verified.digest);
                }
                "publish_intent" => {
                    let digest = item
                        .digest
                        .as_ref()
                        .ok_or_else(|| failed("Missing staged verification."))?;
                    if event.evidence
                        != serde_json::json!({"stage":file.stage,"target":file.target,"digest":digest})
                    {
                        return Err(failed("Publication differs from verified staged content."));
                    }
                }
                "item_completed_verified" => {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields, rename_all = "camelCase")]
                    struct Completed {
                        digest: String,
                        copy_revision: String,
                        object: Object,
                    }
                    let completed: Completed = serde_json::from_value(event.evidence.clone())
                        .map_err(|error| {
                            failed("Invalid completed verification.").caused_by(error)
                        })?;
                    if item.digest.as_ref() != Some(&completed.digest)
                        || item.object.as_ref() != Some(&completed.object)
                        || !hex(&completed.copy_revision)
                    {
                        return Err(failed(
                            "Completed output differs from the verified owned object.",
                        ));
                    }
                    item.copy_revision = Some(completed.copy_revision);
                }
                _ => unreachable!(),
            }
            item.phase += 1;
        } else {
            match event.phase.as_str() {
                "plan_completed_verified"
                    if self.items.iter().all(|item| item.phase == 6)
                        && event.evidence == serde_json::json!({"files":plan.files.len()}) => {}
                "canceled_with_partial_copies" | "needs_attention" => {}
                _ => return Err(failed("Invalid plan completion or terminal journal phase.")),
            }
            self.terminal = true;
        }
        self.count = event.ordinal;
        Ok(())
    }
}

pub(super) struct CatalogJournal {
    catalog: Arc<Catalog>,
    binding: Binding,
    encoded: String,
    anchor: i64,
    history: Mutex<History>,
    source_mode: crate::library::model::RootMode,
}
struct ObservedJournal<'a> {
    inner: &'a CatalogJournal,
    progress: Option<&'a dyn Fn(u32) -> Result<()>>,
}
impl EventJournal for ObservedJournal<'_> {
    fn append(&self, item: Option<usize>, phase: &str, evidence: serde_json::Value) -> Result<()> {
        self.inner.append(item, phase, evidence)?;
        if phase == "item_completed_verified"
            && let Some(progress) = self.progress
        {
            progress(self.inner.verified_files()?)?;
        }
        Ok(())
    }
}
impl CopyJournal for ObservedJournal<'_> {
    fn history_count(&self) -> Result<u64> {
        self.inner.history_count()
    }
    fn stage_identity(&self, item: usize) -> Result<Option<FileIdentity>> {
        self.inner.stage_identity(item)
    }
}
fn validate_binding(db: &Connection, binding: &Binding) -> Result<()> {
    if binding
        .plan
        .files
        .iter()
        .any(|file| file.stage_location.is_some())
    {
        return Err(failed(
            "Organization copy streams do not authorize separate staging locations.",
        ));
    }
    if binding.namespace.is_some() {
        return namespace_journal::validate_copy_binding(db, binding);
    }
    crate::operations::validate_id(&binding.owner_id)?;
    crate::operations::validate_id(&binding.manifest_id)?;
    crate::operations::validate_id(&binding.plan.id)?;
    let draft = operation_store::load(db, &binding.owner_id)?;
    let manifest = copy_manifest::load(db, &binding.owner_id, &binding.manifest_id)?;
    let plan = &binding.plan;
    if binding.policy != "catalog-copy-stream-v1"
        || binding.catalog_id != draft.catalog_id
        || binding.owner_digest != draft.view.digest
        || binding.manifest_digest != manifest.view.digest
        || plan.digest()? != plan.digest
        || !manifest.view.issues.is_empty()
        || manifest.files.len() != plan.files.len()
        || plan.files.is_empty()
        || plan.destination.volume != draft.destination_device
        || plan.destination.file != draft.destination_file
        || plan.destination_volume != draft.destination_volume
    {
        return Err(failed(
            "Copy stream no longer matches its retained draft/manifest.",
        ));
    }
    let root_id = &manifest.files[0].view.root_id;
    let (device, inode, volume, mode): (String, String, String, String) = db.query_row(
        "SELECT device,inode,volume,mode FROM roots WHERE id=?",
        [root_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    if device != plan.source.volume.to_string()
        || inode != plan.source.file.to_string()
        || volume != plan.source_volume
        || mode != "managed"
    {
        return Err(failed(
            "The copy source root requires its unchanged managed write grant.",
        ));
    }
    for (index, (expected, file)) in manifest.files.iter().zip(&plan.files).enumerate() {
        // A prepared stage must enter through the separately bound namespace
        // projection, never through this direct-destination compatibility path.
        if file.prepared_stage.is_some() {
            return Err(failed(
                "Prepared stages require a matching retained namespace reference.",
            ));
        }
        let native_source = if let Some(location) = &file.source_location {
            location.validate()?;
            location.relative_native.clone()
        } else {
            crate::library::native::encode_path(Path::new(&file.source))
        };
        let target = if let Some(location) = &file.destination_location {
            location.validate()?;
            format!("{}/{}", location.relative_parent, file.target)
        } else {
            file.target.clone()
        };
        if &expected.view.root_id != root_id
            || (file.source_location.is_none() && !portable_filename(&file.source))
            || !portable_filename(&file.target)
            || expected.relative_native != native_source
            || expected.view.target_relative_path != target
            || expected.view.bytes != file.bytes.to_string()
            || expected.view.source_revision != file.revision
            || expected.identity != (file.object.volume, file.object.file)
            || file.stage != format!(".stillharbour-{}-{index}.partial", plan.id)
        {
            return Err(failed(
                "This copy stream requires exact single-root native source and destination mappings.",
            ));
        }
    }
    Ok(())
}
fn binding_source_root(db: &Connection, binding: &Binding) -> Result<String> {
    if binding.namespace.is_some() {
        namespace_journal::source_root_id(db, binding)
    } else {
        validate_binding(db, binding)?;
        copy_manifest::load(db, &binding.owner_id, &binding.manifest_id)?
            .files
            .first()
            .map(|file| file.view.root_id.clone())
            .ok_or_else(|| failed("The retained copy manifest has no source root."))
    }
}
impl CatalogJournal {
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(in crate::executor) fn validate_completed_members(
        &self,
        output: &std::fs::File,
    ) -> Result<()> {
        let history = self
            .history
            .lock()
            .map_err(|_| failed("Completed copy history is unavailable."))?;
        if !self.binding.plan.destination.matches(output)?
            || history
                .items
                .iter()
                .any(|item| item.phase != 6 || item.copy_revision.is_none())
        {
            return Err(failed(
                "Only the exact completed published output can be indexed.",
            ));
        }
        for (file, item) in self.binding.plan.files.iter().zip(&history.items) {
            let parent = super::destination_parent(output, file)?;
            let current = super::native::read(&parent, &file.target)?;
            let object = item
                .object
                .as_ref()
                .ok_or_else(|| failed("Published output has no recorded native identity."))?;
            if !object.matches(&current)?
                || crate::library::native::revision(&current)?
                    != *item
                        .copy_revision
                        .as_ref()
                        .ok_or_else(|| failed("Published output has no recorded version."))?
                || current.metadata()?.len() != file.bytes
                || crate::filesystem::identity(&current)?.links != 1
            {
                return Err(failed(
                    "Published output changed since complete byte verification. Indexing was not authorized by this receipt.",
                ));
            }
        }
        Ok(())
    }
    pub(in crate::executor) fn recovery_inventory(
        &self,
        parent: &std::fs::File,
    ) -> Result<super::preparation::RecoveryInventory> {
        let binding = self.binding.clone();
        let (namespace, targets, completion) = self
            .catalog
            .call(move |db| namespace_journal::recovery_inventory_record(db, &binding))?;
        super::preparation::recovery_inventory(parent, &completion, &namespace, &targets)
    }
    #[cfg(any(test, feature = "fixture-probes", feature = "copy-workflow"))]
    pub(in crate::executor) fn recovery_history(
        &self,
    ) -> Result<(Vec<super::resume::Cursor>, String, bool)> {
        let binding = self.binding.clone();
        let encoded = self.encoded.clone();
        let anchor = self.anchor;
        let memory = self
            .history
            .lock()
            .map_err(|_| failed("Copy history is unavailable."))?;
        self.catalog.call(move|db| {
            validate_binding(db,&binding)?;
            let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operation_journal WHERE sequence=?1 AND plan_id=?2 AND item_id=?3 AND phase=?4 AND evidence=?5) AND (SELECT count(*) FROM operation_journal WHERE plan_id=?2 AND item_id=?3 AND phase=?4)=1",params![anchor,binding.owner_id,binding.plan.id,PLAN_PHASE,encoded],|row|row.get(0))?;
            if !valid {return Err(failed("Recovery copy anchor changed."));}
            let mut history=History::new(&binding.plan);
            let mut hash=blake3::Hasher::new();
            hash.update(encoded.as_bytes());
            let mut query=db.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=8192 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT ?4")?;
            let mut rows=query.query(params![binding.owner_id,binding.plan.id,EVENT_PHASE,binding.plan.files.len() as i64*6+4])?;
            while let Some(row)=rows.next()? {
                let text:Option<String>=row.get(0)?;
                let text=text.ok_or_else(||failed("Recovery event exceeds 8 KiB."))?;
                history.apply(&decode(&text,MAX_EVENT)?,&binding.plan)?;
                hash.update(&(text.len() as u64).to_le_bytes());hash.update(text.as_bytes());
            }
            if history.terminal && !history.items.iter().all(|item|item.phase==6) {return Err(failed("A terminal incomplete stream requires separate reconciliation."));}
            Ok((history,hash.finalize().to_hex().to_string()))
        }).and_then(|(history,digest)| {
            if history.count!=memory.count {return Err(failed("Another writer changed recovery history. Reopen it."));}
            Ok((history.items.into_iter().map(|item|super::resume::Cursor{phase:item.phase,object:item.object,digest:item.digest}).collect(),digest,history.terminal))
        })
    }
    #[cfg(any(test, feature = "fixture-probes", feature = "copy-workflow"))]
    pub(in crate::executor) fn recovery_review(
        &self,
        inputs: &super::CopyInputs<'_>,
    ) -> Result<super::resume::Review> {
        let (cursors, digest, _) = self.recovery_history()?;
        super::resume::Review::capture(&self.binding.plan, &cursors, digest, inputs)
    }
    #[cfg(any(test, feature = "fixture-probes", feature = "copy-workflow"))]
    pub(in crate::executor) fn resume_approved(
        &self,
        inputs: super::CopyInputs<'_>,
        selection: &str,
        review: super::resume::Review,
        proof: super::approval::Proof,
        fault: Option<super::CopyFault>,
        progress: Option<&dyn Fn(u32) -> Result<()>>,
    ) -> Result<super::CopyOutcome> {
        proof.authorize(&review.approval(&self.binding.plan, selection)?)?;
        let (cursors, digest, terminal) = self.recovery_history()?;
        let binding = self.binding.clone();
        let root = self
            .catalog
            .call(move |db| binding_source_root(db, &binding))?;
        let validate = || {
            (inputs.validate_scope)()?;
            self.validate_root_grant(&root)
        };
        let scoped_inputs = super::CopyInputs {
            source_root: inputs.source_root,
            destination: inputs.destination,
            io: inputs.io,
            cancel: inputs.cancel,
            validate_scope: &validate,
        };
        super::resume::Continuation {
            plan: &self.binding.plan,
            journal: self,
            cursors: &cursors,
            terminal,
            review: &review,
            history_digest: digest,
            inputs: &scoped_inputs,
            fault,
            progress,
        }
        .execute()
    }
    fn validate_root_grant(&self, root: &str) -> Result<()> {
        let root = root.to_owned();
        let object = self.binding.plan.source.clone();
        let expected_volume = self.binding.plan.source_volume.clone();
        let expected_mode = match self.source_mode {
            crate::library::model::RootMode::Managed => "managed",
            crate::library::model::RootMode::Referenced => "referenced",
        };
        self.catalog.call(move|db| {
            let (device,inode,volume,mode):(String,String,String,String)=db.query_row("SELECT device,inode,volume,mode FROM roots WHERE id=?",[root],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))?;
            if device!=object.volume.to_string() || inode!=object.file.to_string() || volume!=expected_volume || mode!=expected_mode {
                return Err(failed(&format!("The source root {expected_mode} grant changed. Retain current outputs and review again.")));
            }
            Ok(())
        })
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(in crate::executor) fn matches_job(
        &self,
        job: &crate::operations::CopyWorkflowJob,
        digest: &str,
    ) -> Result<bool> {
        let scope = self
            .binding
            .namespace
            .as_ref()
            .ok_or_else(|| failed("Recovery needs a bound namespace."))?;
        if job.plan_id != self.binding.owner_id
            || job.total_files as usize != self.binding.plan.files.len()
        {
            return Ok(false);
        }
        match job.action {
            crate::operations::CopyAction::CopyFiles => {
                if job.review_id != self.binding.plan.id {
                    return Ok(false);
                }
                if digest == self.digest() {
                    return Ok(true);
                }
                let record = crate::library::operation_jobs::continuation_record(
                    &self.catalog,
                    job.plan_id.clone(),
                    job.id.clone(),
                )?;
                if record.stream_id != self.binding.plan.id
                    || record.binding_digest != self.digest()
                {
                    return Ok(false);
                }
                super::resume::validate_record(record.review, &self.binding.plan, digest)?;
                Ok(true)
            }
            crate::operations::CopyAction::PrepareNames => Ok(job.review_id == scope.token
                && digest == scope.scope_digest
                && job.ready_review_id.as_deref() == Some(self.binding.plan.id.as_str())),
        }
    }
    /// Open existing facts only. Unlike bind, this cannot create an anchor or
    /// a new stream. Readable registered roots need no managed write grant.
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(in crate::executor) fn open_inspection(
        catalog: Arc<Catalog>,
        owner: String,
        stream: String,
    ) -> Result<(Self, super::sources::Snapshot, String)> {
        crate::operations::validate_id(&owner)?;
        crate::operations::validate_id(&stream)?;
        let (binding,encoded,anchor,history,sources,namespace)=catalog.call(move|db| {
            let tx=db.transaction()?;
            let (anchor,encoded,binding)={
                let mut query=tx.prepare("SELECT sequence,CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 ORDER BY sequence LIMIT 2")?;
                let mut rows=query.query(params![owner,stream,PLAN_PHASE])?;
                let row=rows.next()?.ok_or_else(||failed("No retained copy stream exists for this job."))?;
                let anchor:i64=row.get(0)?;
                let encoded:Option<String>=row.get(1)?;
                let encoded=encoded.ok_or_else(||failed("Retained copy anchor exceeds 64 KiB."))?;
                if rows.next()?.is_some() {return Err(failed("Duplicate retained copy anchors require review."));}
                let binding:Binding=decode(&encoded,MAX_RECORD)?;
                if binding.owner_id!=owner || binding.plan.id!=stream {return Err(failed("Retained copy anchor belongs to another job."));}
                (anchor,encoded,binding)
            };
            let (sources,namespace)=namespace_journal::inspection_scope(&tx,&binding)?;
            namespace_journal::check_copy_stream_admission(&tx,&binding)?;
            let mut history=History::new(&binding.plan);
            {
                let mut query=tx.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=8192 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND item_id=?3 ORDER BY sequence LIMIT ?4")?;
                let mut rows=query.query(params![owner,EVENT_PHASE,stream,binding.plan.files.len() as i64*6+4])?;
                while let Some(row)=rows.next()? {
                    let text:Option<String>=row.get(0)?;
                    let text=text.ok_or_else(||failed("Retained copy event exceeds 8 KiB."))?;
                    history.apply(&decode(&text,MAX_EVENT)?,&binding.plan)?;
                }
            }
            tx.commit()?;
            Ok((binding,encoded,anchor,history,sources,namespace))
        })?;
        Ok((
            Self {
                source_mode: sources.required_mode(),
                catalog,
                binding,
                encoded,
                anchor,
                history: Mutex::new(history),
            },
            sources,
            namespace,
        ))
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(in crate::executor) fn inspect(
        &self,
        inputs: super::CopyInputs<'_>,
    ) -> Result<Vec<super::RecoveredItem>> {
        (inputs.validate_scope)()?;
        let states = self
            .binding
            .plan
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                super::inspect_item(
                    file,
                    index,
                    self,
                    inputs.source_root,
                    inputs.destination,
                    inputs.io,
                    inputs.cancel,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        (inputs.validate_scope)()?;
        Ok(states)
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(in crate::executor) fn plan(&self) -> &CopyPlan {
        &self.binding.plan
    }
    pub(in crate::executor) fn digest(&self) -> String {
        blake3::hash(self.encoded.as_bytes()).to_hex().to_string()
    }
    pub(in crate::executor) fn verified_files(&self) -> Result<u32> {
        Ok(self
            .history
            .lock()
            .map_err(|_| failed("Copy history is unavailable."))?
            .items
            .iter()
            .filter(|item| item.phase == 6)
            .count() as u32)
    }
    pub(in crate::executor) fn copy_review(
        &self,
        selection: &str,
    ) -> Result<super::approval::Review> {
        super::approval::Review::new(
            super::approval::Action::CopyFiles,
            self.binding.plan.id.clone(),
            self.digest(),
            selection.into(),
        )
    }
    pub(in crate::executor) fn execute_approved(
        &self,
        inputs: super::CopyInputs<'_>,
        selection: &str,
        proof: super::approval::Proof,
        fault: Option<super::CopyFault>,
        progress: Option<&dyn Fn(u32) -> Result<()>>,
    ) -> Result<super::CopyOutcome> {
        proof.authorize(&self.copy_review(selection)?)?;
        let binding = self.binding.clone();
        let root = self
            .catalog
            .call(move |db| binding_source_root(db, &binding))?;
        let validate = || {
            (inputs.validate_scope)()?;
            self.validate_root_grant(&root)
        };
        let journal = ObservedJournal {
            inner: self,
            progress,
        };
        super::CopyContext {
            plan: &self.binding.plan,
            journal: &journal,
            source_root: inputs.source_root,
            destination: inputs.destination,
            io: inputs.io,
            cancel: inputs.cancel,
            validate_scope: &validate,
        }
        .execute(fault)
    }
    #[cfg(any(test, feature = "fixture-probes"))]
    pub(super) fn bind(
        catalog: Arc<Catalog>,
        owner_id: String,
        manifest_id: String,
        plan: CopyPlan,
    ) -> Result<Self> {
        Self::bind_with_namespace(catalog, owner_id, manifest_id, plan, None)
    }
    fn bind_with_namespace(
        catalog: Arc<Catalog>,
        owner_id: String,
        manifest_id: String,
        plan: CopyPlan,
        namespace: Option<namespace_journal::Reference>,
    ) -> Result<Self> {
        let supplied = plan.clone();
        let owner = owner_id.clone();
        let manifest = manifest_id.clone();
        let (binding,encoded,anchor,history,source_mode)=catalog.call(move |db| {
            let tx=db.transaction()?;
            let draft=operation_store::load(&tx,&owner)?;
            let scope=copy_manifest::load(&tx,&owner,&manifest)?;
            let binding=Binding{policy:"catalog-copy-stream-v1".into(),catalog_id:draft.catalog_id,owner_id:owner,owner_digest:draft.view.digest,manifest_id:manifest,manifest_digest:scope.view.digest,plan:supplied,namespace};
            validate_binding(&tx,&binding)?;
            namespace_journal::check_copy_stream_admission(&tx,&binding)?;
            let encoded=serde_json::to_string(&binding).map_err(|error|failed("Could not retain copy stream.").caused_by(error))?;
            if encoded.len()>MAX_RECORD {return Err(failed("Copy stream exceeds its 64-KiB retained record bound."));}
            let previous={
                let mut query=tx.prepare("SELECT sequence,CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND item_id=?3 ORDER BY sequence LIMIT 2")?;
                let mut rows=query.query(params![binding.owner_id,PLAN_PHASE,binding.plan.id])?;
                let previous=if let Some(row)=rows.next()? {Some((row.get::<_,i64>(0)?,row.get::<_,Option<String>>(1)?))} else {None};
                if rows.next()?.is_some() {return Err(failed("Multiple immutable copy-stream anchors require review."));}
                previous
            };
            let anchor=if let Some((sequence,previous))=previous {
                if previous.as_ref()!=Some(&encoded) {return Err(failed("The immutable copy stream changed or exceeds its bound."));} sequence
            } else {
                tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![binding.owner_id,binding.plan.id,PLAN_PHASE,encoded,crate::library::store::now()])?;
                tx.last_insert_rowid()
            };
            let mut history=History::new(&binding.plan);
            {
                let mut query=tx.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=8192 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND item_id=?3 ORDER BY sequence LIMIT ?4")?;
                let mut rows=query.query(params![binding.owner_id,EVENT_PHASE,binding.plan.id,binding.plan.files.len() as i64*6+4])?;
                while let Some(row)=rows.next()? {
                    let text:Option<String>=row.get(0)?;
                    let text=text.ok_or_else(||failed("Retained copy event exceeds 8 KiB."))?;
                    let event:Event=decode(&text,MAX_EVENT)?;
                    history.apply(&event,&binding.plan)?;
                }
            }
            let source_mode=if binding.namespace.is_some(){namespace_journal::required_source_mode(&tx,&binding)?}else{crate::library::model::RootMode::Managed};
            tx.commit()?;Ok((binding,encoded,anchor,history,source_mode))
        })?;
        Ok(Self {
            source_mode,
            catalog,
            binding,
            encoded,
            anchor,
            history: Mutex::new(history),
        })
    }
}
impl EventJournal for CatalogJournal {
    fn append(&self, item: Option<usize>, phase: &str, evidence: serde_json::Value) -> Result<()> {
        let mut history = self
            .history
            .lock()
            .map_err(|_| failed("The copy journal state is unavailable."))?;
        let event = Event {
            ordinal: history.count + 1,
            stream_digest: self.binding.plan.digest.clone(),
            item,
            phase: phase.into(),
            evidence,
        };
        let mut proposed = history.clone();
        proposed.apply(&event, &self.binding.plan)?;
        let text = serde_json::to_string(&event)
            .map_err(|error| failed("Could not encode copy event.").caused_by(error))?;
        if text.len() > MAX_EVENT {
            return Err(failed("Copy event exceeds its 8-KiB bound."));
        }
        let anchor = self.anchor;
        let encoded = self.encoded.clone();
        let owner = self.binding.owner_id.clone();
        let stream = self.binding.plan.id.clone();
        let count = history.count;
        self.catalog.call(move |db| {
            let tx=db.transaction()?;
            let unchanged:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_journal WHERE sequence=?1 AND plan_id=?2 AND phase=?3 AND item_id=?4 AND evidence=?5)",params![anchor,owner,PLAN_PHASE,stream,encoded],|row|row.get(0))?;
            if !unchanged {return Err(failed("Copy stream anchor changed before journaling."));}
            let existing:i64=tx.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND item_id=?3",params![owner,EVENT_PHASE,stream],|row|row.get(0))?;
            if u64::try_from(existing).ok()!=Some(count) {return Err(failed("Another writer changed this copy stream. Reopen recovery inspection."));}
            tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![owner,stream,EVENT_PHASE,text,crate::library::store::now()])?;
            tx.commit()?;Ok(())
        })?;
        *history = proposed;
        Ok(())
    }
}
impl CopyJournal for CatalogJournal {
    fn history_count(&self) -> Result<u64> {
        Ok(self
            .history
            .lock()
            .map_err(|_| failed("Copy history is unavailable."))?
            .count)
    }
    fn stage_identity(&self, item: usize) -> Result<Option<FileIdentity>> {
        let history = self
            .history
            .lock()
            .map_err(|_| failed("Copy history is unavailable."))?;
        let item = history
            .items
            .get(item)
            .ok_or_else(|| failed("Unknown copy history item."))?;
        Ok(item.object.as_ref().map(|object| FileIdentity {
            volume: object.volume,
            file: object.file,
            links: 1,
        }))
    }
}

#[cfg(any(test, feature = "fixture-probes"))]
fn generated_photo_bytes() -> Result<Vec<u8>> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(16, 8, image::Rgb([19, 43, 67]))
        .write_to(&mut bytes, image::ImageFormat::Jpeg)
        .map_err(|error| failed("Could not generate owned journal media.").caused_by(error))?;
    Ok(bytes.into_inner())
}
#[cfg(any(test, feature = "fixture-probes"))]
const COMPANION_BYTES: &[u8] = b"<x:xmpmeta>generated owned companion</x:xmpmeta>";
#[cfg(any(test, feature = "fixture-probes"))]
const NEIGHBOR_BYTES: &[u8] = b"retained unrelated destination";
#[cfg(any(test, feature = "fixture-probes"))]
struct Fixture {
    library: Arc<crate::library::Library>,
    _run: tempfile::TempDir,
    source: std::path::PathBuf,
    destination: std::path::PathBuf,
    owner: String,
    manifest: String,
    plan: CopyPlan,
    baseline: Vec<(String, Vec<u8>)>,
}
#[cfg(any(test, feature = "fixture-probes"))]
impl Fixture {
    #[cfg(test)]
    fn new() -> Result<Self> {
        Self::with_nested(false)
    }
    fn with_nested(nested: bool) -> Result<Self> {
        Self::with_media(nested, generated_photo_bytes()?)
    }
    fn with_media(nested: bool, photo: Vec<u8>) -> Result<Self> {
        Self::with_media_at(nested, photo, None)
    }
    fn with_media_at(
        nested: bool,
        photo: Vec<u8>,
        destination_parent: Option<&Path>,
    ) -> Result<Self> {
        Self::with_media_destination(nested, photo, destination_parent, None)
    }
    fn with_media_destination(
        nested: bool,
        photo: Vec<u8>,
        destination_parent: Option<&Path>,
        destination_mode: Option<crate::library::model::RootMode>,
    ) -> Result<Self> {
        use crate::library::{
            Library, LibraryOptions, WorkerPaths,
            model::{RootMode, ScanFilter},
            native, store,
        };
        use crate::operations::{
            BundleConfirmationInput, BundleRelation, CopyManifestInput, PlanDraftInput,
        };
        use crate::organization::{MissingDatePolicy, OrganizationTemplate};
        let run = tempfile::Builder::new()
            .prefix("stillharbour-catalog-copy-")
            .tempdir()?;
        let base = run.path().canonicalize()?;
        let source = base.join("source");
        let data = base.join("data");
        let destination = match destination_parent {
            Some(parent) => parent.join(
                base.file_name()
                    .ok_or_else(|| failed("Owned fixture run has no directory name."))?,
            ),
            None => base.join("output"),
        };
        for path in [&source, &data, &destination] {
            std::fs::create_dir(path)?;
        }
        let source_folder = if nested {
            source.join("source-folder")
        } else {
            source.clone()
        };
        if nested {
            std::fs::create_dir(&source_folder)?;
            std::fs::create_dir(destination.join("organized"))?;
        }
        std::fs::write(source_folder.join("Photo.JPG"), photo)?;
        std::fs::write(source_folder.join("Photo.xmp"), COMPANION_BYTES)?;
        std::fs::write(destination.join("destination-only.bin"), NEIGHBOR_BYTES)?;
        let baseline = ["Photo.JPG", "Photo.xmp"]
            .into_iter()
            .map(|name| {
                Ok((
                    if nested {
                        format!("source-folder/{name}")
                    } else {
                        name.to_owned()
                    },
                    std::fs::read(source_folder.join(name))?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
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
        let root = native::open_root(&source, None)?;
        let source_object = Object::of(&root)?;
        let identity = filesystem::identity(&root)?;
        let source_volume = native::volume_token(&root)?;
        let root_id = store::add_root(
            &catalog,
            source.clone(),
            identity.clone(),
            source_volume.clone(),
            "Owned catalog copy fixture".into(),
            RootMode::Managed,
            ScanFilter::default(),
        )?;
        let relative = if nested {
            "source-folder/Photo.JPG"
        } else {
            "Photo.JPG"
        };
        let file = native::open_asset(&source, &identity, &source_volume, Path::new(relative))?;
        let revision = native::revision(&file)?;
        let file_id = filesystem::identity(&file)?;
        let bytes = file.metadata()?.len();
        drop(file);
        let asset = uuid::Uuid::new_v4().to_string();
        let inserted = asset.clone();
        let inserted_revision = revision.clone();
        // Fixture-only catalog inventory, just as the existing generated metadata
        // tests use. This does not claim a scan/decoder/user-flow acceptance.
        catalog.call(move |db| {
            db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,?2,?3,?8,?4,?9,'Photo.JPG','Photo','jpg',?5,?6,?7,'now','2026-10-31T00:00:00','filesystem_modified','ready','pending','present',1,'owned-fixture')",params![inserted,root_id,native::encode_path(Path::new(relative)),native::encode_path(Path::new(if nested {"source-folder"} else {""})),format!("{}:{}",file_id.volume,file_id.file),bytes as i64,inserted_revision,relative,if nested {"source-folder"}else{""}])?;
            db.execute("UPDATE library_state SET revision=revision+1",[])?;Ok(())
        })?;
        let template = OrganizationTemplate {
            pattern: if nested {
                "organized/{original_stem}.{ext}"
            } else {
                "{original_stem}.{ext}"
            }
            .into(),
            event: String::new(),
            missing_date: MissingDatePolicy::Block,
            filename_stem_case: None,
        };
        let target = crate::library::native::open_root(&destination, None)?;
        let target_identity = filesystem::identity(&target)?;
        let destination_volume = native::volume_token(&target)?;
        let destination_root_id = destination_mode
            .map(|mode| {
                store::add_root(
                    &catalog,
                    destination.clone(),
                    target_identity.clone(),
                    destination_volume.clone(),
                    "Explicit generated destination library".into(),
                    mode,
                    ScanFilter::default(),
                )
            })
            .transpose()?;
        let preview = library.organization_preview(vec![asset.clone()], template.clone())?;
        let draft = library.organization_plan_create(
            PlanDraftInput {
                destination_root_id,
                asset_ids: vec![asset.clone()],
                template,
                preview_fingerprint: preview.fingerprint,
                template_reference: None,
            },
            destination.clone(),
            (target_identity, destination_volume.clone()),
        )?;
        let inventory = library.organization_bundle_inventory(asset.clone(), revision.clone())?;
        let confirmation = library.organization_bundle_confirm(BundleConfirmationInput {
            plan_id: draft.plan.id.clone(),
            asset_id: asset,
            source_revision: revision,
            inventory_fingerprint: inventory.fingerprint,
            companion_ids: inventory
                .entries
                .iter()
                .filter(|entry| entry.selectable && entry.relation != BundleRelation::Additional)
                .map(|entry| entry.id.clone())
                .collect(),
            reviewed_all_entries: true,
        })?;
        let manifest = library.organization_copy_manifest(CopyManifestInput {
            plan_id: draft.plan.id.clone(),
            bundle_confirmation_ids: vec![confirmation.id],
        })?;
        let stream = uuid::Uuid::new_v4().to_string();
        let mut files = Vec::new();
        let owner = draft.plan.id.clone();
        let selected = manifest.id.clone();
        let private = catalog.call(move |db| copy_manifest::load(db, &owner, &selected))?;
        for (index, (item, native_file)) in manifest.files.iter().zip(&private.files).enumerate() {
            let source_location = if nested {
                Some(super::locations::Source::capture(
                    &root,
                    &native::decode_path(&native_file.relative_native)?,
                )?)
            } else {
                None
            };
            let file = if let Some(location) = &source_location {
                location.open(&root)?
            } else {
                super::native::read(&root, &item.source_name)?
            };
            files.push(super::CopyFile {
                source: item.source_name.clone(),
                target: if nested {
                    Path::new(&item.target_relative_path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| failed("Invalid fixture target."))?
                        .into()
                } else {
                    item.target_relative_path.clone()
                },
                stage: format!(".stillharbour-{stream}-{index}.partial"),
                object: Object::of(&file)?,
                revision: native::revision(&file)?,
                bytes: file.metadata()?.len(),
                source_location,
                destination_location: if nested {
                    Some(super::locations::Destination::capture(
                        &target,
                        "organized",
                    )?)
                } else {
                    None
                },
                prepared_stage: None,
                stage_location: None,
            });
        }
        let mut plan = CopyPlan {
            id: stream,
            digest: String::new(),
            source: source_object,
            destination: Object::of(&target)?,
            source_volume,
            destination_volume,
            files,
        };
        plan.digest = plan.digest()?;
        Ok(Self {
            library,
            _run: run,
            source,
            destination,
            owner: draft.plan.id,
            manifest: manifest.id,
            plan,
            baseline,
        })
    }
    fn scope(
        &self,
    ) -> Result<(
        crate::filesystem::DirectoryChain,
        crate::filesystem::DirectoryChain,
    )> {
        let source = crate::filesystem::DirectoryChain::open(&self.source)?;
        let output = super::output_chain(&self.destination)?;
        if !self.plan.source.matches(source.leaf())?
            || !self.plan.destination.matches(output.leaf())?
            || crate::library::native::volume_token(source.leaf())? != self.plan.source_volume
            || crate::library::native::volume_token(output.leaf())? != self.plan.destination_volume
        {
            return Err(failed("Owned catalog fixture scope changed."));
        }
        Ok((source, output))
    }
    fn journal(&self) -> Result<CatalogJournal> {
        CatalogJournal::bind(
            self.library.catalog.clone(),
            self.owner.clone(),
            self.manifest.clone(),
            self.plan.clone(),
        )
    }
    fn preserved(&self) -> Result<bool> {
        for (name, bytes) in &self.baseline {
            if std::fs::read(self.source.join(name))? != *bytes {
                return Ok(false);
            }
        }
        Ok(
            std::fs::read(self.destination.join("destination-only.bin"))?
                == b"retained unrelated destination",
        )
    }
    fn execute(
        &self,
        journal: &CatalogJournal,
        fault: Option<super::CopyFault>,
    ) -> Result<super::CopyOutcome> {
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let (source, output) = self.scope()?;
        let lease = Registry::shared().try_acquire(vec![
            Scope::new(source, Access::Read),
            Scope::new(output, Access::Write),
        ])?;
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let validate = || self.scope().map(|_| ());
        let io = crate::jobs::paced_io::PacedIo::shared();
        super::CopyContext {
            plan: &self.plan,
            journal,
            source_root: lease.directory(0).leaf(),
            destination: lease.directory(1).leaf(),
            io: &io,
            cancel: &cancel,
            validate_scope: &validate,
        }
        .execute(fault)
    }
    #[cfg(any(test, feature = "fixture-probes"))]
    fn resume(
        &self,
        journal: &CatalogJournal,
        fault: Option<super::CopyFault>,
    ) -> Result<super::CopyOutcome> {
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let (source, output) = self.scope()?;
        let lease = Registry::shared().try_acquire(vec![
            Scope::new(source, Access::Read),
            Scope::new(output, Access::Write),
        ])?;
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let validate = || {
            let binding = journal.binding.clone();
            self.library
                .catalog
                .call(move |db| validate_binding(db, &binding))?;
            self.scope().map(|_| ())
        };
        let io = crate::jobs::paced_io::PacedIo::shared();
        let inputs = super::CopyInputs {
            source_root: lease.directory(0).leaf(),
            destination: lease.directory(1).leaf(),
            io: &io,
            cancel: &cancel,
            validate_scope: &validate,
        };
        let review = journal.recovery_review(&inputs)?;
        let selection = uuid::Uuid::new_v4().to_string();
        let expected = review.approval(&journal.binding.plan, &selection)?;
        let approvals = super::approval::Approvals::new();
        let ticket = approvals.issue(expected.clone())?;
        let proof = approvals.consume(&ticket, &expected)?;
        journal.resume_approved(inputs, &selection, review, proof, fault, None)
    }
    fn inspect(&self, journal: &CatalogJournal) -> Result<Vec<super::RecoveredItem>> {
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let (source, output) = self.scope()?;
        let lease = Registry::shared().try_acquire(vec![
            Scope::new(source, Access::Read),
            Scope::new(output, Access::Read),
        ])?;
        let io = crate::jobs::paced_io::PacedIo::shared();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        self.plan
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                super::inspect_item(
                    file,
                    index,
                    journal,
                    lease.directory(0).leaf(),
                    lease.directory(1).leaf(),
                    &io,
                    &cancel,
                )
            })
            .collect()
    }
}
#[cfg(any(test, feature = "fixture-probes"))]
fn resume_fixture_probe() -> Result<serde_json::Value> {
    let mut photo = generated_photo_bytes()?;
    photo.resize(crate::jobs::paced_io::CHUNK_BYTES * 3 + 117, 0x5a);
    let fixture = Fixture::with_media(false, photo)?;
    let journal = fixture.journal()?;
    fixture.execute(&journal, Some(super::CopyFault::AfterCopyChunk))?;
    let staged = super::native::read(
        super::output_chain(&fixture.destination)?.leaf(),
        &fixture.plan.files[0].stage,
    )?;
    let original = filesystem::identity(&staged)?;
    let partial_bytes = staged.metadata()?.len();
    drop(staged);
    drop(journal);
    let reopened = fixture.journal()?;
    if fixture.resume(&reopened, Some(super::CopyFault::AfterPublish))?
        != super::CopyOutcome::Interrupted
    {
        return Err(failed(
            "Owned continuation did not interrupt after publication.",
        ));
    }
    let interrupted = reopened.history_count()?;
    drop(reopened);
    let reopened = fixture.journal()?;
    let outcome = fixture.resume(&reopened, None)?;
    let events = reopened.history_count()?;
    let states = fixture.inspect(&reopened)?;
    let published = super::native::read(
        super::output_chain(&fixture.destination)?.leaf(),
        &fixture.plan.files[0].target,
    )?;
    let same_object = filesystem::identity(&published)? == original;
    let no_op = fixture.resume(&reopened, None)? == super::CopyOutcome::CompletedVerified
        && reopened.history_count()? == events;
    let preserved = fixture.preserved()?;
    if outcome != super::CopyOutcome::CompletedVerified
        || events != 13
        || interrupted != 5
        || !same_object
        || !no_op
        || !preserved
        || states
            .iter()
            .any(|state| *state != super::RecoveredItem::PublishedVerified)
    {
        return Err(failed(
            "Owned reviewed continuation failed its byte/identity/history checks.",
        ));
    }
    Ok(
        serde_json::json!({"policy":"reviewed-copy-continuation-v1","scope":"generated-temporary-only","executionApproval":false,"initialPartialBytes":partial_bytes.to_string(),"interruptedEvents":interrupted,"completedEvents":events,"publishedVerified":states.len(),"originalStageObjectPublished":same_object,"freshVerifiedNoOp":no_op,"sourceAndNeighborPreserved":preserved}),
    )
}
#[cfg(any(test, feature = "fixture-probes"))]
pub(super) fn fixture_probe() -> Result<serde_json::Value> {
    let mut cases = Vec::new();
    for (nested, fault) in [
        (false, None),
        (false, Some(super::CopyFault::AfterPublish)),
        (true, None),
    ] {
        let fixture = Fixture::with_nested(nested)?;
        let journal = fixture.journal()?;
        let outcome = fixture.execute(&journal, fault)?;
        drop(journal);
        let reopened = fixture.journal()?;
        let states = fixture.inspect(&reopened)?;
        let preserved = fixture.preserved()?;
        let rejected = fixture
            .execute(&reopened, None)
            .is_err_and(|error| error.code == ErrorCode::Conflict);
        let verified = states
            .iter()
            .filter(|state| **state == super::RecoveredItem::PublishedVerified)
            .count();
        if !preserved
            || !rejected
            || verified
                != if fault.is_none() {
                    fixture.plan.files.len()
                } else {
                    1
                }
        {
            return Err(failed(
                "Owned catalog-journal fixture failed its actual-state checks.",
            ));
        }
        cases.push(serde_json::json!({"outcome":outcome,"publishedVerified":verified,"sourceAndNeighborPreserved":preserved,"replayBlocked":rejected,"journalEvents":reopened.history_count()?}));
    }
    Ok(
        serde_json::json!({"scope":"generated-temporary-only","catalogSchema":crate::catalog::SCHEMA_VERSION,"completed":cases[0],"interruptedAfterPublish":cases[1],"nested":cases[2],"namespacePreparation":namespace_journal::fixture_probe()?,"reviewedContinuation":resume_fixture_probe()?}),
    )
}

#[cfg(test)]
mod tests {
    pub(super) fn comparison_root(
        fixture: &Fixture,
        files: &[(&str, Vec<u8>)],
    ) -> (String, std::path::PathBuf, Vec<String>) {
        use crate::library::{
            model::{RootMode, ScanFilter},
            native, store,
        };
        let path = fixture
            ._run
            .path()
            .join(format!("comparison-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        for (name, bytes) in files {
            assert!(crate::organization::portable_filename(name));
            std::fs::write(path.join(name), bytes).unwrap();
        }
        let path = path.canonicalize().unwrap();
        let root = native::open_root(&path, None).unwrap();
        let identity = crate::filesystem::identity(&root).unwrap();
        let volume = native::volume_token(&root).unwrap();
        drop(root);
        let id = store::add_root(
            &fixture.library.catalog,
            path.clone(),
            identity.clone(),
            volume.clone(),
            "Owned comparison fixture".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
        let mut ids = Vec::new();
        for (name, bytes) in files {
            let file = native::open_asset(&path, &identity, &volume, Path::new(name)).unwrap();
            let object = Object::of(&file).unwrap();
            let revision = native::revision(&file).unwrap();
            drop(file);
            let asset = uuid::Uuid::new_v4().to_string();
            let retained = asset.clone();
            let root = id.clone();
            let name = name.to_string();
            let size = bytes.len() as i64;
            fixture.library.catalog.call(move|db|{db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,?2,?3,?4,?5,'',?4,?4,'jpg',?6,?7,?8,'now','2026-10-31T00:00:00','filesystem_modified','ready','pending','present',1,'owned-comparison-fixture')",params![retained,root,native::encode_path(Path::new(&name)),name,native::encode_path(Path::new("")),format!("{}:{}",object.volume,object.file),size,revision])?;Ok(())}).unwrap();
            ids.push(asset);
        }
        let root = id.clone();
        fixture.library.catalog.call(move|db|{db.execute("INSERT INTO scans(id,root_id,state,message,started_at) VALUES(?1,?2,'completed','Owned seeded comparison fixture','2026-10-31T00:00:00Z')",params![uuid::Uuid::new_v4().to_string(),root])?;db.execute("UPDATE roots SET availability='online',last_scan='owned-complete' WHERE id=?",[root])?;Ok(())}).unwrap();
        (id, path, ids)
    }
    pub(super) fn wait_comparison(
        workflow: &crate::executor::CopyWorkflow,
        id: String,
    ) -> crate::operations::ImportComparisonJob {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            let job = workflow.import_comparison_job(id.clone()).unwrap();
            if !matches!(
                job.state,
                crate::operations::ImportComparisonState::Queued
                    | crate::operations::ImportComparisonState::Running
                    | crate::operations::ImportComparisonState::CancelRequested
            ) {
                return job;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "read comparison did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    #[test]
    fn import_comparison_reports_renamed_exact_bytes_without_writes_or_managed_source_authority() {
        use crate::operations::{
            ImportComparisonInput, ImportComparisonState, ImportFileComparison,
        };
        let fixture = Fixture::with_nested(true).unwrap();
        let (root, path, ids) = comparison_root(
            &fixture,
            &[
                ("Renamed.JPG", generated_photo_bytes().unwrap()),
                ("Renamed.xmp", COMPANION_BYTES.to_vec()),
                ("Other.JPG", b"different".to_vec()),
            ],
        );
        let owner = fixture.owner.clone();
        let source_root = fixture
            .library
            .catalog
            .call(move |db| {
                Ok(operation_store::load(db, &owner)?.sources[0]
                    .root_id
                    .clone())
            })
            .unwrap();
        let source = source_root.clone();
        fixture
            .library
            .catalog
            .call(move |db| {
                db.execute("UPDATE roots SET mode='referenced' WHERE id=?", [source])?;
                Ok(())
            })
            .unwrap();
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let before: i64 = fixture
            .library
            .catalog
            .call(
                |db| Ok(db.query_row("SELECT count(*) FROM operation_journal", [], |r| r.get(0))?),
            )
            .unwrap();
        let queued = workflow
            .start_import_comparison(
                &fixture.library,
                ImportComparisonInput {
                    source_root_id: None,
                    plan_id: fixture.owner.clone(),
                    manifest_id: fixture.manifest.clone(),
                    scope_root_ids: vec![root.clone()],
                },
            )
            .unwrap();
        let completed = wait_comparison(&workflow, queued.id);
        assert_eq!(
            completed.state,
            ImportComparisonState::Completed,
            "{:?}",
            completed.error
        );
        assert!(completed.bytes_read.parse::<u128>().unwrap() > 0);
        let report = completed.report.unwrap();
        assert!(report.all_scope_available);
        assert!(!report.mutation_authorized);
        assert!(report.bundle_review_required);
        assert_eq!(report.files.len(), 2);
        assert_eq!(report.bundles.len(), 1);
        assert_eq!(
            report.bundles[0].disposition,
            crate::operations::ImportBundleDisposition::ReviewExistingBundle
        );
        assert_eq!(report.bundles[0].existing_groups.len(), 1);
        assert_eq!(report.bundles[0].existing_groups[0].members.len(), 2);
        assert!(
            report
                .files
                .iter()
                .all(|file| file.state == ImportFileComparison::ExactBytesCompared)
        );
        assert!(
            report
                .files
                .iter()
                .flat_map(|file| &file.matches)
                .all(|found| ids[..2].contains(&found.asset_id))
        );
        let after: i64 = fixture
            .library
            .catalog
            .call(
                |db| Ok(db.query_row("SELECT count(*) FROM operation_journal", [], |r| r.get(0))?),
            )
            .unwrap();
        assert_eq!(before, after);
        assert!(fixture.preserved().unwrap());
        assert_eq!(
            std::fs::read(path.join("Renamed.JPG")).unwrap(),
            generated_photo_bytes().unwrap()
        );
        // Read-only comparison is not a managed-write grant for ordinary copying.
        let target = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&target).unwrap();
        let volume = crate::library::native::volume_token(&target).unwrap();
        drop(target);
        assert!(
            fixture
                .library
                .organization_copy_review_create(
                    fixture.owner.clone(),
                    fixture.manifest.clone(),
                    uuid::Uuid::new_v4().to_string(),
                    fixture.destination.clone(),
                    (identity, volume)
                )
                .is_err()
        );
        assert!(
            workflow
                .start_import_comparison(
                    &fixture.library,
                    ImportComparisonInput {
                        source_root_id: None,
                        plan_id: fixture.owner.clone(),
                        manifest_id: fixture.manifest.clone(),
                        scope_root_ids: vec![source_root]
                    }
                )
                .is_err()
        );
    }
    #[test]
    fn import_comparison_mock_collisions_offline_scope_and_candidate_errors_never_claim_absence() {
        use crate::jobs::paced_io::IoStep;
        use crate::operations::{ImportComparisonInput, ImportFileComparison};
        let fixture = Fixture::with_nested(true).unwrap();
        let mut different = generated_photo_bytes().unwrap();
        different[0] ^= 1;
        let (root, path, ids) = comparison_root(&fixture, &[("Collision.JPG", different.clone())]);
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let input = ImportComparisonInput {
            source_root_id: None,
            plan_id: fixture.owner.clone(),
            manifest_id: fixture.manifest.clone(),
            scope_root_ids: vec![root.clone()],
        };
        let IoStep::Completed(report) =
            crate::executor::workflow::compare_fixture(&workflow, input.clone(), true).unwrap()
        else {
            panic!("unexpected cancellation")
        };
        assert!(report.files.iter().all(|file| file.matches.is_empty()));
        assert!(
            report
                .files
                .iter()
                .all(|file| file.state == ImportFileComparison::NoIndexedMatch)
        );
        assert!(!report.mutation_authorized);
        let inaccessible = ids[0].clone();
        fixture
            .library
            .catalog
            .call(move |db| {
                db.execute(
                    "UPDATE assets SET presence='inaccessible' WHERE id=?",
                    [inaccessible],
                )?;
                Ok(())
            })
            .unwrap();
        let IoStep::Completed(unresolved) =
            crate::executor::workflow::compare_fixture(&workflow, input.clone(), false).unwrap()
        else {
            panic!("unexpected cancellation")
        };
        assert!(!unresolved.all_scope_available);
        assert_eq!(unresolved.excluded_or_missing_files, 0);
        assert!(
            unresolved
                .files
                .iter()
                .all(|file| file.state == ImportFileComparison::NeedsAttention)
        );
        let accessible = ids[0].clone();
        fixture
            .library
            .catalog
            .call(move |db| {
                db.execute(
                    "UPDATE assets SET presence='present' WHERE id=?",
                    [accessible],
                )?;
                Ok(())
            })
            .unwrap();
        // Stale indexed metadata is an explicit error, even when restored mtime
        // would make a size/mtime-only cache appear valid.
        let modified = std::fs::metadata(path.join("Collision.JPG"))
            .unwrap()
            .modified()
            .unwrap();
        std::fs::write(path.join("Collision.JPG"), generated_photo_bytes().unwrap()).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(path.join("Collision.JPG"))
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        let IoStep::Completed(report) =
            crate::executor::workflow::compare_fixture(&workflow, input.clone(), false).unwrap()
        else {
            panic!("unexpected cancellation")
        };
        assert!(!report.all_scope_available);
        assert!(
            report
                .files
                .iter()
                .all(|file| file.state == ImportFileComparison::NeedsAttention)
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.asset_id.as_ref() == Some(&ids[0]))
        );
        let offline = root.clone();
        fixture
            .library
            .catalog
            .call(move |db| {
                db.execute(
                    "UPDATE roots SET availability='offline' WHERE id=?",
                    [offline],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(crate::executor::workflow::compare_fixture(&workflow, input, false).is_err());
        assert!(fixture.preserved().unwrap());
        assert_eq!(
            std::fs::read(path.join("Collision.JPG")).unwrap(),
            generated_photo_bytes().unwrap()
        );
    }
    #[test]
    fn import_decisions_retain_whole_bundle_native_proofs_and_reuse_exact_selection() {
        use crate::operations::*;
        let fixture = Fixture::with_nested(true).unwrap();
        let (root, path, _) = comparison_root(
            &fixture,
            &[
                ("Renamed.JPG", generated_photo_bytes().unwrap()),
                ("Renamed.xmp", COMPANION_BYTES.to_vec()),
            ],
        );
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let job = workflow
            .start_import_comparison(
                &fixture.library,
                ImportComparisonInput {
                    source_root_id: None,
                    plan_id: fixture.owner.clone(),
                    manifest_id: fixture.manifest.clone(),
                    scope_root_ids: vec![root],
                },
            )
            .unwrap();
        let job = wait_comparison(&workflow, job.id);
        let bundle = &job.report.as_ref().unwrap().bundles[0];
        let input = ImportPlanInput {
            comparison_job_id: job.id.clone(),
            choices: vec![ImportBundleChoice {
                file_id: bundle.file_ids[0].clone(),
                existing_group_id: Some(bundle.existing_groups[0].id.clone()),
            }],
        };
        let retained = workflow
            .create_import_plan(&fixture.library, input.clone())
            .unwrap();
        assert_eq!(retained.skip_files.len(), 2);
        assert!(retained.copy_file_ids.is_empty());
        assert!(retained.sources_retained);
        assert!(retained.execution_available);
        let repeated = workflow
            .create_import_plan(&fixture.library, input.clone())
            .unwrap();
        assert_eq!(retained.id, repeated.id);
        assert_eq!(retained.digest, repeated.digest);
        let mut copy = input.clone();
        copy.choices[0].existing_group_id = None;
        let copy = workflow.create_import_plan(&fixture.library, copy).unwrap();
        assert_eq!(copy.copy_file_ids.len(), 2);
        assert!(copy.skip_files.is_empty());
        assert_ne!(copy.id, retained.id);
        for choices in [
            vec![],
            vec![input.choices[0].clone(), input.choices[0].clone()],
            vec![ImportBundleChoice {
                file_id: "foreign".into(),
                existing_group_id: input.choices[0].existing_group_id.clone(),
            }],
            vec![ImportBundleChoice {
                file_id: input.choices[0].file_id.clone(),
                existing_group_id: Some("f".repeat(64)),
            }],
        ] {
            assert!(
                workflow
                    .create_import_plan(
                        &fixture.library,
                        ImportPlanInput {
                            comparison_job_id: job.id.clone(),
                            choices
                        }
                    )
                    .is_err()
            );
        }
        workflow.discard_import_comparison(job.id).unwrap();
        drop(workflow);
        let reopened = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let opened = reopened
            .open_import_plan(fixture.owner.clone(), retained.id.clone())
            .unwrap();
        let summaries = reopened
            .import_plan_summaries(fixture.owner.clone(), None)
            .unwrap();
        assert_eq!(summaries.len(), 2);
        assert!(
            summaries
                .iter()
                .all(|summary| summary.manifest_id == fixture.manifest)
        );
        assert_eq!(opened.digest, retained.digest);
        assert!(opened.execution_available);
        assert!(
            reopened
                .open_import_plan(uuid::Uuid::new_v4().to_string(), retained.id)
                .is_err()
        );
        assert!(fixture.preserved().unwrap());
        assert_eq!(
            std::fs::read(path.join("Renamed.xmp")).unwrap(),
            COMPANION_BYTES
        );
        let owner = fixture.owner.clone();
        let count:i64=fixture.library.catalog.call(move|db|Ok(db.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=? AND phase='import_selection_plan_v1'",[owner],|row|row.get(0))?)).unwrap();
        assert_eq!(count, 2);
    }
    #[test]
    fn import_decision_save_rejects_changed_source_or_keeper_without_writing_a_plan() {
        use crate::operations::*;
        for source_changed in [false, true] {
            let fixture = Fixture::with_nested(true).unwrap();
            let (root, path, _) = comparison_root(
                &fixture,
                &[
                    ("Renamed.JPG", generated_photo_bytes().unwrap()),
                    ("Renamed.xmp", COMPANION_BYTES.to_vec()),
                ],
            );
            let workflow = crate::executor::CopyWorkflow::new(
                fixture.library.catalog.clone(),
                fixture._run.path().join("data"),
            )
            .unwrap();
            let job = workflow
                .start_import_comparison(
                    &fixture.library,
                    ImportComparisonInput {
                        source_root_id: None,
                        plan_id: fixture.owner.clone(),
                        manifest_id: fixture.manifest.clone(),
                        scope_root_ids: vec![root],
                    },
                )
                .unwrap();
            let job = wait_comparison(&workflow, job.id);
            let bundle = &job.report.as_ref().unwrap().bundles[0];
            let input = ImportPlanInput {
                comparison_job_id: job.id,
                choices: vec![ImportBundleChoice {
                    file_id: bundle.file_ids[0].clone(),
                    existing_group_id: Some(bundle.existing_groups[0].id.clone()),
                }],
            };
            let changed = if source_changed {
                fixture.source.join("source-folder/Photo.xmp")
            } else {
                path.join("Renamed.xmp")
            };
            let mut bytes = COMPANION_BYTES.to_vec();
            bytes[0] ^= 1;
            std::fs::write(&changed, &bytes).unwrap();
            assert!(
                workflow
                    .create_import_plan(&fixture.library, input)
                    .is_err()
            );
            assert_eq!(std::fs::read(&changed).unwrap(), bytes);
            let owner = fixture.owner.clone();
            let count:i64=fixture.library.catalog.call(move|db|Ok(db.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=? AND phase='import_selection_plan_v1'",[owner],|row|row.get(0))?)).unwrap();
            assert_eq!(count, 0);
        }
    }
    fn import_skip_plan(
        fixture: &Fixture,
        workflow: &Arc<crate::executor::CopyWorkflow>,
        root: String,
    ) -> crate::operations::ImportPlan {
        use crate::operations::*;
        let job = workflow
            .start_import_comparison(
                &fixture.library,
                ImportComparisonInput {
                    source_root_id: None,
                    plan_id: fixture.owner.clone(),
                    manifest_id: fixture.manifest.clone(),
                    scope_root_ids: vec![root],
                },
            )
            .unwrap();
        let job = wait_comparison(workflow, job.id);
        let report = job.report.as_ref().unwrap();
        assert!(report.all_scope_available);
        workflow
            .create_import_plan(
                &fixture.library,
                ImportPlanInput {
                    comparison_job_id: job.id,
                    choices: report
                        .bundles
                        .iter()
                        .map(|bundle| ImportBundleChoice {
                            file_id: bundle.file_ids[0].clone(),
                            existing_group_id: Some(bundle.existing_groups[0].id.clone()),
                        })
                        .collect(),
                },
            )
            .unwrap()
    }
    fn import_approval(
        fixture: &Fixture,
        workflow: &crate::executor::CopyWorkflow,
        plan: &crate::operations::ImportPlan,
    ) -> crate::operations::ImportSkipStartInput {
        use crate::operations::*;
        let approval = workflow
            .approve_import_skip(
                &fixture.library,
                ImportSkipApprovalInput {
                    plan_id: fixture.owner.clone(),
                    import_plan_id: plan.id.clone(),
                    digest: plan.digest.clone(),
                    sources_retained_acknowledged: true,
                },
            )
            .unwrap();
        ImportSkipStartInput {
            plan_id: fixture.owner.clone(),
            import_plan_id: plan.id.clone(),
            digest: plan.digest.clone(),
            approval_id: approval.id,
        }
    }
    pub(super) fn wait_import(
        fixture: &Fixture,
        workflow: &crate::executor::CopyWorkflow,
        id: String,
    ) -> crate::operations::ImportSession {
        use crate::operations::ImportSessionState::*;
        let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            let job = workflow
                .import_session(fixture.owner.clone(), id.clone())
                .unwrap();
            if !matches!(job.state, Queued | Running | CancelRequested) {
                return job;
            }
            assert!(
                std::time::Instant::now() < until,
                "import verification did not settle"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[test]
    fn import_skip_freshly_verifies_all_members_records_history_and_creates_no_output() {
        use crate::operations::*;
        let fixture = Fixture::with_nested(true).unwrap();
        let (root, path, _) = comparison_root(
            &fixture,
            &[
                ("Renamed.JPG", generated_photo_bytes().unwrap()),
                ("Renamed.xmp", COMPANION_BYTES.to_vec()),
            ],
        );
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let plan = import_skip_plan(&fixture, &workflow, root);
        let revoked = import_approval(&fixture, &workflow, &plan);
        workflow
            .release_import_plan(fixture.owner.clone(), plan.id.clone())
            .unwrap();
        assert!(
            workflow
                .start_import_skip(&fixture.library, revoked)
                .is_err()
        );
        let replaced = import_approval(&fixture, &workflow, &plan);
        let _new_approval = import_approval(&fixture, &workflow, &plan);
        assert!(
            workflow
                .start_import_skip(&fixture.library, replaced)
                .is_err()
        );
        assert!(
            workflow
                .start_import_skip(
                    &fixture.library,
                    ImportSkipStartInput {
                        plan_id: fixture.owner.clone(),
                        import_plan_id: plan.id.clone(),
                        digest: plan.digest.clone(),
                        approval_id: uuid::Uuid::new_v4().to_string()
                    }
                )
                .is_err()
        );
        assert!(
            workflow
                .approve_import_skip(
                    &fixture.library,
                    ImportSkipApprovalInput {
                        plan_id: fixture.owner.clone(),
                        import_plan_id: plan.id.clone(),
                        digest: plan.digest.clone(),
                        sources_retained_acknowledged: false
                    }
                )
                .is_err()
        );
        let mut verified = Vec::new();
        for _ in 0..2 {
            let input = import_approval(&fixture, &workflow, &plan);
            let job = workflow
                .start_import_skip(&fixture.library, input.clone())
                .unwrap();
            let job = wait_import(&fixture, &workflow, job.id);
            assert_eq!(job.state, ImportSessionState::Completed, "{:?}", job.error);
            assert_eq!(job.verified_files, 2);
            assert!(!job.output_created);
            assert!(job.sources_retained);
            assert_eq!(
                job.bytes_read.parse::<u128>().unwrap(),
                2 * (generated_photo_bytes().unwrap().len() + COMPANION_BYTES.len()) as u128
            );
            assert!(workflow.start_import_skip(&fixture.library, input).is_err());
            assert!(fixture.preserved().unwrap());
            assert_eq!(
                std::fs::read_dir(fixture.destination.join("organized"))
                    .unwrap()
                    .count(),
                0
            );
            verified.push(job);
        }
        assert_ne!(verified[0].id, verified[1].id);
        let old_ticket = import_approval(&fixture, &workflow, &plan);
        drop(workflow);
        std::fs::rename(&path, path.with_extension("offline")).unwrap();
        let fresh = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let history = fresh
            .import_session(fixture.owner.clone(), verified[0].id.clone())
            .unwrap();
        assert_eq!(history.state, ImportSessionState::Completed);
        assert!(
            fresh
                .start_import_skip(&fixture.library, old_ticket)
                .is_err()
        );
        assert!(
            fresh
                .approve_import_skip(
                    &fixture.library,
                    ImportSkipApprovalInput {
                        plan_id: fixture.owner.clone(),
                        import_plan_id: plan.id,
                        digest: plan.digest,
                        sources_retained_acknowledged: true
                    }
                )
                .is_err()
        );
    }
    #[test]
    fn import_skip_wrong_digest_consumes_ticket_and_changed_keeper_never_records_completion() {
        use crate::operations::*;
        let fixture = Fixture::with_nested(true).unwrap();
        let (root, path, _) = comparison_root(
            &fixture,
            &[
                ("Renamed.JPG", generated_photo_bytes().unwrap()),
                ("Renamed.xmp", COMPANION_BYTES.to_vec()),
            ],
        );
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let plan = import_skip_plan(&fixture, &workflow, root);
        let input = import_approval(&fixture, &workflow, &plan);
        let mut wrong = input.clone();
        wrong.digest = "f".repeat(64);
        assert!(workflow.start_import_skip(&fixture.library, wrong).is_err());
        assert!(workflow.start_import_skip(&fixture.library, input).is_err());
        let input = import_approval(&fixture, &workflow, &plan);
        let modified = std::fs::metadata(path.join("Renamed.xmp"))
            .unwrap()
            .modified()
            .unwrap();
        let mut changed = COMPANION_BYTES.to_vec();
        changed[0] ^= 1;
        std::fs::write(path.join("Renamed.xmp"), &changed).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(path.join("Renamed.xmp"))
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        let job = workflow.start_import_skip(&fixture.library, input).unwrap();
        let job = wait_import(&fixture, &workflow, job.id);
        assert_eq!(job.state, ImportSessionState::Failed);
        assert!(job.error.is_some());
        assert!(!job.output_created);
        assert_eq!(job.verified_files, 0);
        assert_eq!(std::fs::read(path.join("Renamed.xmp")).unwrap(), changed);
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn import_skip_journal_denial_retains_a_needs_recovery_attempt_without_false_success() {
        use crate::operations::*;
        let fixture = Fixture::with_nested(true).unwrap();
        let (root, _, _) = comparison_root(
            &fixture,
            &[
                ("Renamed.JPG", generated_photo_bytes().unwrap()),
                ("Renamed.xmp", COMPANION_BYTES.to_vec()),
            ],
        );
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let plan = import_skip_plan(&fixture, &workflow, root);
        let input = import_approval(&fixture, &workflow, &plan);
        fixture.library.catalog.call(|db|{db.execute_batch("CREATE TRIGGER fixture_deny_import_completion BEFORE INSERT ON operation_journal WHEN NEW.phase='import_skip_outcome_v1' BEGIN SELECT RAISE(ABORT,'Owned import journal denied completion'); END;")?;Ok(())}).unwrap();
        let job = workflow.start_import_skip(&fixture.library, input).unwrap();
        let job = wait_import(&fixture, &workflow, job.id);
        assert_eq!(job.state, ImportSessionState::Failed);
        assert_eq!(job.verified_files, 2);
        assert!(job.error.is_some());
        assert!(fixture.preserved().unwrap());
        workflow
            .discard_import_session(fixture.owner.clone(), job.id.clone())
            .unwrap();
        let retained = workflow
            .import_session(fixture.owner.clone(), job.id)
            .unwrap();
        assert_eq!(retained.state, ImportSessionState::NeedsRecovery);
        assert_eq!(retained.verified_files, 0);
    }
    #[test]
    fn import_skip_root_cancellation_keeps_source_and_keeper_and_releases_admission() {
        use crate::operations::*;
        let mut bytes = generated_photo_bytes().unwrap();
        bytes.extend(vec![81; 2 * 1024 * 1024]);
        let fixture = Fixture::with_media(true, bytes.clone()).unwrap();
        let (root, path, _) = comparison_root(
            &fixture,
            &[
                ("Renamed.JPG", bytes.clone()),
                ("Renamed.xmp", COMPANION_BYTES.to_vec()),
            ],
        );
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let plan = import_skip_plan(&fixture, &workflow, root.clone());
        let input = import_approval(&fixture, &workflow, &plan);
        let job = workflow.start_import_skip(&fixture.library, input).unwrap();
        assert!(workflow.cancel_root(&root).unwrap());
        let job = wait_import(&fixture, &workflow, job.id);
        assert_eq!(job.state, ImportSessionState::Canceled);
        assert!(!workflow.has_active_root(&root).unwrap());
        assert!(fixture.preserved().unwrap());
        assert_eq!(std::fs::read(path.join("Renamed.JPG")).unwrap(), bytes);
        let retry = workflow
            .start_import_skip(
                &fixture.library,
                import_approval(&fixture, &workflow, &plan),
            )
            .unwrap();
        assert_eq!(
            wait_import(&fixture, &workflow, retry.id).state,
            ImportSessionState::Completed
        );
        let closing = workflow
            .start_import_skip(
                &fixture.library,
                import_approval(&fixture, &workflow, &plan),
            )
            .unwrap();
        workflow
            .discard_import_session(fixture.owner.clone(), closing.id.clone())
            .unwrap();
        let closed = wait_import(&fixture, &workflow, closing.id);
        assert_eq!(closed.state, ImportSessionState::Canceled);
        assert!(!workflow.has_active_root(&root).unwrap());
        assert!(fixture.preserved().unwrap());
    }
    pub(super) fn import_draft_for(
        fixture: &Fixture,
        ids: Vec<String>,
    ) -> crate::operations::CopyManifest {
        use crate::operations::*;
        let owner = fixture.owner.clone();
        let destination_root_id = fixture
            .library
            .catalog
            .call(move |db| Ok(operation_store::load(db, &owner)?.view.destination_root_id))
            .unwrap();
        let template = crate::organization::OrganizationTemplate {
            pattern: "Imports/{original_stem}.{ext}".into(),
            event: String::new(),
            missing_date: crate::organization::MissingDatePolicy::Block,
            filename_stem_case: None,
        };
        let preview = fixture
            .library
            .organization_preview(ids.clone(), template.clone())
            .unwrap();
        let target = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&target).unwrap();
        let volume = crate::library::native::volume_token(&target).unwrap();
        drop(target);
        let draft = fixture
            .library
            .organization_plan_create(
                PlanDraftInput {
                    destination_root_id,
                    asset_ids: ids.clone(),
                    template,
                    preview_fingerprint: preview.fingerprint,
                    template_reference: None,
                },
                fixture.destination.clone(),
                (identity, volume),
            )
            .unwrap();
        let owner = draft.plan.id.clone();
        let retained = fixture
            .library
            .catalog
            .call(move |db| operation_store::load(db, &owner))
            .unwrap();
        let mut decisions = Vec::new();
        for source in retained.sources {
            let inventory = fixture
                .library
                .organization_bundle_inventory(source.id.clone(), source.revision.clone())
                .unwrap();
            decisions.push(
                fixture
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
                    .unwrap()
                    .id,
            );
        }
        fixture
            .library
            .organization_copy_manifest(CopyManifestInput {
                plan_id: draft.plan.id,
                bundle_confirmation_ids: decisions,
            })
            .unwrap()
    }
    fn mixed_import_fixture() -> (
        Fixture,
        Arc<crate::executor::CopyWorkflow>,
        crate::operations::ImportPlan,
        String,
        String,
    ) {
        mixed_import_fixture_with_destination(None)
    }
    fn mixed_import_fixture_with_destination(
        destination_mode: Option<crate::library::model::RootMode>,
    ) -> (
        Fixture,
        Arc<crate::executor::CopyWorkflow>,
        crate::operations::ImportPlan,
        String,
        String,
    ) {
        use crate::operations::*;
        let mut fixture = Fixture::with_media_destination(
            true,
            generated_photo_bytes().unwrap(),
            None,
            destination_mode,
        )
        .unwrap();
        let owner = fixture.owner.clone();
        let source = fixture
            .library
            .catalog
            .call(move |db| Ok(operation_store::load(db, &owner)?.sources[0].clone()))
            .unwrap();
        let folder = fixture.source.join("z-new");
        std::fs::create_dir(&folder).unwrap();
        let mut new_photo = generated_photo_bytes().unwrap();
        new_photo.extend_from_slice(b"new generated fixture");
        let new_sidecar = b"<x:xmpmeta>new generated companion</x:xmpmeta>".to_vec();
        std::fs::write(folder.join("New.JPG"), &new_photo).unwrap();
        std::fs::write(folder.join("New.xmp"), &new_sidecar).unwrap();
        fixture.baseline.extend([
            ("z-new/New.JPG".into(), new_photo.clone()),
            ("z-new/New.xmp".into(), new_sidecar.clone()),
        ]);
        let file = crate::filesystem::open_readonly(&folder.join("New.JPG")).unwrap();
        let object = Object::of(&file).unwrap();
        let revision = crate::library::native::revision(&file).unwrap();
        drop(file);
        let id = uuid::Uuid::new_v4().to_string();
        let inserted = id.clone();
        let root = source.root_id.clone();
        fixture.library.catalog.call(move|db|{db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,?2,?3,'z-new/New.JPG',?4,'z-new','New.JPG','New','jpg',?5,?6,?7,'now','2026-10-31T00:00:00','filesystem_modified','ready','pending','present',1,'owned-fixture')",params![inserted,root,crate::library::native::encode_path(Path::new("z-new/New.JPG")),crate::library::native::encode_path(Path::new("z-new")),format!("{}:{}",object.volume,object.file),new_photo.len() as i64,revision])?;db.execute("UPDATE library_state SET revision=revision+1",[])?;Ok(())}).unwrap();
        let (root, _, _) = comparison_root(
            &fixture,
            &[
                ("Renamed.JPG", generated_photo_bytes().unwrap()),
                ("Renamed.xmp", COMPANION_BYTES.to_vec()),
            ],
        );
        let only_new = import_draft_for(&fixture, vec![id.clone()]);
        let expected = crate::executor::sources::Snapshot::build(
            &fixture.library.catalog,
            only_new.plan_id,
            only_new.id,
        )
        .unwrap();
        let expected_digest = fixture
            .library
            .catalog
            .call(move |db| Ok(crate::executor::effects::Effect::capture(db, &expected)?.digest))
            .unwrap();
        let manifest = import_draft_for(&fixture, vec![source.id, id]);
        fixture.owner = manifest.plan_id.clone();
        fixture.manifest = manifest.id.clone();
        assert_eq!(manifest.files.len(), 4);
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let job = workflow
            .start_import_comparison(
                &fixture.library,
                ImportComparisonInput {
                    source_root_id: None,
                    plan_id: fixture.owner.clone(),
                    manifest_id: fixture.manifest.clone(),
                    scope_root_ids: vec![root],
                },
            )
            .unwrap();
        let job = wait_comparison(&workflow, job.id);
        let plan = workflow
            .create_import_plan(
                &fixture.library,
                ImportPlanInput {
                    comparison_job_id: job.id.clone(),
                    choices: job
                        .report
                        .as_ref()
                        .unwrap()
                        .bundles
                        .iter()
                        .map(|bundle| ImportBundleChoice {
                            file_id: bundle.file_ids[0].clone(),
                            existing_group_id: bundle
                                .existing_groups
                                .first()
                                .map(|group| group.id.clone()),
                        })
                        .collect(),
                },
            )
            .unwrap();
        assert_eq!(plan.copy_file_ids.len(), 2);
        assert_eq!(plan.skip_files.len(), 2);
        (fixture, workflow, plan, source.root_id, expected_digest)
    }
    #[test]
    fn import_copy_projection_preserves_ids_memberships_targets_and_legacy_encoding() {
        let (fixture, workflow, plan, root, expected_digest) = mixed_import_fixture();
        let original = crate::executor::sources::Snapshot::build(
            &fixture.library.catalog,
            fixture.owner.clone(),
            fixture.manifest.clone(),
        )
        .unwrap();
        assert!(
            !serde_json::to_value(&original)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("import_selection")
        );
        let projected = workflow
            .import_copy_sources(fixture.owner.clone(), plan.id.clone())
            .unwrap();
        assert_eq!(projected.files.len(), 2);
        assert!(
            projected
                .files
                .iter()
                .all(|file| plan.copy_file_ids.contains(&file.id)
                    && file.target.starts_with("Imports/New."))
        );
        let snapshot = projected.clone();
        let effect = fixture
            .library
            .catalog
            .call(move |db| Ok(crate::executor::effects::Effect::capture(db, &snapshot)?.digest))
            .unwrap();
        assert_eq!(
            effect, expected_digest,
            "effect memberships must be looked up by ID, not zipped against skipped prefix files"
        );
        let mut changes = Vec::new();
        let mut changed = projected.clone();
        changed.files.push(original.files[0].clone());
        changes.push(changed);
        let mut changed = projected.clone();
        changed.files[0].target = "renumbered.JPG".into();
        changes.push(changed);
        let mut changed = projected.clone();
        changed.import_selection.as_mut().unwrap().digest = "f".repeat(64);
        changes.push(changed);
        let mut changed = projected.clone();
        changed.import_selection.as_mut().unwrap().id = uuid::Uuid::new_v4().to_string();
        changes.push(changed);
        for changed in changes {
            assert!(
                fixture
                    .library
                    .catalog
                    .call(move |db| changed.validate(db))
                    .is_err()
            );
        }
        let changed = root.clone();
        fixture
            .library
            .catalog
            .call(move |db| {
                db.execute("UPDATE roots SET mode='referenced' WHERE id=?", [changed])?;
                Ok(())
            })
            .unwrap();
        assert!(
            projected
                .open_scope(
                    &fixture.library.catalog,
                    crate::jobs::operation_locks::Access::Write
                )
                .is_err()
        );
        let referenced = workflow
            .import_copy_sources(fixture.owner.clone(), plan.id)
            .unwrap();
        assert_eq!(
            referenced.required_mode(),
            crate::library::model::RootMode::Referenced
        );
        assert!(
            crate::executor::sources::Snapshot::build(
                &fixture.library.catalog,
                fixture.owner.clone(),
                fixture.manifest.clone()
            )
            .is_err()
        );
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn import_copy_projection_prepares_and_publishes_only_new_bundle_with_referenced_source() {
        use crate::executor::CopyOutcome;
        use crate::executor::approval::Approvals;
        use crate::jobs::{operation_locks::Access, paced_io::PacedIo};
        use std::sync::atomic::AtomicBool;
        for referenced in [false, true] {
            let (fixture, workflow, plan, root, expected_digest) = mixed_import_fixture();
            if referenced {
                fixture
                    .library
                    .catalog
                    .call(move |db| {
                        db.execute("UPDATE roots SET mode='referenced' WHERE id=?", [root])?;
                        Ok(())
                    })
                    .unwrap();
            }
            let snapshot = workflow
                .import_copy_sources(fixture.owner.clone(), plan.id.clone())
                .unwrap();
            let sources = snapshot.clone();
            let effect = fixture
                .library
                .catalog
                .call(move |db| crate::executor::effects::Effect::capture(db, &sources))
                .unwrap();
            assert_eq!(effect.digest, expected_digest);
            let namespace = namespace_journal::NamespaceJournal::bind_effect(
                fixture.library.catalog.clone(),
                snapshot.clone(),
                uuid::Uuid::new_v4().to_string(),
                true,
                Some(effect),
            )
            .unwrap();
            let lease = snapshot
                .open_scope(&fixture.library.catalog, Access::Write)
                .unwrap();
            let selection = uuid::Uuid::new_v4().to_string();
            let approvals = Approvals::new();
            let review = namespace.preparation_review(&selection).unwrap();
            let ticket = approvals.issue(review.clone()).unwrap();
            namespace
                .prepare_approved(
                    lease.directory(1).leaf(),
                    &AtomicBool::new(false),
                    &selection,
                    approvals.consume(&ticket, &review).unwrap(),
                )
                .unwrap();
            let journal = namespace.ready_journal().unwrap();
            assert_eq!(journal.plan().files.len(), 2);
            assert!(
                journal
                    .plan()
                    .files
                    .iter()
                    .zip(&snapshot.files)
                    .all(|(file, source)| file.object == source.object
                        && file.destination_location.as_ref().map_or_else(
                            || file.target.clone(),
                            |location| format!("{}/{}", location.relative_parent, file.target)
                        ) == source.target
                        && file
                            .source_location
                            .as_ref()
                            .is_some_and(|location| location.relative_native
                                == source.location.relative_native))
            );
            let output =
                crate::executor::output_chain(&fixture.destination.join(namespace.namespace()))
                    .unwrap();
            let io = PacedIo::shared();
            let cancel = AtomicBool::new(false);
            let validate = || snapshot.validate_roots(&fixture.library.catalog);
            let review = journal.copy_review(&selection).unwrap();
            let ticket = approvals.issue(review.clone()).unwrap();
            let outcome = journal
                .execute_approved(
                    crate::executor::CopyInputs {
                        source_root: lease.directory(0).leaf(),
                        destination: output.leaf(),
                        io: &io,
                        cancel: &cancel,
                        validate_scope: &validate,
                    },
                    &selection,
                    approvals.consume(&ticket, &review).unwrap(),
                    None,
                    None,
                )
                .unwrap();
            assert_eq!(outcome, CopyOutcome::CompletedVerified);
            assert_eq!(journal.verified_files().unwrap(), 2);
            for member in &snapshot.files {
                assert_eq!(
                    std::fs::read(
                        fixture
                            .destination
                            .join(namespace.namespace())
                            .join(&member.target)
                    )
                    .unwrap(),
                    std::fs::read(
                        fixture.source.join(
                            crate::library::native::decode_path(&member.location.relative_native)
                                .unwrap()
                        )
                    )
                    .unwrap()
                );
            }
            assert!(
                !fixture
                    .destination
                    .join(namespace.namespace())
                    .join("Imports/Photo.jpg")
                    .exists()
            );
            assert!(
                !fixture
                    .destination
                    .join(namespace.namespace())
                    .join("Imports/Photo.xmp")
                    .exists()
            );
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn import_copy_projection_grant_change_retains_empty_stages_without_publication() {
        use crate::executor::{RecoveredItem, approval::Approvals};
        use crate::jobs::{operation_locks::Access, paced_io::PacedIo};
        use std::sync::atomic::AtomicBool;
        let (fixture, workflow, plan, root, _) = mixed_import_fixture();
        let changed = root.clone();
        fixture
            .library
            .catalog
            .call(move |db| {
                db.execute("UPDATE roots SET mode='referenced' WHERE id=?", [changed])?;
                Ok(())
            })
            .unwrap();
        let snapshot = workflow
            .import_copy_sources(fixture.owner.clone(), plan.id)
            .unwrap();
        let namespace = namespace_journal::NamespaceJournal::bind_snapshot(
            fixture.library.catalog.clone(),
            snapshot.clone(),
            uuid::Uuid::new_v4().to_string(),
            false,
        )
        .unwrap();
        let lease = snapshot
            .open_scope(&fixture.library.catalog, Access::Write)
            .unwrap();
        let approvals = Approvals::new();
        let selection = uuid::Uuid::new_v4().to_string();
        let review = namespace.preparation_review(&selection).unwrap();
        let ticket = approvals.issue(review.clone()).unwrap();
        namespace
            .prepare_approved(
                lease.directory(1).leaf(),
                &AtomicBool::new(false),
                &selection,
                approvals.consume(&ticket, &review).unwrap(),
            )
            .unwrap();
        let journal = namespace.ready_journal().unwrap();
        let output =
            crate::executor::output_chain(&fixture.destination.join(namespace.namespace()))
                .unwrap();
        let io = PacedIo::shared();
        let cancel = AtomicBool::new(false);
        let changed = AtomicBool::new(false);
        let validate = || {
            // Revoke only after the stream's initial catalog validation, then
            // return success: the journal's own boundary check must refuse it.
            if !changed.swap(true, std::sync::atomic::Ordering::AcqRel) {
                let root = root.clone();
                fixture.library.catalog.call(move |db| {
                    db.execute("UPDATE roots SET mode='managed' WHERE id=?", [root])?;
                    Ok(())
                })?;
            }
            Ok(())
        };
        let review = journal.copy_review(&selection).unwrap();
        let ticket = approvals.issue(review.clone()).unwrap();
        let error = journal
            .execute_approved(
                crate::executor::CopyInputs {
                    source_root: lease.directory(0).leaf(),
                    destination: output.leaf(),
                    io: &io,
                    cancel: &cancel,
                    validate_scope: &validate,
                },
                &selection,
                approvals.consume(&ticket, &review).unwrap(),
                None,
                None,
            )
            .err()
            .unwrap();
        assert!(error.message.contains("referenced grant changed"));
        assert_eq!(journal.history_count().unwrap(), 0);
        let inspected = journal
            .inspect(crate::executor::CopyInputs {
                source_root: lease.directory(0).leaf(),
                destination: output.leaf(),
                io: &io,
                cancel: &cancel,
                validate_scope: &validate,
            })
            .unwrap();
        assert_eq!(
            inspected,
            vec![
                RecoveredItem::PreparedEmptyStage,
                RecoveredItem::PreparedEmptyStage
            ]
        );
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn import_projected_workflow_reviews_subset_and_separately_approves_both_actions() {
        if crate::test_process::isolated(
            "executor::catalog_journal::tests::import_projected_workflow_reviews_subset_and_separately_approves_both_actions",
            60,
        ) {
            return;
        }
        use crate::operations::{CopyAction, CopyApprovalInput, CopyJobState, CopyStartInput};
        for variant in 0..6 {
            eprintln!("Generated projected import variant {variant}: setup");
            let (fixture, workflow, plan, source_root, _) = mixed_import_fixture_with_destination(
                (variant == 5).then_some(crate::library::model::RootMode::Managed),
            );
            if variant == 1 {
                fixture
                    .library
                    .catalog
                    .call(move |db| {
                        db.execute(
                            "UPDATE roots SET mode='referenced' WHERE id=?",
                            [source_root],
                        )?;
                        Ok(())
                    })
                    .unwrap();
            }
            let original = fixture
                .library
                .organization_copy_manifest_open(fixture.owner.clone(), fixture.manifest.clone())
                .unwrap();
            let sources = workflow
                .import_copy_sources(fixture.owner.clone(), plan.id.clone())
                .unwrap();
            let logical = sources
                .files
                .iter()
                .map(|file| file.bytes as u128)
                .sum::<u128>();
            let selected = crate::library::native::open_root(&fixture.destination, None).unwrap();
            let identity = crate::filesystem::identity(&selected).unwrap();
            let volume = crate::library::native::volume_token(&selected).unwrap();
            drop(selected);
            let selection = uuid::Uuid::new_v4().to_string();
            let mut review = workflow
                .create_import_review(
                    &fixture.library,
                    fixture.owner.clone(),
                    plan.id.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            assert_eq!(review.files.len(), 2);
            assert_eq!(
                review.assessment.capacity.logical_bytes,
                logical.to_string()
            );
            assert!(logical < original.selected_bytes.parse::<u128>().unwrap());
            assert_eq!(review.assessment.manifest_digest, original.digest);
            assert!(
                review
                    .files
                    .iter()
                    .all(|file| plan.copy_file_ids.contains(&file.id))
            );
            assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 2);
            let mut final_job = String::new();
            for action in [CopyAction::PrepareNames, CopyAction::CopyFiles] {
                eprintln!("Generated projected import variant {variant}: {action:?}");
                assert_eq!(review.action, action);
                let approval = workflow
                    .approve(
                        &fixture.library,
                        CopyApprovalInput {
                            review_id: review.id.clone(),
                            digest: review.digest.clone(),
                            action,
                            fidelity_limits_acknowledged: true,
                        },
                        selection.clone(),
                        fixture.destination.clone(),
                        identity.clone(),
                        volume.clone(),
                    )
                    .unwrap();
                if variant == 2 && action == CopyAction::CopyFiles {
                    let keeper = plan.skip_files[0].keeper_asset_id.clone();
                    fixture
                        .library
                        .catalog
                        .call(move |db| {
                            db.execute(
                                "UPDATE assets SET presence='inaccessible' WHERE id=?",
                                [keeper],
                            )?;
                            Ok(())
                        })
                        .unwrap();
                }
                if action == CopyAction::CopyFiles && (3..=4).contains(&variant) {
                    fixture.library.catalog.call(move|db|{db.execute_batch(if variant==3 {"CREATE TRIGGER deny_import_copy_evidence BEFORE INSERT ON operation_journal WHEN NEW.phase='import_copy_verified_v1' BEGIN SELECT RAISE(ABORT,'owned import receipt denial'); END;"} else {"CREATE TRIGGER deny_import_completion BEFORE UPDATE ON operation_jobs WHEN NEW.state='completed_verified' BEGIN SELECT RAISE(ABORT,'owned import job completion denial'); END;"})?;Ok(())}).unwrap();
                }
                let queued = workflow
                    .start(
                        fixture.library.clone(),
                        CopyStartInput {
                            review_id: review.id.clone(),
                            digest: review.digest.clone(),
                            approval_id: approval.id,
                        },
                        selection.clone(),
                        fixture.destination.clone(),
                        identity.clone(),
                        volume.clone(),
                    )
                    .unwrap();
                let finished = wait_operation(|| workflow.job(queued.id.clone()));
                assert_eq!(
                    finished.state,
                    if action == CopyAction::PrepareNames {
                        CopyJobState::Prepared
                    } else if variant == 2 || variant == 3 {
                        CopyJobState::NeedsAttention
                    } else if variant == 4 {
                        CopyJobState::NeedsRecovery
                    } else {
                        CopyJobState::CompletedVerified
                    },
                    "variant={variant}, action={action:?}, error={:?}",
                    finished.error
                );
                if action == CopyAction::PrepareNames {
                    workflow.discard(&review.id).unwrap();
                    let reopened = workflow
                        .reopen_preparation(
                            &fixture.library,
                            finished.id,
                            selection.clone(),
                            fixture.destination.clone(),
                            (identity.clone(), volume.clone()),
                        )
                        .unwrap();
                    assert_eq!(reopened.manifest.files.len(), 4);
                    assert_eq!(reopened.import_plan.as_ref().unwrap().id, plan.id);
                    review = reopened.review;
                    assert_eq!(
                        review.assessment.capacity.logical_bytes,
                        logical.to_string()
                    );
                } else {
                    final_job = finished.id.clone();
                    assert_eq!(finished.verified_files, if variant == 2 { 0 } else { 2 });
                    let receipt = workflow.import_copy_receipt(finished.id.clone()).unwrap();
                    assert_eq!(receipt.is_some(), variant < 2 || variant == 5);
                    if let Some(receipt) = receipt {
                        assert_eq!(receipt.copied_file_ids, plan.copy_file_ids);
                        assert_eq!(receipt.skipped_files.len(), 2);
                        assert_eq!(receipt.import_digest, plan.digest);
                        assert!(receipt.keeper_bytes_read.parse::<u128>().unwrap() > 0);
                    }
                    let job_id = finished.id.clone();
                    let facts:i64=fixture.library.catalog.call(move|db|Ok(db.query_row("SELECT count(*) FROM operation_journal WHERE item_id=? AND phase='import_copy_verified_v1'",[job_id],|row|row.get(0))?)).unwrap();
                    assert_eq!(
                        facts,
                        if variant < 2 || variant == 4 || variant == 5 {
                            1
                        } else {
                            0
                        }
                    );
                    for file in &review.files {
                        let output = fixture.destination.join(&file.target_path);
                        if variant == 2 {
                            assert!(!output.exists());
                        } else {
                            assert_eq!(
                                std::fs::read(output).unwrap(),
                                std::fs::read(fixture.source.join(&file.source_path)).unwrap()
                            );
                        }
                    }
                    if (3..=4).contains(&variant) {
                        fixture.library.catalog.call(|db|{db.execute_batch("DROP TRIGGER IF EXISTS deny_import_copy_evidence; DROP TRIGGER IF EXISTS deny_import_completion;")?;Ok(())}).unwrap();
                        let continuation = workflow
                            .create_continuation_review(
                                &fixture.library,
                                finished.id.clone(),
                                selection.clone(),
                                fixture.destination.clone(),
                                (identity.clone(), volume.clone()),
                            )
                            .unwrap();
                        assert_eq!(continuation.files.len(), 2);
                        assert_eq!(continuation.assessment.capacity.rounded_data_bytes, "0");
                        let approval = workflow
                            .approve_continuation(
                                &fixture.library,
                                CopyApprovalInput {
                                    review_id: continuation.id.clone(),
                                    digest: continuation.digest.clone(),
                                    action: CopyAction::CopyFiles,
                                    fidelity_limits_acknowledged: true,
                                },
                                selection.clone(),
                                fixture.destination.clone(),
                                (identity.clone(), volume.clone()),
                            )
                            .unwrap();
                        let queued = workflow
                            .start_continuation(
                                fixture.library.clone(),
                                CopyStartInput {
                                    review_id: continuation.id,
                                    digest: continuation.digest,
                                    approval_id: approval.id,
                                },
                                selection.clone(),
                                fixture.destination.clone(),
                                (identity.clone(), volume.clone()),
                            )
                            .unwrap();
                        let completed = wait_operation(|| workflow.job(queued.id.clone()));
                        assert_eq!(completed.state, CopyJobState::CompletedVerified);
                        assert_eq!(completed.review_id, finished.review_id);
                        assert_ne!(completed.id, finished.id);
                        final_job = completed.id.clone();
                        let receipt = workflow.import_copy_receipt(completed.id).unwrap().unwrap();
                        assert_eq!(receipt.import_plan_id, plan.id);
                        assert_eq!(receipt.copied_file_ids, plan.copy_file_ids);
                        assert!(receipt.keeper_bytes_read.parse::<u128>().unwrap() > 0);
                        assert!(workflow.import_copy_receipt(finished.id).unwrap().is_none());
                    }
                }
            }
            assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 3);
            assert!(fixture.preserved().unwrap());
            let retained = fixture
                .library
                .organization_copy_manifest_open(fixture.owner.clone(), fixture.manifest.clone())
                .unwrap();
            assert_eq!(
                serde_json::to_vec(&original).unwrap(),
                serde_json::to_vec(&retained).unwrap()
            );
            assert!(
                workflow
                    .create_import_review(
                        &fixture.library,
                        fixture.owner.clone(),
                        plan.id,
                        selection,
                        fixture.destination.clone(),
                        (identity.clone(), volume.clone())
                    )
                    .is_err()
            );
            assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 3);
            let root_count = crate::library::store::roots(&fixture.library.catalog)
                .unwrap()
                .len();
            if variant == 3 {
                let path = fixture.destination.join(&review.files[0].target_path);
                let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
                let mut bytes = std::fs::read(&path).unwrap();
                bytes[0] ^= 1;
                std::fs::write(&path, bytes).unwrap();
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_times(std::fs::FileTimes::new().set_modified(modified))
                    .unwrap();
            } else if variant == 4 {
                let parent = fixture
                    .destination
                    .join(&review.output_folder)
                    .canonicalize()
                    .unwrap();
                assert!(parent.starts_with(fixture._run.path().canonicalize().unwrap()));
                let path = parent.join("unknown-added.txt");
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .unwrap();
            }
            let indexed = workflow.index_import_output(
                fixture.library.clone(),
                final_job.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            );
            if variant < 2 || variant == 5 {
                let indexed = indexed.unwrap();
                assert_eq!(indexed.already_registered, variant == 5);
                assert_eq!(
                    indexed.root.mode,
                    if variant == 5 {
                        crate::library::model::RootMode::Managed
                    } else {
                        crate::library::model::RootMode::Referenced
                    }
                );
                assert!(!indexed.original_mutations_enabled);
                let retry = workflow
                    .index_import_output(
                        fixture.library.clone(),
                        final_job,
                        fixture.destination.clone(),
                        (identity, volume),
                    )
                    .unwrap();
                assert!(retry.already_registered);
                assert_eq!(retry.root.id, indexed.root.id);
                assert_eq!(
                    crate::library::store::roots(&fixture.library.catalog)
                        .unwrap()
                        .len(),
                    root_count + usize::from(variant != 5)
                );
                if variant != 5 {
                    assert_eq!(
                        retry.scan.as_ref().map(|scan| &scan.id),
                        indexed.scan.as_ref().map(|scan| &scan.id)
                    );
                } else {
                    assert_eq!(retry.root.mode, crate::library::model::RootMode::Managed);
                }
            } else {
                assert!(indexed.is_err());
                assert_eq!(
                    crate::library::store::roots(&fixture.library.catalog)
                        .unwrap()
                        .len(),
                    root_count
                );
            }
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn import_keeper_guard_holds_atomic_copy_and_keeper_scopes_without_partial_leases() {
        use crate::executor::fingerprints::Stats;
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        use std::sync::atomic::AtomicBool;
        let (fixture, workflow, plan, _, _) = mixed_import_fixture();
        let sources = workflow
            .import_copy_sources(fixture.owner.clone(), plan.id.clone())
            .unwrap();
        let root = plan.scope_root_ids[0].clone();
        let path = fixture
            .library
            .catalog
            .call(move |db| {
                crate::library::native::decode_path(
                    &crate::executor::sources::root(db, &root, false)?.0,
                )
            })
            .unwrap();
        let path = path.canonicalize().unwrap();
        let destination = fixture.destination.canonicalize().unwrap();
        let registry = Registry::shared();
        let blocker = registry
            .try_acquire(vec![Scope::new(
                crate::filesystem::DirectoryChain::open(&destination).unwrap(),
                Access::Read,
            )])
            .unwrap();
        assert!(workflow.copy_scope(&sources).is_err());
        let not_leaked = registry
            .try_acquire(vec![Scope::new(
                crate::filesystem::DirectoryChain::open(&path).unwrap(),
                Access::Write,
            )])
            .unwrap();
        drop(not_leaked);
        drop(blocker);
        let (scope, check) = workflow.copy_scope(&sources).unwrap();
        let check = check.unwrap();
        assert!(
            registry
                .try_acquire(vec![Scope::new(
                    crate::filesystem::DirectoryChain::open(&path).unwrap(),
                    Access::Write
                )])
                .is_err()
        );
        let mut stats = Stats::default();
        let mut total = 0;
        for _ in 0..2 {
            assert!(matches!(
                check
                    .verify(
                        &workflow,
                        &scope,
                        &AtomicBool::new(false),
                        &mut stats,
                        &|_, _| Ok(())
                    )
                    .unwrap(),
                crate::jobs::paced_io::IoStep::Completed(())
            ));
            total += 2 * (generated_photo_bytes().unwrap().len() + COMPANION_BYTES.len()) as u128;
            assert_eq!(stats.bytes_read, total);
        }
        assert!(fixture.preserved().unwrap());
        drop(scope);
        let released = registry
            .try_acquire(vec![Scope::new(
                crate::filesystem::DirectoryChain::open(&path).unwrap(),
                Access::Write,
            )])
            .unwrap();
        drop(released);
    }
    #[test]
    fn import_keeper_guard_refuses_late_keeper_change_and_inaccessible_index_without_output() {
        use crate::executor::fingerprints::Stats;
        use std::sync::atomic::AtomicBool;
        for inaccessible in [false, true] {
            let (fixture, workflow, plan, _, _) = mixed_import_fixture();
            let sources = workflow
                .import_copy_sources(fixture.owner.clone(), plan.id)
                .unwrap();
            let (scope, check) = workflow.copy_scope(&sources).unwrap();
            let check = check.unwrap();
            let keeper = plan
                .skip_files
                .iter()
                .find(|file| file.keeper_relative_path.ends_with(".xmp"))
                .unwrap()
                .clone();
            if inaccessible {
                fixture
                    .library
                    .catalog
                    .call(move |db| {
                        db.execute(
                            "UPDATE assets SET presence='inaccessible' WHERE id=?",
                            [keeper.keeper_asset_id],
                        )?;
                        Ok(())
                    })
                    .unwrap();
            } else {
                let root = keeper.keeper_root_id;
                let path = fixture
                    .library
                    .catalog
                    .call(move |db| {
                        crate::library::native::decode_path(
                            &crate::executor::sources::root(db, &root, false)?.0,
                        )
                    })
                    .unwrap();
                let file = path.join("Renamed.xmp");
                let modified = std::fs::metadata(&file).unwrap().modified().unwrap();
                let mut changed = COMPANION_BYTES.to_vec();
                changed[0] ^= 1;
                std::fs::write(&file, &changed).unwrap();
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&file)
                    .unwrap()
                    .set_times(std::fs::FileTimes::new().set_modified(modified))
                    .unwrap();
            }
            assert!(
                check
                    .verify(
                        &workflow,
                        &scope,
                        &AtomicBool::new(false),
                        &mut Stats::default(),
                        &|_, _| Ok(())
                    )
                    .is_err()
            );
            assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 2);
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn import_keeper_guard_cancel_reads_no_payload_and_creates_no_namespace() {
        use crate::executor::fingerprints::Stats;
        use std::sync::atomic::AtomicBool;
        let (fixture, workflow, plan, _, _) = mixed_import_fixture();
        let sources = workflow
            .import_copy_sources(fixture.owner.clone(), plan.id)
            .unwrap();
        let (scope, check) = workflow.copy_scope(&sources).unwrap();
        let mut stats = Stats::default();
        assert!(matches!(
            check
                .unwrap()
                .verify(
                    &workflow,
                    &scope,
                    &AtomicBool::new(true),
                    &mut stats,
                    &|_, _| Ok(())
                )
                .unwrap(),
            crate::jobs::paced_io::IoStep::Canceled
        ));
        assert_eq!(stats.bytes_read, 0);
        assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 2);
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn import_comparison_bounds_reports_matches_and_candidate_inventory() {
        use crate::operations::{ImportComparisonInput, ImportComparisonState};
        let fixture = Fixture::with_nested(true).unwrap();
        let bytes = generated_photo_bytes().unwrap();
        let files = (0..5)
            .map(|index| (format!("Renamed-{index}.JPG"), bytes.clone()))
            .collect::<Vec<_>>();
        let borrowed = files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.clone()))
            .collect::<Vec<_>>();
        let (root, _, _) = comparison_root(&fixture, &borrowed);
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let input = ImportComparisonInput {
            source_root_id: None,
            plan_id: fixture.owner.clone(),
            manifest_id: fixture.manifest.clone(),
            scope_root_ids: vec![root.clone()],
        };
        let too_many = crate::operations::ImportComparisonInput {
            source_root_id: None,
            scope_root_ids: (0..17).map(|_| uuid::Uuid::new_v4().to_string()).collect(),
            ..input.clone()
        };
        assert!(
            workflow
                .start_import_comparison(&fixture.library, too_many)
                .is_err()
        );
        let duplicated = crate::operations::ImportComparisonInput {
            source_root_id: None,
            scope_root_ids: vec![root.clone(), root],
            ..input.clone()
        };
        assert!(
            workflow
                .start_import_comparison(&fixture.library, duplicated)
                .is_err()
        );
        let mut retained = Vec::new();
        for _ in 0..8 {
            let queued = workflow
                .start_import_comparison(&fixture.library, input.clone())
                .unwrap();
            let completed = wait_comparison(&workflow, queued.id.clone());
            assert_eq!(completed.state, ImportComparisonState::Completed);
            let report = completed.report.unwrap();
            let matching = report
                .files
                .iter()
                .find(|file| !file.matches.is_empty())
                .unwrap();
            assert_eq!(matching.matches.len(), 4);
            assert!(matching.further_candidates_uncompared);
            retained.push(queued.id);
        }
        assert!(
            workflow
                .start_import_comparison(&fixture.library, input.clone())
                .is_err()
        );
        workflow
            .discard_import_comparison(retained.remove(0))
            .unwrap();
        let queued = workflow
            .start_import_comparison(&fixture.library, input)
            .unwrap();
        assert_eq!(
            wait_comparison(&workflow, queued.id).state,
            ImportComparisonState::Completed
        );
        // Corrupt/overlarge indexed scopes are rejected before unbounded native
        // traversal; no missing files are manufactured as successful evidence.
        let (oversized, _, _) = comparison_root(&fixture, &[]);
        let added = oversized.clone();
        fixture.library.catalog.call(move|db|{let tx=db.transaction()?;for index in 0..4097 {let name=format!("bounded-{index}.JPG");tx.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,?2,?3,?4,?5,'',?4,?4,'jpg','1:1',1,?6,'now','2026-10-31T00:00:00','filesystem_modified','ready','pending','present',1,'bounded-fixture')",params![uuid::Uuid::new_v4().to_string(),added,crate::library::native::encode_path(Path::new(&name)),name,crate::library::native::encode_path(Path::new("")),"0".repeat(64)])?;}tx.commit()?;Ok(())}).unwrap();
        let fresh = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let queued = fresh
            .start_import_comparison(
                &fixture.library,
                ImportComparisonInput {
                    source_root_id: None,
                    plan_id: fixture.owner.clone(),
                    manifest_id: fixture.manifest.clone(),
                    scope_root_ids: vec![oversized],
                },
            )
            .unwrap();
        let failed = wait_comparison(&fresh, queued.id);
        assert_eq!(failed.state, ImportComparisonState::Failed);
        assert!(failed.report.is_none());
        assert!(failed.error.unwrap().message.contains("4,096"));
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn import_comparison_root_cancellation_releases_admission_and_discarded_reports() {
        use crate::operations::{ImportComparisonInput, ImportComparisonState};
        let mut bytes = generated_photo_bytes().unwrap();
        bytes.extend(vec![81; 2 * 1024 * 1024]);
        let fixture = Fixture::with_media(true, bytes.clone()).unwrap();
        let (root, path, _) = comparison_root(&fixture, &[("Large renamed.JPG", bytes.clone())]);
        let workflow = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let queued = workflow
            .start_import_comparison(
                &fixture.library,
                ImportComparisonInput {
                    source_root_id: None,
                    plan_id: fixture.owner.clone(),
                    manifest_id: fixture.manifest.clone(),
                    scope_root_ids: vec![root.clone()],
                },
            )
            .unwrap();
        assert!(workflow.cancel_root(&root).unwrap());
        let stopped = wait_comparison(&workflow, queued.id.clone());
        assert_eq!(stopped.state, ImportComparisonState::Canceled);
        assert!(stopped.report.is_none());
        assert!(!workflow.has_active_root(&root).unwrap());
        workflow
            .discard_import_comparison(queued.id.clone())
            .unwrap();
        assert!(workflow.import_comparison_job(queued.id).is_err());
        assert!(fixture.preserved().unwrap());
        assert_eq!(
            std::fs::read(path.join("Large renamed.JPG")).unwrap(),
            bytes
        );
    }
    fn equivalent_manifest(
        fixture: &Fixture,
        changed_template: bool,
    ) -> crate::operations::CopyManifest {
        use crate::operations::{
            BundleConfirmationInput, BundleRelation, CopyManifestInput, PlanDraftInput,
        };
        let owner = fixture.owner.clone();
        let draft = fixture
            .library
            .catalog
            .call(move |db| crate::library::operation_store::load(db, &owner))
            .unwrap();
        let ids = draft
            .sources
            .iter()
            .map(|source| source.id.clone())
            .collect::<Vec<_>>();
        let source = &draft.sources[0];
        let mut template = draft.view.preview.template;
        if changed_template {
            template.pattern = "changed/{original_stem}.{ext}".into();
        }
        let preview = fixture
            .library
            .organization_preview(ids.clone(), template.clone())
            .unwrap();
        let target = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&target).unwrap();
        let volume = crate::library::native::volume_token(&target).unwrap();
        drop(target);
        let next = fixture
            .library
            .organization_plan_create(
                PlanDraftInput {
                    destination_root_id: None,
                    asset_ids: ids,
                    template,
                    preview_fingerprint: preview.fingerprint,
                    template_reference: None,
                },
                fixture.destination.clone(),
                (identity, volume),
            )
            .unwrap();
        let inventory = fixture
            .library
            .organization_bundle_inventory(source.id.clone(), source.revision.clone())
            .unwrap();
        let confirmation = fixture
            .library
            .organization_bundle_confirm(BundleConfirmationInput {
                plan_id: next.plan.id.clone(),
                asset_id: source.id.clone(),
                source_revision: source.revision.clone(),
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
        fixture
            .library
            .organization_copy_manifest(CopyManifestInput {
                plan_id: next.plan.id,
                bundle_confirmation_ids: vec![confirmation.id],
            })
            .unwrap()
    }
    #[test]
    fn preparation_reopen_retains_incomplete_changed_and_unknown_objects_without_binding() {
        use crate::operations::{CopyAction, CopyApprovalInput, CopyJobState, CopyStartInput};
        for variant in 0..3 {
            let fixture = Fixture::with_nested(true).unwrap();
            let selected = crate::library::native::open_root(&fixture.destination, None).unwrap();
            let identity = crate::filesystem::identity(&selected).unwrap();
            let volume = crate::library::native::volume_token(&selected).unwrap();
            drop(selected);
            let selection = uuid::Uuid::new_v4().to_string();
            let review = fixture
                .library
                .organization_copy_review_create(
                    fixture.owner.clone(),
                    fixture.manifest.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            if variant == 0 {
                fixture.library.catalog.call(|db|{db.execute_batch("CREATE TRIGGER deny_namespace_created BEFORE INSERT ON operation_journal WHEN NEW.phase='namespace_preparation_fact_v2' AND json_extract(NEW.evidence,'$.phase')='namespace_created' BEGIN SELECT RAISE(ABORT,'owned namespace creation record denial'); END;")?;Ok(())}).unwrap();
            }
            let approval = fixture
                .library
                .organization_copy_approve(
                    CopyApprovalInput {
                        review_id: review.id.clone(),
                        digest: review.digest.clone(),
                        action: CopyAction::PrepareNames,
                        fidelity_limits_acknowledged: false,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            let queued = fixture
                .library
                .organization_copy_start(
                    CopyStartInput {
                        review_id: review.id,
                        digest: review.digest,
                        approval_id: approval.id,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            let stopped =
                wait_operation(|| fixture.library.organization_copy_job(queued.id.clone()));
            assert_eq!(
                stopped.state,
                if variant == 0 {
                    CopyJobState::NeedsAttention
                } else {
                    CopyJobState::Prepared
                }
            );
            let root = fixture.destination.join(review.output_folder);
            let handle = crate::library::native::open_root(&root, None).unwrap();
            let original = Object::of(&handle).unwrap();
            drop(handle);
            let retained = if variant == 1 {
                let path = std::fs::read_dir(root.join("organized"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                std::fs::write(&path, b"changed stage").unwrap();
                Some((path, b"changed stage".to_vec()))
            } else if variant == 2 {
                let path = root.join("unknown-preserved.txt");
                std::fs::write(&path, b"unknown data").unwrap();
                Some((path, b"unknown data".to_vec()))
            } else {
                None
            };
            let before: i64 = fixture
                .library
                .catalog
                .call(|db| {
                    Ok(db.query_row("SELECT count(*) FROM operation_journal", [], |r| r.get(0))?)
                })
                .unwrap();
            assert!(
                fixture
                    .library
                    .organization_copy_preparation_reopen(
                        stopped.id,
                        selection,
                        fixture.destination.clone(),
                        (identity, volume)
                    )
                    .is_err()
            );
            let after: i64 = fixture
                .library
                .catalog
                .call(|db| {
                    Ok(db.query_row("SELECT count(*) FROM operation_journal", [], |r| r.get(0))?)
                })
                .unwrap();
            assert_eq!(before, after);
            let handle = crate::library::native::open_root(&root, None).unwrap();
            assert!(original.matches(&handle).unwrap());
            if let Some((path, bytes)) = retained {
                assert_eq!(std::fs::read(path).unwrap(), bytes);
            } else {
                assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
            }
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn reopen_prepared_application_job_reconstructs_only_metadata_and_requires_new_approval() {
        use crate::operations::{CopyAction, CopyApprovalInput, CopyJobState, CopyStartInput};
        for deny_binding in [false, true] {
            let fixture = Fixture::with_nested(true).unwrap();
            let selected = crate::library::native::open_root(&fixture.destination, None).unwrap();
            let identity = crate::filesystem::identity(&selected).unwrap();
            let volume = crate::library::native::volume_token(&selected).unwrap();
            drop(selected);
            let selection = uuid::Uuid::new_v4().to_string();
            let review = fixture
                .library
                .organization_copy_review_create(
                    fixture.owner.clone(),
                    fixture.manifest.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            if deny_binding {
                fixture.library.catalog.call(|db|{db.execute_batch("CREATE TRIGGER deny_ready_binding BEFORE INSERT ON operation_journal WHEN NEW.phase='copy_execution_plan_v1' BEGIN SELECT RAISE(ABORT,'owned ready binding denial'); END;")?;Ok(())}).unwrap();
            }
            let approved = fixture
                .library
                .organization_copy_approve(
                    CopyApprovalInput {
                        review_id: review.id.clone(),
                        digest: review.digest.clone(),
                        action: CopyAction::PrepareNames,
                        fidelity_limits_acknowledged: false,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            let queued = fixture
                .library
                .organization_copy_start(
                    CopyStartInput {
                        review_id: review.id.clone(),
                        digest: review.digest,
                        approval_id: approved.id,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            let stopped =
                wait_operation(|| fixture.library.organization_copy_job(queued.id.clone()));
            assert_eq!(
                stopped.state,
                if deny_binding {
                    CopyJobState::NeedsAttention
                } else {
                    CopyJobState::Prepared
                }
            );
            assert_eq!(stopped.ready_review_id.is_none(), deny_binding);
            let directory = fixture
                .destination
                .join(&review.output_folder)
                .join("organized");
            let inventory = || {
                std::fs::read_dir(&directory)
                    .unwrap()
                    .map(|entry| {
                        let path = entry.unwrap().path();
                        let file = std::fs::File::open(&path).unwrap();
                        (
                            path.file_name().unwrap().to_os_string(),
                            {
                                let object = Object::of(&file).unwrap();
                                (object.volume, object.file)
                            },
                            super::super::source_native::revision(&file).unwrap(),
                            file.metadata().unwrap().len(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let mut before = inventory();
            before.sort_by(|a, b| a.0.cmp(&b.0));
            assert_eq!(before.len(), 2);
            assert!(before.iter().all(|entry| entry.3 == 0));
            fixture
                .library
                .organization_copy_discard(review.id)
                .unwrap();
            let fresh = crate::executor::CopyWorkflow::new(
                fixture.library.catalog.clone(),
                fixture._run.path().join("data"),
            )
            .unwrap();
            if deny_binding {
                assert!(
                    fresh
                        .reopen_preparation(
                            &fixture.library,
                            stopped.id.clone(),
                            selection.clone(),
                            fixture.destination.clone(),
                            (identity.clone(), volume.clone())
                        )
                        .is_err()
                );
                fixture
                    .library
                    .catalog
                    .call(|db| {
                        db.execute_batch("DROP TRIGGER deny_ready_binding")?;
                        Ok(())
                    })
                    .unwrap();
            }
            let reopened = fresh
                .reopen_preparation(
                    &fixture.library,
                    stopped.id.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            assert_eq!(reopened.job_id, stopped.id);
            assert_eq!(reopened.manifest.id, fixture.manifest);
            assert_eq!(reopened.review.action, CopyAction::CopyFiles);
            let mut after = inventory();
            after.sort_by(|a, b| a.0.cmp(&b.0));
            assert_eq!(before, after);
            let stale = fresh
                .approve(
                    &fixture.library,
                    CopyApprovalInput {
                        review_id: reopened.review.id.clone(),
                        digest: reopened.review.digest.clone(),
                        action: CopyAction::CopyFiles,
                        fidelity_limits_acknowledged: true,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    identity.clone(),
                    volume.clone(),
                )
                .unwrap();
            // Reopening the same exact scope/selection must still revoke old grants.
            let reopened = fresh
                .reopen_preparation(
                    &fixture.library,
                    stopped.id,
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            assert!(
                fresh
                    .start(
                        fixture.library.clone(),
                        CopyStartInput {
                            review_id: reopened.review.id.clone(),
                            digest: reopened.review.digest.clone(),
                            approval_id: stale.id
                        },
                        selection.clone(),
                        fixture.destination.clone(),
                        identity.clone(),
                        volume.clone()
                    )
                    .is_err()
            );
            let approval = fresh
                .approve(
                    &fixture.library,
                    CopyApprovalInput {
                        review_id: reopened.review.id.clone(),
                        digest: reopened.review.digest.clone(),
                        action: CopyAction::CopyFiles,
                        fidelity_limits_acknowledged: true,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    identity.clone(),
                    volume.clone(),
                )
                .unwrap();
            let queued = fresh
                .start(
                    fixture.library.clone(),
                    CopyStartInput {
                        review_id: reopened.review.id,
                        digest: reopened.review.digest,
                        approval_id: approval.id,
                    },
                    selection,
                    fixture.destination.clone(),
                    identity,
                    volume,
                )
                .unwrap();
            let copied = wait_operation(|| fresh.job(queued.id.clone()));
            assert_eq!(
                copied.state,
                CopyJobState::CompletedVerified,
                "{:?}",
                copied.error
            );
            let final_files = inventory();
            let before_ids = before.iter().map(|entry| entry.1).collect::<Vec<_>>();
            assert!(
                final_files
                    .iter()
                    .all(|entry| before_ids.contains(&entry.1))
            );
            assert_eq!(final_files.len(), 2);
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn effect_binding_reuses_unstarted_review_and_blocks_cross_draft_duplicates() {
        let fixture = Fixture::with_nested(true).unwrap();
        let selected = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&selected).unwrap();
        let volume = crate::library::native::volume_token(&selected).unwrap();
        drop(selected);
        let selection = uuid::Uuid::new_v4().to_string();
        let review = fixture
            .library
            .organization_copy_review_create(
                fixture.owner.clone(),
                fixture.manifest.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let again = fixture
            .library
            .organization_copy_review_create(
                fixture.owner.clone(),
                fixture.manifest.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert_eq!(again.id, review.id);
        assert_eq!(again.digest, review.digest);
        fixture
            .library
            .catalog
            .call(|db| {
                db.execute("UPDATE library_state SET revision=revision+1", [])?;
                Ok(())
            })
            .unwrap();
        let next = equivalent_manifest(&fixture, false);
        assert_ne!(next.plan_id, fixture.owner);
        let matched = fixture
            .library
            .organization_copy_effect_match(
                next.plan_id.clone(),
                next.id.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert_eq!(matched.existing.len(), 1);
        assert_eq!(matched.existing[0].review_id, review.id);
        assert!(!matched.can_reuse_review);
        assert!(
            fixture
                .library
                .organization_copy_review_create(
                    next.plan_id,
                    next.id,
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err()
        );
        assert!(!fixture.destination.join(&review.output_folder).exists());
        let changed = equivalent_manifest(&fixture, true);
        let different = fixture
            .library
            .organization_copy_effect_match(
                changed.plan_id,
                changed.id,
                selection,
                fixture.destination.clone(),
                (identity, volume),
            )
            .unwrap();
        assert!(different.existing.is_empty());
        assert!(different.can_reuse_review);
        fixture.library.catalog.call(|db|{let count:u32=db.query_row("SELECT count(*) FROM operation_journal WHERE phase='namespace_preparation_plan_v2'",[],|r|r.get(0))?;assert_eq!(count,1);Ok(())}).unwrap();
    }
    #[test]
    fn concurrent_equivalent_drafts_reserve_only_one_namespace() {
        let fixture = Fixture::with_nested(true).unwrap();
        let next = equivalent_manifest(&fixture, false);
        let selected = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&selected).unwrap();
        let volume = crate::library::native::volume_token(&selected).unwrap();
        drop(selected);
        let barrier = std::sync::Barrier::new(2);
        let results = std::thread::scope(|scope| {
            let handles = [
                (fixture.owner.clone(), fixture.manifest.clone()),
                (next.plan_id, next.id),
            ]
            .into_iter()
            .map(|(owner, manifest)| {
                let barrier = &barrier;
                let library = &fixture.library;
                let destination = &fixture.destination;
                let identity = &identity;
                let volume = &volume;
                scope.spawn(move || {
                    barrier.wait();
                    library.organization_copy_review_create(
                        owner,
                        manifest,
                        uuid::Uuid::new_v4().to_string(),
                        destination.clone(),
                        (identity.clone(), volume.clone()),
                    )
                })
            })
            .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        let winner = results.into_iter().find_map(|result| result.ok()).unwrap();
        assert!(!fixture.destination.join(winner.output_folder).exists());
        fixture
            .library
            .catalog
            .call(|db| {
                for phase in [
                    "namespace_preparation_plan_v2",
                    "copy_effect_group_v1",
                    "copy_effect_candidate_v1",
                ] {
                    let count: u32 = db.query_row(
                        "SELECT count(*) FROM operation_journal WHERE phase=?",
                        [phase],
                        |r| r.get(0),
                    )?;
                    assert_eq!(count, 1);
                }
                Ok(())
            })
            .unwrap();
    }
    #[test]
    fn effect_candidate_denial_rolls_back_namespace_and_retry_uses_no_filesystem_effects() {
        let fixture = Fixture::with_nested(true).unwrap();
        let selected = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&selected).unwrap();
        let volume = crate::library::native::volume_token(&selected).unwrap();
        drop(selected);
        fixture.library.catalog.call(|db|{db.execute_batch("CREATE TRIGGER deny_effect BEFORE INSERT ON operation_journal WHEN NEW.phase='copy_effect_candidate_v1' BEGIN SELECT RAISE(ABORT,'fixture effect denial'); END;")?;Ok(())}).unwrap();
        let selection = uuid::Uuid::new_v4().to_string();
        assert!(
            fixture
                .library
                .organization_copy_review_create(
                    fixture.owner.clone(),
                    fixture.manifest.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err()
        );
        fixture.library.catalog.call(|db|{let count:u32=db.query_row("SELECT count(*) FROM operation_journal WHERE phase IN ('namespace_preparation_plan_v2','copy_effect_group_v1','copy_effect_candidate_v1')",[],|r|r.get(0))?;assert_eq!(count,0);db.execute_batch("DROP TRIGGER deny_effect")?;Ok(())}).unwrap();
        let review = fixture
            .library
            .organization_copy_review_create(
                fixture.owner.clone(),
                fixture.manifest.clone(),
                selection,
                fixture.destination.clone(),
                (identity, volume),
            )
            .unwrap();
        assert!(!fixture.destination.join(&review.output_folder).exists());
    }
    #[test]
    fn effect_lookup_indexes_legacy_organized_namespace_without_execution_authority() {
        let fixture = Fixture::with_nested(true).unwrap();
        let snapshot = super::super::sources::Snapshot::build(
            &fixture.library.catalog,
            fixture.owner.clone(),
            fixture.manifest.clone(),
        )
        .unwrap();
        let token = uuid::Uuid::new_v4().to_string();
        let legacy = namespace_journal::NamespaceJournal::bind_snapshot(
            fixture.library.catalog.clone(),
            snapshot.clone(),
            token.clone(),
            true,
        )
        .unwrap();
        let (effect, existing) =
            super::super::effects::Effect::lookup(&fixture.library.catalog, &snapshot).unwrap();
        assert_eq!(existing.len(), 1);
        assert_eq!(existing[0].token, token);
        let (again, existing) =
            super::super::effects::Effect::lookup(&fixture.library.catalog, &snapshot).unwrap();
        assert_eq!(again.digest, effect.digest);
        assert_eq!(existing.len(), 1);
        assert!(!fixture.destination.join(legacy.namespace()).exists());
    }

    use super::*;
    pub(super) fn wait_operation(
        load: impl Fn() -> Result<crate::operations::CopyWorkflowJob>,
    ) -> crate::operations::CopyWorkflowJob {
        use crate::operations::CopyJobState;
        // Native flush/lookup latency under the full CI fault matrix is not a
        // ten-second product guarantee. Keep a bounded wait and state evidence;
        // the isolated scenario retains its independent process deadline.
        let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let current = load().unwrap();
            if !matches!(
                current.state,
                CopyJobState::Queued
                    | CopyJobState::Running
                    | CopyJobState::PauseRequested
                    | CopyJobState::CancelRequested
            ) {
                return current;
            }
            assert!(
                std::time::Instant::now() < until,
                "Operation worker did not stop: {:?}, {}/{} files verified",
                current.state,
                current.verified_files,
                current.total_files
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[test]
    fn application_continuation_reopens_failed_copy_without_cached_sessions_and_preserves_stage_objects()
     {
        use crate::operations::{CopyAction, CopyApprovalInput, CopyJobState, CopyStartInput};
        let fixture = Fixture::with_nested(true).unwrap();
        let chosen = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = filesystem::identity(&chosen).unwrap();
        let volume = crate::library::native::volume_token(&chosen).unwrap();
        drop(chosen);
        let selection = uuid::Uuid::new_v4().to_string();
        let mut view = fixture
            .library
            .organization_copy_review_create(
                fixture.owner.clone(),
                fixture.manifest.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let mut failed = None;
        for action in [CopyAction::PrepareNames, CopyAction::CopyFiles] {
            if action == CopyAction::CopyFiles {
                fixture.library.catalog.call(|db|{db.execute_batch("CREATE TRIGGER deny_application_flush BEFORE INSERT ON operation_journal WHEN NEW.phase='copy_execution_event_v1' AND json_extract(NEW.evidence,'$.phase')='stage_flushed' BEGIN SELECT RAISE(ABORT,'fixture flush record denied'); END;")?;Ok(())}).unwrap();
            }
            let approved = fixture
                .library
                .organization_copy_approve(
                    CopyApprovalInput {
                        review_id: view.id.clone(),
                        digest: view.digest.clone(),
                        action,
                        fidelity_limits_acknowledged: true,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            let job = fixture
                .library
                .organization_copy_start(
                    CopyStartInput {
                        review_id: view.id.clone(),
                        digest: view.digest.clone(),
                        approval_id: approved.id,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            let stopped = wait_operation(|| fixture.library.organization_copy_job(job.id.clone()));
            if action == CopyAction::PrepareNames {
                assert_eq!(stopped.state, CopyJobState::Prepared);
                view = fixture
                    .library
                    .organization_copy_review(
                        view.id,
                        selection.clone(),
                        fixture.destination.clone(),
                        (identity.clone(), volume.clone()),
                    )
                    .unwrap();
            } else {
                assert_eq!(stopped.state, CopyJobState::NeedsAttention);
                assert_eq!(stopped.verified_files, 0);
                failed = Some(stopped);
            }
        }
        let failed = failed.unwrap();
        fixture
            .library
            .catalog
            .call(|db| {
                db.execute_batch("DROP TRIGGER deny_application_flush")?;
                Ok(())
            })
            .unwrap();
        let (before, _, _) = CatalogJournal::open_inspection(
            fixture.library.catalog.clone(),
            fixture.owner.clone(),
            failed.review_id.clone(),
        )
        .unwrap();
        assert_eq!(before.history_count().unwrap(), 2);
        let original_objects = before
            .plan()
            .files
            .iter()
            .map(|file| file.prepared_stage.as_ref().unwrap().object.clone())
            .collect::<Vec<_>>();
        drop(before);
        let fresh = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let continuation = fresh
            .create_continuation_review(
                &fixture.library,
                failed.id.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert!(!continuation.recorded_complete);
        assert_eq!(
            continuation.files[0].retained_stage_bytes,
            Some(fixture.baseline[0].1.len().to_string())
        );
        let approved = fresh
            .approve_continuation(
                &fixture.library,
                CopyApprovalInput {
                    review_id: continuation.id.clone(),
                    digest: continuation.digest.clone(),
                    action: CopyAction::CopyFiles,
                    fidelity_limits_acknowledged: true,
                },
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let queued = fresh
            .start_continuation(
                fixture.library.clone(),
                CopyStartInput {
                    review_id: continuation.id,
                    digest: continuation.digest,
                    approval_id: approved.id,
                },
                selection,
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let completed = wait_operation(|| fresh.job(queued.id.clone()));
        assert_eq!(
            completed.state,
            CopyJobState::CompletedVerified,
            "{:?}",
            completed.error
        );
        assert_eq!(completed.verified_files, 2);
        let inspection = fresh
            .inspect_retained(
                completed.id.clone(),
                fixture.destination.clone(),
                identity,
                volume,
            )
            .unwrap();
        assert!(inspection.inspection.all_published_bytes_verified);
        let (after, _, _) = CatalogJournal::open_inspection(
            fixture.library.catalog.clone(),
            fixture.owner.clone(),
            completed.review_id,
        )
        .unwrap();
        assert_eq!(after.history_count().unwrap(), 13);
        let output =
            super::super::output_chain(&fixture.destination.join(inspection.output_folder))
                .unwrap();
        for (file, expected) in after.plan().files.iter().zip(original_objects) {
            let parent = super::super::destination_parent(output.leaf(), file).unwrap();
            assert!(
                expected
                    .matches(&super::super::native::read(&parent, &file.target).unwrap())
                    .unwrap()
            );
        }
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn reviewed_continuation_reconciles_every_retained_phase_without_duplicate_streams() {
        use crate::executor::{CopyFault, CopyOutcome};
        for fault in [
            CopyFault::AfterIntent,
            CopyFault::AfterStageCreation,
            CopyFault::AfterCopyChunk,
            CopyFault::AfterFlush,
            CopyFault::AfterVerification,
            CopyFault::BeforePublish,
            CopyFault::AfterPublish,
            CopyFault::AfterItemCommit,
            CopyFault::AfterPlanCommit,
        ] {
            let fixture = Fixture::with_nested(true).unwrap();
            let journal = fixture.journal().unwrap();
            assert_eq!(
                fixture.execute(&journal, Some(fault)).unwrap(),
                CopyOutcome::Interrupted
            );
            drop(journal);
            let reopened = fixture.journal().unwrap();
            assert_eq!(
                fixture.resume(&reopened, None).unwrap(),
                CopyOutcome::CompletedVerified,
                "{fault:?}"
            );
            assert_eq!(reopened.history_count().unwrap(), 13);
            assert_eq!(
                fixture.inspect(&reopened).unwrap(),
                vec![super::super::RecoveredItem::PublishedVerified; 2]
            );
            assert!(fixture.preserved().unwrap());
            let count = reopened.history_count().unwrap();
            assert_eq!(
                fixture.resume(&reopened, None).unwrap(),
                CopyOutcome::CompletedVerified
            );
            assert_eq!(
                reopened.history_count().unwrap(),
                count,
                "completed continuation must be a freshly verified no-op"
            );
        }
    }
    #[test]
    fn reviewed_partial_stage_continuation_verifies_every_prefix_byte_before_append() {
        for corrupt in [false, true] {
            let mut photo = generated_photo_bytes().unwrap();
            photo.resize(crate::jobs::paced_io::CHUNK_BYTES * 3 + 117, 0x5a);
            let fixture = Fixture::with_media(false, photo).unwrap();
            let journal = fixture.journal().unwrap();
            fixture
                .execute(&journal, Some(super::super::CopyFault::AfterCopyChunk))
                .unwrap();
            let stage = fixture.destination.join(&fixture.plan.files[0].stage);
            let mut prefix =
                fixture.baseline[0].1[..crate::jobs::paced_io::CHUNK_BYTES * 2 + 17].to_vec();
            if corrupt {
                prefix[crate::jobs::paced_io::CHUNK_BYTES + 7] ^= 1;
            }
            std::fs::write(&stage, prefix).unwrap();
            let before = std::fs::read(&stage).unwrap();
            let count = journal.history_count().unwrap();
            let result = fixture.resume(&journal, None);
            if corrupt {
                assert!(result.is_err());
                assert_eq!(std::fs::read(&stage).unwrap(), before);
                assert_eq!(journal.history_count().unwrap(), count);
                assert!(
                    !fixture
                        .destination
                        .join(&fixture.plan.files[0].target)
                        .exists()
                );
            } else {
                assert_eq!(
                    result.unwrap(),
                    super::super::CopyOutcome::CompletedVerified
                );
                assert_eq!(
                    std::fs::read(fixture.destination.join(&fixture.plan.files[0].target)).unwrap(),
                    fixture.baseline[0].1
                );
            }
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn reviewed_continuation_can_itself_stop_at_each_boundary_and_continue_again() {
        use crate::executor::{CopyFault, CopyOutcome};
        for fault in [
            CopyFault::AfterIntent,
            CopyFault::AfterStageCreation,
            CopyFault::AfterCopyChunk,
            CopyFault::AfterFlush,
            CopyFault::AfterVerification,
            CopyFault::BeforePublish,
            CopyFault::AfterPublish,
            CopyFault::AfterItemCommit,
            CopyFault::AfterPlanCommit,
        ] {
            let mut photo = generated_photo_bytes().unwrap();
            photo.resize(crate::jobs::paced_io::CHUNK_BYTES * 3 + 117, 0x5a);
            let fixture = Fixture::with_media(false, photo).unwrap();
            let journal = fixture.journal().unwrap();
            assert_eq!(
                fixture.resume(&journal, Some(fault)).unwrap(),
                CopyOutcome::Interrupted,
                "{fault:?}"
            );
            assert!(fixture.preserved().unwrap());
            drop(journal);
            let reopened = fixture.journal().unwrap();
            assert_eq!(
                fixture.resume(&reopened, None).unwrap(),
                CopyOutcome::CompletedVerified,
                "{fault:?}"
            );
            assert_eq!(reopened.history_count().unwrap(), 13);
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn reviewed_continuation_rejects_stale_observations_and_other_action_approvals_before_append() {
        use crate::executor::{CopyFault, CopyInputs};
        for stale in [false, true] {
            let fixture = Fixture::new().unwrap();
            let journal = fixture.journal().unwrap();
            fixture
                .execute(&journal, Some(CopyFault::AfterCopyChunk))
                .unwrap();
            let (source, output) = fixture.scope().unwrap();
            let cancel = std::sync::atomic::AtomicBool::new(false);
            let io = crate::jobs::paced_io::PacedIo::shared();
            let validate = || fixture.scope().map(|_| ());
            let inputs = CopyInputs {
                source_root: source.leaf(),
                destination: output.leaf(),
                io: &io,
                cancel: &cancel,
                validate_scope: &validate,
            };
            let review = journal.recovery_review(&inputs).unwrap();
            let selection = uuid::Uuid::new_v4().to_string();
            let expected = if stale {
                review.approval(&fixture.plan, &selection).unwrap()
            } else {
                journal.copy_review(&selection).unwrap()
            };
            let approvals = crate::executor::approval::Approvals::new();
            let ticket = approvals.issue(expected.clone()).unwrap();
            let proof = approvals.consume(&ticket, &expected).unwrap();
            let stage = fixture.destination.join(&fixture.plan.files[0].stage);
            if stale {
                std::fs::write(&stage, b"changed retained bytes").unwrap();
            }
            let before = std::fs::read(&stage).unwrap();
            let count = journal.history_count().unwrap();
            assert!(
                journal
                    .resume_approved(inputs, &selection, review, proof, None, None)
                    .is_err()
            );
            assert_eq!(std::fs::read(stage).unwrap(), before);
            assert_eq!(journal.history_count().unwrap(), count);
            assert!(approvals.consume(&ticket, &expected).is_err());
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn reviewed_continuation_rechecks_managed_grants_before_publication_and_retains_journal_failures()
     {
        use crate::executor::{CopyFault, CopyInputs};
        for at_admission in [false, true] {
            let fixture = Fixture::new().unwrap();
            let journal = fixture.journal().unwrap();
            fixture
                .execute(&journal, Some(CopyFault::AfterVerification))
                .unwrap();
            let (source, output) = fixture.scope().unwrap();
            let cancel = std::sync::atomic::AtomicBool::new(false);
            let io = crate::jobs::paced_io::PacedIo::shared();
            let armed = std::sync::atomic::AtomicBool::new(false);
            let calls = std::sync::atomic::AtomicUsize::new(0);
            let validate = || {
                if armed.load(std::sync::atomic::Ordering::Acquire)
                    && calls.fetch_add(1, std::sync::atomic::Ordering::AcqRel) == 2
                {
                    fixture.library.catalog.call(|db| {
                        db.execute("UPDATE roots SET mode='referenced'", [])?;
                        Ok(())
                    })?;
                }
                fixture.scope().map(|_| ())
            };
            let inputs = CopyInputs {
                source_root: source.leaf(),
                destination: output.leaf(),
                io: &io,
                cancel: &cancel,
                validate_scope: &validate,
            };
            let review = journal.recovery_review(&inputs).unwrap();
            let selection = uuid::Uuid::new_v4().to_string();
            let expected = review.approval(&fixture.plan, &selection).unwrap();
            let approvals = crate::executor::approval::Approvals::new();
            let ticket = approvals.issue(expected.clone()).unwrap();
            let proof = approvals.consume(&ticket, &expected).unwrap();
            if at_admission {
                fixture
                    .library
                    .catalog
                    .call(|db| {
                        db.execute("UPDATE roots SET mode='referenced'", [])?;
                        Ok(())
                    })
                    .unwrap();
            } else {
                armed.store(true, std::sync::atomic::Ordering::Release);
            }
            assert!(
                journal
                    .resume_approved(inputs, &selection, review, proof, None, None)
                    .is_err()
            );
            assert_eq!(journal.history_count().unwrap(), 4);
            assert!(
                fixture
                    .destination
                    .join(&fixture.plan.files[0].stage)
                    .exists()
            );
            assert!(
                !fixture
                    .destination
                    .join(&fixture.plan.files[0].target)
                    .exists()
            );
            assert!(fixture.preserved().unwrap());
        }
        let fixture = Fixture::new().unwrap();
        let journal = fixture.journal().unwrap();
        fixture
            .execute(&journal, Some(CopyFault::AfterVerification))
            .unwrap();
        fixture.library.catalog.call(|db| {
            db.execute_batch("CREATE TRIGGER deny_resume_completion BEFORE INSERT ON operation_journal WHEN NEW.phase='copy_execution_event_v1' AND json_extract(NEW.evidence,'$.phase')='item_completed_verified' BEGIN SELECT RAISE(ABORT,'fixture journal completion denied'); END;")?;Ok(())
        }).unwrap();
        assert!(fixture.resume(&journal, None).is_err());
        assert_eq!(journal.history_count().unwrap(), 5);
        assert!(
            fixture
                .destination
                .join(&fixture.plan.files[0].target)
                .exists()
        );
        assert!(fixture.preserved().unwrap());
        assert_eq!(
            fixture.inspect(&journal).unwrap()[0],
            crate::executor::RecoveredItem::PublishedVerified
        );
    }
    #[test]
    fn durable_operation_job_transitions_are_atomic_monotonic_and_recover_without_io() {
        use crate::library::operation_jobs;
        use crate::operations::{CopyAction, CopyJobState, CopyWorkflowJob};
        let fixture = Fixture::new().unwrap();
        let owner = fixture.owner.clone();
        let manifest = fixture.manifest.clone();
        let root = fixture
            .library
            .catalog
            .call(move |db| {
                Ok(copy_manifest::load(db, &owner, &manifest)?.files[0]
                    .view
                    .root_id
                    .clone())
            })
            .unwrap();
        let job = CopyWorkflowJob {
            id: uuid::Uuid::new_v4().to_string(),
            plan_id: fixture.owner.clone(),
            review_id: uuid::Uuid::new_v4().to_string(),
            source_root_id: root,
            action: CopyAction::CopyFiles,
            state: CopyJobState::Queued,
            total_files: 2,
            verified_files: 0,
            ready_review_id: None,
            message: "Metadata transition fixture; no copy executed".into(),
            error: None,
            started_at: "now".into(),
            updated_at: "now".into(),
        };
        operation_jobs::create(&fixture.library.catalog, job.clone(), "a".repeat(64)).unwrap();
        let mut invalid = job.clone();
        invalid.state = CopyJobState::CompletedVerified;
        invalid.verified_files = 2;
        assert!(
            operation_jobs::transition(&fixture.library.catalog, invalid, CopyJobState::Queued)
                .is_err()
        );
        fixture.library.catalog.call(|db|{db.execute_batch("CREATE TRIGGER reject_job_transition BEFORE INSERT ON operation_journal WHEN NEW.phase='operation_job_state_v1' AND json_extract(NEW.evidence,'$.state')='running' BEGIN SELECT RAISE(ABORT,'owned job journal denial'); END;")?;Ok(())}).unwrap();
        let mut running = job.clone();
        running.state = CopyJobState::Running;
        assert!(
            operation_jobs::transition(
                &fixture.library.catalog,
                running.clone(),
                CopyJobState::Queued
            )
            .is_err()
        );
        assert_eq!(
            operation_jobs::load(&fixture.library.catalog, job.id.clone())
                .unwrap()
                .state,
            CopyJobState::Queued
        );
        fixture
            .library
            .catalog
            .call(|db| {
                db.execute_batch("DROP TRIGGER reject_job_transition;")?;
                Ok(())
            })
            .unwrap();
        operation_jobs::transition(
            &fixture.library.catalog,
            running.clone(),
            CopyJobState::Queued,
        )
        .unwrap();
        running.verified_files = 1;
        operation_jobs::transition(
            &fixture.library.catalog,
            running.clone(),
            CopyJobState::Running,
        )
        .unwrap();
        running.verified_files = 0;
        assert!(
            operation_jobs::transition(&fixture.library.catalog, running, CopyJobState::Running)
                .is_err()
        );
        operation_jobs::recover(&fixture.library.catalog).unwrap();
        operation_jobs::recover(&fixture.library.catalog).unwrap();
        let recovered = operation_jobs::load(&fixture.library.catalog, job.id).unwrap();
        assert_eq!(recovered.state, CopyJobState::NeedsRecovery);
        assert_eq!(recovered.verified_files, 1);
        assert!(
            operation_jobs::transition(
                &fixture.library.catalog,
                recovered,
                CopyJobState::NeedsRecovery
            )
            .is_err()
        );
        assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 1);
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn application_copy_api_prepares_and_copies_only_after_two_exact_session_approvals() {
        use crate::operations::{CopyAction, CopyApprovalInput, CopyJobState, CopyStartInput};
        let fixture = Fixture::with_nested(true).unwrap();
        // A picker may return a valid Windows spelling that differs in case
        // from enumeration. Native publication must use the held base handle,
        // without treating display/path spelling as an exact child locator.
        #[cfg(windows)]
        let fixture = {
            let mut fixture = fixture;
            let name = fixture.destination.file_name().unwrap().to_str().unwrap();
            fixture.destination = fixture.destination.with_file_name(name.to_uppercase());
            fixture
        };
        let selected = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&selected).unwrap();
        let volume = crate::library::native::volume_token(&selected).unwrap();
        drop(selected);
        let selection = uuid::Uuid::new_v4().to_string();
        let mut review = fixture
            .library
            .organization_copy_review_create(
                fixture.owner.clone(),
                fixture.manifest.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert_eq!(review.action, CopyAction::PrepareNames);
        assert!(!fixture.destination.join(&review.output_folder).exists());
        assert!(
            review
                .files
                .iter()
                .all(|file| file.target_path.starts_with(&review.output_folder))
        );
        for action in [CopyAction::PrepareNames, CopyAction::CopyFiles] {
            assert_eq!(review.action, action);
            if action == CopyAction::CopyFiles {
                assert!(
                    fixture
                        .library
                        .organization_copy_approve(
                            CopyApprovalInput {
                                review_id: review.id.clone(),
                                digest: review.digest.clone(),
                                action,
                                fidelity_limits_acknowledged: false
                            },
                            selection.clone(),
                            fixture.destination.clone(),
                            (identity.clone(), volume.clone())
                        )
                        .is_err()
                );
            }
            let mut wrong = review.digest.clone();
            wrong.replace_range(..1, if wrong.starts_with('0') { "1" } else { "0" });
            assert!(
                fixture
                    .library
                    .organization_copy_approve(
                        CopyApprovalInput {
                            review_id: review.id.clone(),
                            digest: wrong,
                            action,
                            fidelity_limits_acknowledged: true
                        },
                        selection.clone(),
                        fixture.destination.clone(),
                        (identity.clone(), volume.clone())
                    )
                    .is_err()
            );
            let approval = fixture
                .library
                .organization_copy_approve(
                    CopyApprovalInput {
                        review_id: review.id.clone(),
                        digest: review.digest.clone(),
                        action,
                        fidelity_limits_acknowledged: true,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            let input = CopyStartInput {
                review_id: review.id.clone(),
                digest: review.digest.clone(),
                approval_id: approval.id,
            };
            let job = fixture
                .library
                .organization_copy_start(
                    input.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let finished = loop {
                let current = fixture
                    .library
                    .organization_copy_job(job.id.clone())
                    .unwrap();
                if !matches!(
                    current.state,
                    CopyJobState::Queued
                        | CopyJobState::Running
                        | CopyJobState::PauseRequested
                        | CopyJobState::CancelRequested
                ) {
                    break current;
                }
                assert!(
                    std::time::Instant::now() < until,
                    "Organization job did not finish"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            };
            assert_eq!(
                finished.state,
                if action == CopyAction::PrepareNames {
                    CopyJobState::Prepared
                } else {
                    CopyJobState::CompletedVerified
                },
                "{:?}",
                finished.error
            );
            assert_eq!(
                finished.verified_files,
                if action == CopyAction::CopyFiles {
                    2
                } else {
                    0
                }
            );
            assert!(fixture.preserved().unwrap());
            assert!(
                fixture
                    .library
                    .organization_copy_start(
                        input,
                        selection.clone(),
                        fixture.destination.clone(),
                        (identity.clone(), volume.clone())
                    )
                    .is_err()
            );
            review = fixture
                .library
                .organization_copy_review(
                    review.id,
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
        }
        let inspection = fixture
            .library
            .organization_copy_inspect(
                review.id,
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert!(inspection.all_published_bytes_verified);
        assert_eq!(
            inspection.states,
            vec![crate::operations::CopyInspectionState::PublishedVerified; 2]
        );
        assert_eq!(
            crate::library::operation_jobs::list(&fixture.library.catalog)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            std::fs::read(
                fixture
                    .destination
                    .join(&review.output_folder)
                    .join("organized/Photo.JPG")
            )
            .unwrap(),
            generated_photo_bytes().unwrap()
        );
        assert_eq!(
            std::fs::read(
                fixture
                    .destination
                    .join(&review.output_folder)
                    .join("organized/Photo.xmp")
            )
            .unwrap(),
            COMPANION_BYTES
        );
        // A new controller has no live sessions or approvals. Inspection reads
        // only retained job/namespace/copy records, including referenced roots.
        let mut jobs = crate::library::operation_jobs::list(&fixture.library.catalog).unwrap();
        let original_job = jobs
            .iter()
            .find(|job| job.action == CopyAction::CopyFiles)
            .unwrap()
            .clone();
        let next = equivalent_manifest(&fixture, false);
        let matched = fixture
            .library
            .organization_copy_effect_match(
                next.plan_id.clone(),
                next.id.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert_eq!(matched.existing.len(), 1);
        assert_eq!(
            matched.existing[0].job.as_ref().unwrap().id,
            original_job.id
        );
        assert!(!matched.can_reuse_review);
        assert!(
            fixture
                .library
                .organization_copy_review_create(
                    next.plan_id,
                    next.id,
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err()
        );
        let fresh = super::super::workflow::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let before: i64 = fixture
            .library
            .catalog
            .call(
                |db| Ok(db.query_row("SELECT count(*) FROM operation_journal", [], |r| r.get(0))?),
            )
            .unwrap();
        let inspected = fresh
            .inspect_retained(
                original_job.id.clone(),
                fixture.destination.clone(),
                identity.clone(),
                volume.clone(),
            )
            .unwrap();
        assert!(inspected.inspection.all_published_bytes_verified);
        let after: i64 = fixture
            .library
            .catalog
            .call(
                |db| Ok(db.query_row("SELECT count(*) FROM operation_journal", [], |r| r.get(0))?),
            )
            .unwrap();
        assert_eq!(before, after);

        let chosen = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&chosen).unwrap();
        let volume = crate::library::native::volume_token(&chosen).unwrap();
        drop(chosen);
        let selection = uuid::Uuid::new_v4().to_string();
        let continuation = fixture
            .library
            .organization_copy_continuation_review(
                original_job.id.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert!(continuation.recorded_complete);
        assert_eq!(continuation.assessment.capacity.rounded_data_bytes, "0");
        assert!(
            fixture
                .library
                .organization_copy_continuation_approve(
                    CopyApprovalInput {
                        review_id: continuation.id.clone(),
                        digest: continuation.digest.clone(),
                        action: CopyAction::CopyFiles,
                        fidelity_limits_acknowledged: false
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err()
        );
        let approval = fixture
            .library
            .organization_copy_continuation_approve(
                CopyApprovalInput {
                    review_id: continuation.id.clone(),
                    digest: continuation.digest.clone(),
                    action: CopyAction::CopyFiles,
                    fidelity_limits_acknowledged: true,
                },
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let paths = ["Photo.JPG", "Photo.xmp"].map(|name| {
            fixture
                .destination
                .join(&review.output_folder)
                .join("organized")
                .join(name)
        });
        let objects = paths
            .iter()
            .map(|path| {
                let file = std::fs::File::open(path).unwrap();
                (
                    filesystem::identity(&file).unwrap(),
                    crate::library::native::revision(&file).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let mut input = CopyStartInput {
            review_id: continuation.id.clone(),
            digest: continuation.digest.clone(),
            approval_id: approval.id,
        };
        fixture.library.catalog.call(|db|{db.execute_batch("CREATE TRIGGER deny_continuation_admission BEFORE INSERT ON operation_jobs WHEN NEW.kind='copy_files' BEGIN SELECT RAISE(ABORT,'fixture queued job denied'); END;")?;Ok(())}).unwrap();
        let records_before = fixture
            .library
            .catalog
            .call(|db| {
                Ok(
                    db.query_row("SELECT count(*) FROM operation_journal", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                )
            })
            .unwrap();
        assert!(
            fixture
                .library
                .organization_copy_continuation_start(
                    input.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err()
        );
        assert_eq!(
            crate::library::operation_jobs::list(&fixture.library.catalog)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            fixture
                .library
                .catalog
                .call(|db| Ok(db.query_row(
                    "SELECT count(*) FROM operation_journal",
                    [],
                    |row| row.get::<_, i64>(0)
                )?))
                .unwrap(),
            records_before
        );
        assert!(
            fixture
                .library
                .organization_copy_continuation_start(
                    input.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err()
        );
        assert!(fixture.preserved().unwrap());
        fixture
            .library
            .catalog
            .call(|db| {
                db.execute_batch("DROP TRIGGER deny_continuation_admission")?;
                Ok(())
            })
            .unwrap();
        input.approval_id = fixture
            .library
            .organization_copy_continuation_approve(
                CopyApprovalInput {
                    review_id: continuation.id.clone(),
                    digest: continuation.digest.clone(),
                    action: CopyAction::CopyFiles,
                    fidelity_limits_acknowledged: true,
                },
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap()
            .id;
        let continued = fixture
            .library
            .organization_copy_continuation_start(
                input.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let completed = loop {
            let current = fixture
                .library
                .organization_copy_job(continued.id.clone())
                .unwrap();
            if !matches!(
                current.state,
                CopyJobState::Queued
                    | CopyJobState::Running
                    | CopyJobState::PauseRequested
                    | CopyJobState::CancelRequested
            ) {
                break current;
            }
            assert!(std::time::Instant::now() < until);
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(
            completed.state,
            CopyJobState::CompletedVerified,
            "{:?}",
            completed.error
        );
        assert_eq!(completed.verified_files, 2);
        assert!(
            fixture
                .library
                .organization_copy_continuation_start(
                    input,
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err()
        );
        for (path, expected) in paths.iter().zip(objects) {
            let file = std::fs::File::open(path).unwrap();
            assert_eq!(
                (
                    filesystem::identity(&file).unwrap(),
                    crate::library::native::revision(&file).unwrap()
                ),
                expected
            );
        }
        let fresh = fixture
            .library
            .organization_copy_recovery_inspect(
                completed.id.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert!(fresh.inspection.all_published_bytes_verified);
        assert_eq!(
            fresh.files[0].source_path,
            std::path::Path::new("source-folder")
                .join("Photo.JPG")
                .to_string_lossy()
        );
        let owner = fixture.owner.clone();
        let stream = completed.review_id.clone();
        assert_eq!(fixture.library.catalog.call(move|db|Ok(db.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3",params![owner,stream,EVENT_PHASE],|row|row.get::<_,i64>(0))?)).unwrap(),13);
        let foreign = fixture
            .destination
            .join(&review.output_folder)
            .join("foreign-retained.bin");
        std::fs::write(&foreign, b"unknown object stays").unwrap();
        assert!(
            fixture
                .library
                .organization_copy_continuation_review(
                    completed.id.clone(),
                    selection,
                    fixture.destination.clone(),
                    (identity, volume)
                )
                .is_err()
        );
        assert_eq!(std::fs::read(foreign).unwrap(), b"unknown object stays");
        jobs = crate::library::operation_jobs::list(&fixture.library.catalog).unwrap();
        assert_eq!(jobs.len(), 3);
        let copy_job = jobs.iter().find(|job| job.id == completed.id).unwrap();
        let restarted = crate::executor::CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        fixture
            .library
            .catalog
            .call(|db| {
                db.execute("UPDATE roots SET mode='referenced'", [])?;
                Ok(())
            })
            .unwrap();
        let before: i64 = fixture
            .library
            .catalog
            .call(|db| {
                Ok(
                    db.query_row("SELECT count(*) FROM operation_journal", [], |row| {
                        row.get(0)
                    })?,
                )
            })
            .unwrap();
        let selected = crate::library::native::open_root(&fixture.destination, None).unwrap();
        let identity = crate::filesystem::identity(&selected).unwrap();
        let volume = crate::library::native::volume_token(&selected).unwrap();
        drop(selected);
        let recovered = restarted
            .inspect_retained(
                copy_job.id.clone(),
                fixture.destination.clone(),
                identity.clone(),
                volume.clone(),
            )
            .unwrap();
        assert!(recovered.inspection.all_published_bytes_verified);
        assert_eq!(recovered.files.len(), 2);
        assert_eq!(recovered.source_root_id, copy_job.source_root_id);
        let wrong = tempfile::tempdir().unwrap();
        assert!(
            restarted
                .inspect_retained(
                    copy_job.id.clone(),
                    wrong.path().into(),
                    identity.clone(),
                    volume.clone()
                )
                .is_err()
        );
        let target = fixture
            .destination
            .join(&review.output_folder)
            .join("organized/Photo.JPG");
        std::fs::write(&target, b"new output must be retained").unwrap();
        let changed = restarted
            .inspect_retained(
                copy_job.id.clone(),
                fixture.destination.clone(),
                identity,
                volume,
            )
            .unwrap();
        assert!(!changed.inspection.all_published_bytes_verified);
        assert!(
            changed
                .inspection
                .states
                .contains(&crate::operations::CopyInspectionState::NeedsAttention)
        );
        assert_eq!(
            std::fs::read(target).unwrap(),
            b"new output must be retained"
        );
        assert_eq!(
            before,
            fixture
                .library
                .catalog
                .call(|db| Ok(db.query_row(
                    "SELECT count(*) FROM operation_journal",
                    [],
                    |row| row.get::<_, i64>(0)
                )?))
                .unwrap()
        );
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn approved_copy_rechecks_revoked_root_grant_at_admission_and_before_publication() {
        use crate::executor::approval::Approvals;
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        for during_copy in [false, true] {
            let fixture = Fixture::new().unwrap();
            let journal = fixture.journal().unwrap();
            let selection = uuid::Uuid::new_v4().to_string();
            let review = journal.copy_review(&selection).unwrap();
            let approvals = Approvals::new();
            let ticket = approvals.issue(review.clone()).unwrap();
            let proof = approvals.consume(&ticket, &review).unwrap();
            let revoke = || {
                fixture.library.catalog.call(|db| {
                    db.execute("UPDATE roots SET mode='referenced'", [])?;
                    Ok(())
                })
            };
            if !during_copy {
                revoke().unwrap();
            }
            let (source, output) = fixture.scope().unwrap();
            let count = AtomicUsize::new(0);
            let validate = || {
                fixture.scope()?;
                if during_copy && count.fetch_add(1, Ordering::Relaxed) == 1 {
                    revoke()?;
                }
                Ok(())
            };
            let io = crate::jobs::paced_io::PacedIo::shared();
            let cancel = AtomicBool::new(false);
            let error = journal
                .execute_approved(
                    crate::executor::CopyInputs {
                        source_root: source.leaf(),
                        destination: output.leaf(),
                        io: &io,
                        cancel: &cancel,
                        validate_scope: &validate,
                    },
                    &selection,
                    proof,
                    None,
                    None,
                )
                .err()
                .unwrap();
            assert_eq!(error.code, ErrorCode::Conflict);
            assert!(error.message.contains("managed"));
            assert_eq!(
                journal.history_count().unwrap(),
                if during_copy { 4 } else { 0 }
            );
            assert!(
                fixture
                    .plan
                    .files
                    .iter()
                    .all(|file| !fixture.destination.join(&file.target).exists())
            );
            if during_copy {
                assert_eq!(
                    fixture.inspect(&journal).unwrap(),
                    vec![
                        crate::executor::RecoveredItem::StagedVerified,
                        crate::executor::RecoveredItem::NotStarted
                    ]
                );
            }
            assert!(fixture.preserved().unwrap());
        }
    }
    fn crash_boundaries() -> [super::super::CopyFault; 9] {
        use super::super::CopyFault::*;
        [
            AfterIntent,
            AfterStageCreation,
            AfterCopyChunk,
            AfterFlush,
            AfterVerification,
            BeforePublish,
            AfterPublish,
            AfterItemCommit,
            AfterPlanCommit,
        ]
    }
    #[test]
    fn catalog_crash_fixture_child() {
        use std::io::Read;
        let Some(root) = std::env::var_os("STILLHARBOUR_M2_CATALOG_CHILD_ROOT") else {
            return;
        };
        let root = std::path::PathBuf::from(root).canonicalize().unwrap();
        let token = std::env::var("STILLHARBOUR_M2_CATALOG_CHILD_TOKEN").unwrap();
        uuid::Uuid::parse_str(&token).unwrap();
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("stillharbour-catalog-copy-")
        );
        let run_scope = crate::filesystem::DirectoryChain::open(&root).unwrap();
        let marker = super::super::native::read(run_scope.leaf(), "fixture-owner").unwrap();
        assert_eq!(marker.metadata().unwrap().len(), token.len() as u64);
        assert_eq!(filesystem::identity(&marker).unwrap().links, 1);
        let mut owner = Vec::new();
        marker
            .take(token.len() as u64 + 1)
            .read_to_end(&mut owner)
            .unwrap();
        assert_eq!(owner, token.as_bytes());
        let source = crate::filesystem::DirectoryChain::open(&root.join("source")).unwrap();
        let output = super::super::output_chain(&root.join("output")).unwrap();
        let _data = crate::filesystem::DirectoryChain::open(&root.join("data")).unwrap();
        // Validate the fixed constructor-owned source bytes through held native
        // parents, with fixed-size buffers; never adopt supplied library paths.
        for (parent, name, expected) in [
            (source.leaf(), "Photo.JPG", generated_photo_bytes().unwrap()),
            (source.leaf(), "Photo.xmp", COMPANION_BYTES.to_vec()),
            (
                output.leaf(),
                "destination-only.bin",
                NEIGHBOR_BYTES.to_vec(),
            ),
        ] {
            let file = super::super::native::read(parent, name).unwrap();
            assert_eq!(file.metadata().unwrap().len(), expected.len() as u64);
            assert_eq!(filesystem::identity(&file).unwrap().links, 1);
            let mut bytes = Vec::new();
            file.take(expected.len() as u64 + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            assert_eq!(bytes, expected);
        }
        let database = root.join("data/Fixture.stillharbour");
        let proof = crate::filesystem::open_readonly(&database).unwrap();
        assert!(proof.metadata().unwrap().is_file());
        assert_eq!(filesystem::identity(&proof).unwrap().links, 1);
        assert!(proof.metadata().unwrap().len() <= 8 * 1024 * 1024);
        drop(proof);
        let catalog = Arc::new(Catalog::open(database).unwrap());
        assert_eq!(
            catalog.summary().unwrap().schema,
            crate::catalog::SCHEMA_VERSION
        );
        let selected = token.clone();
        let expected_source = crate::library::native::encode_path(&root.join("source"));
        let binding=catalog.call(move|db| {
            let text:Option<String>=db.query_row("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE phase=?1 AND item_id=?2",params![PLAN_PHASE,selected],|row|row.get(0))?;
            let binding:Binding=decode(text.as_deref().unwrap(),MAX_RECORD)?;
            let paths:Vec<Vec<u8>>=db.prepare("SELECT native_path FROM roots")?.query_map([],|row|row.get(0))?.collect::<rusqlite::Result<_>>()?;
            assert_eq!(paths,vec![expected_source]);
            Ok(binding)
        }).unwrap();
        assert_eq!(binding.plan.id, token);
        assert_eq!(binding.plan.files.len(), 2);
        assert_eq!(
            binding
                .plan
                .files
                .iter()
                .map(|file| file.source.as_str())
                .collect::<Vec<_>>(),
            ["Photo.JPG", "Photo.xmp"]
        );
        assert_eq!(
            binding
                .plan
                .files
                .iter()
                .map(|file| file.target.as_str())
                .collect::<Vec<_>>(),
            ["Photo.JPG", "Photo.xmp"]
        );
        assert!(binding.plan.source.matches(source.leaf()).unwrap());
        assert!(binding.plan.destination.matches(output.leaf()).unwrap());
        let journal = CatalogJournal::bind(
            catalog,
            binding.owner_id,
            binding.manifest_id,
            binding.plan.clone(),
        )
        .unwrap();
        assert_eq!(journal.history_count().unwrap(), 0);
        let point = std::env::var("STILLHARBOUR_M2_CATALOG_CHILD_FAULT").unwrap();
        let fault = crash_boundaries()
            .into_iter()
            .find(|fault| format!("{fault:?}") == point)
            .unwrap();
        let validate = || {
            let source = crate::filesystem::DirectoryChain::open(&root.join("source"))?;
            let output = super::super::output_chain(&root.join("output"))?;
            if !binding.plan.source.matches(source.leaf())?
                || !binding.plan.destination.matches(output.leaf())?
            {
                return Err(failed("Owned catalog crash-fixture roots changed."));
            }
            Ok(())
        };
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let lease = Registry::shared()
            .try_acquire(vec![
                Scope::new(source, Access::Read),
                Scope::new(output, Access::Write),
            ])
            .unwrap();
        let io = crate::jobs::paced_io::PacedIo::shared();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        super::super::CRASH_ON_FAULT.store(true, std::sync::atomic::Ordering::Release);
        if std::env::var_os("STILLHARBOUR_M2_CATALOG_CHILD_RESUME").is_some() {
            assert_eq!(
                std::env::var("STILLHARBOUR_M2_CATALOG_CHILD_RESUME").unwrap(),
                "1"
            );
            let inputs = super::super::CopyInputs {
                source_root: lease.directory(0).leaf(),
                destination: lease.directory(1).leaf(),
                io: &io,
                cancel: &cancel,
                validate_scope: &validate,
            };
            let review = journal.recovery_review(&inputs).unwrap();
            let selection = uuid::Uuid::new_v4().to_string();
            let expected = review.approval(&binding.plan, &selection).unwrap();
            let approvals = super::super::approval::Approvals::new();
            let ticket = approvals.issue(expected.clone()).unwrap();
            let proof = approvals.consume(&ticket, &expected).unwrap();
            let _ = journal.resume_approved(inputs, &selection, review, proof, Some(fault), None);
            panic!("Owned reviewed continuation process-exit boundary was not reached");
        }
        let _ = super::super::CopyContext {
            plan: &binding.plan,
            journal: &journal,
            source_root: lease.directory(0).leaf(),
            destination: lease.directory(1).leaf(),
            io: &io,
            cancel: &cancel,
            validate_scope: &validate,
        }
        .execute(Some(fault));
        panic!("Owned catalog process-exit boundary was not reached");
    }
    #[test]
    fn catalog_actor_process_exit_matrix_reopens_real_state_without_replay_or_source_loss() {
        catalog_exit_matrix(false);
    }
    #[test]
    fn reviewed_continuation_process_exit_matrix_requires_fresh_approval_after_reopening() {
        catalog_exit_matrix(true);
    }
    fn catalog_exit_matrix(resume: bool) {
        for fault in crash_boundaries() {
            let fixture = Fixture::new().unwrap();
            drop(fixture.journal().unwrap());
            let Fixture {
                library,
                _run,
                source,
                destination,
                owner,
                manifest,
                plan,
                baseline,
            } = fixture;
            let root = _run.path().canonicalize().unwrap();
            std::fs::write(root.join("fixture-owner"), plan.id.as_bytes()).unwrap();
            drop(library);
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "executor::catalog_journal::tests::catalog_crash_fixture_child",
                    "--nocapture",
                ])
                .env("STILLHARBOUR_M2_CATALOG_CHILD_ROOT", &root)
                .env("STILLHARBOUR_M2_CATALOG_CHILD_TOKEN", &plan.id)
                .env("STILLHARBOUR_M2_CATALOG_CHILD_FAULT", format!("{fault:?}"));
            if resume {
                command.env("STILLHARBOUR_M2_CATALOG_CHILD_RESUME", "1");
            } else {
                command.env_remove("STILLHARBOUR_M2_CATALOG_CHILD_RESUME");
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW; owned noninteractive child.
            }
            let mut child = command.spawn().unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("Owned catalog crash child timed out");
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            };
            assert_eq!(
                status.code(),
                Some(77),
                "Catalog child did not reach {fault:?}"
            );
            let catalog = Arc::new(Catalog::open(root.join("data/Fixture.stillharbour")).unwrap());
            let library = crate::library::Library::new(
                catalog,
                crate::library::LibraryOptions {
                    data_directory: root.join("data"),
                    workers: crate::library::WorkerPaths {
                        executable: std::env::current_exe().unwrap(),
                        exiftool: None,
                        working_directory: root.join("data/worker"),
                    },
                },
            )
            .unwrap();
            let fixture = Fixture {
                library,
                _run,
                source,
                destination,
                owner,
                manifest,
                plan,
                baseline,
            };
            let journal = fixture.journal().unwrap();
            let before = journal.history_count().unwrap();
            let states = fixture.inspect(&journal).unwrap();
            assert_eq!(fixture.inspect(&journal).unwrap(), states);
            use super::super::{CopyFault, RecoveredItem};
            let expected = match fault {
                CopyFault::AfterIntent => RecoveredItem::NotStarted,
                CopyFault::AfterStageCreation => RecoveredItem::RetainedPartialStage,
                CopyFault::AfterCopyChunk
                | CopyFault::AfterFlush
                | CopyFault::AfterVerification
                | CopyFault::BeforePublish => RecoveredItem::StagedVerified,
                CopyFault::AfterPublish
                | CopyFault::AfterItemCommit
                | CopyFault::AfterPlanCommit => RecoveredItem::PublishedVerified,
                _ => unreachable!(),
            };
            assert_eq!(
                states[0], expected,
                "Unexpected recovery state at {fault:?}"
            );
            assert_eq!(
                states[1],
                if fault == CopyFault::AfterPlanCommit {
                    RecoveredItem::PublishedVerified
                } else {
                    RecoveredItem::NotStarted
                }
            );
            assert!(fixture.execute(&journal, None).is_err());
            assert_eq!(journal.history_count().unwrap(), before);
            assert!(fixture.preserved().unwrap());
            if resume {
                assert_eq!(
                    fixture.resume(&journal, None).unwrap(),
                    super::super::CopyOutcome::CompletedVerified
                );
                assert_eq!(journal.history_count().unwrap(), 13);
                assert!(fixture.preserved().unwrap());
                assert_eq!(
                    fixture.resume(&journal, None).unwrap(),
                    super::super::CopyOutcome::CompletedVerified
                );
                assert_eq!(journal.history_count().unwrap(), 13);
            }
        }
    }
    #[test]
    fn catalog_stream_uses_actual_retained_scope_and_reopens_completed_and_interrupted_evidence() {
        let receipt = fixture_probe().unwrap();
        assert_eq!(receipt["completed"]["publishedVerified"], 2);
        assert_eq!(receipt["interruptedAfterPublish"]["publishedVerified"], 1);
        assert_eq!(receipt["nested"]["publishedVerified"], 2);
    }
    #[test]
    fn nested_parent_substitution_never_adopts_new_source_or_destination_directories() {
        for source_swap in [true, false] {
            let fixture = Fixture::with_nested(true).unwrap();
            let journal = fixture.journal().unwrap();
            if source_swap {
                std::fs::rename(
                    fixture.source.join("source-folder"),
                    fixture.source.join("retained-source-folder"),
                )
                .unwrap();
                std::fs::create_dir(fixture.source.join("source-folder")).unwrap();
                std::fs::write(
                    fixture.source.join("source-folder/Photo.JPG"),
                    generated_photo_bytes().unwrap(),
                )
                .unwrap();
                std::fs::write(
                    fixture.source.join("source-folder/Photo.xmp"),
                    COMPANION_BYTES,
                )
                .unwrap();
            } else {
                std::fs::rename(
                    fixture.destination.join("organized"),
                    fixture.destination.join("retained-output-folder"),
                )
                .unwrap();
                std::fs::create_dir(fixture.destination.join("organized")).unwrap();
                std::fs::write(
                    fixture.destination.join("organized/untouched.bin"),
                    b"new destination occupant",
                )
                .unwrap();
            }
            assert!(fixture.execute(&journal, None).is_err());
            assert_eq!(journal.history_count().unwrap(), 0);
            assert!(fixture.preserved().unwrap());
            if source_swap {
                assert_eq!(
                    std::fs::read(fixture.source.join("retained-source-folder/Photo.JPG")).unwrap(),
                    generated_photo_bytes().unwrap()
                );
                assert_eq!(
                    std::fs::read(fixture.source.join("retained-source-folder/Photo.xmp")).unwrap(),
                    COMPANION_BYTES
                );
            } else {
                assert_eq!(
                    std::fs::read(fixture.destination.join("organized/untouched.bin")).unwrap(),
                    b"new destination occupant"
                );
            }
        }
    }
    #[test]
    fn bounded_unknown_retained_events_fail_before_execution_and_consistent_snapshots_reopen() {
        for payload in [
            serde_json::json!({"padding":"x".repeat(MAX_EVENT)}),
            serde_json::json!({"ordinal":1,"stream_digest":"foreign","item":0,"phase":"item_intent","evidence":{}}),
        ] {
            let fixture = Fixture::new().unwrap();
            let journal = fixture.journal().unwrap();
            drop(journal);
            let owner = fixture.owner.clone();
            let stream = fixture.plan.id.clone();
            fixture.library.catalog.call(move |db| {
                db.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,'owned-fixture')",params![owner,stream,EVENT_PHASE,payload.to_string()])?;Ok(())
            }).unwrap();
            assert!(fixture.journal().is_err());
            assert!(fixture.preserved().unwrap());
        }
        let fixture = Fixture::new().unwrap();
        let journal = fixture.journal().unwrap();
        fixture.execute(&journal, None).unwrap();
        drop(journal);
        let snapshot = fixture._run.path().join("Snapshot.stillharbour");
        fixture.library.catalog.snapshot(snapshot.clone()).unwrap();
        let restored = Arc::new(Catalog::open(snapshot).unwrap());
        let journal = CatalogJournal::bind(
            restored,
            fixture.owner.clone(),
            fixture.manifest.clone(),
            fixture.plan.clone(),
        )
        .unwrap();
        assert_eq!(
            fixture.inspect(&journal).unwrap(),
            [
                super::super::RecoveredItem::PublishedVerified,
                super::super::RecoveredItem::PublishedVerified
            ]
        );
        assert!(fixture.execute(&journal, None).is_err());
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn catalog_write_denial_after_publication_never_invents_completion() {
        let fixture = Fixture::new().unwrap();
        let journal = fixture.journal().unwrap();
        fixture.library.catalog.call(|db| {db.execute_batch("CREATE TRIGGER fixture_deny_completion BEFORE INSERT ON operation_journal WHEN NEW.phase='copy_execution_event_v1' AND json_extract(NEW.evidence,'$.phase')='item_completed_verified' BEGIN SELECT RAISE(ABORT,'Owned fixture denied completion'); END;")?;Ok(())}).unwrap();
        assert_eq!(
            fixture.execute(&journal, None).unwrap_err().code,
            ErrorCode::Catalog
        );
        drop(journal);
        let reopened = fixture.journal().unwrap();
        assert_eq!(
            fixture.inspect(&reopened).unwrap(),
            [
                super::super::RecoveredItem::PublishedVerified,
                super::super::RecoveredItem::NotStarted
            ]
        );
        assert!(fixture.preserved().unwrap());
        fixture.library.catalog.call(|db| {let complete:u32=db.query_row("SELECT count(*) FROM operation_journal WHERE phase='copy_execution_event_v1' AND json_extract(evidence,'$.phase') IN ('item_completed_verified','plan_completed_verified')",[],|row|row.get(0))?;assert_eq!(complete,0);Ok(())}).unwrap();
    }
    #[test]
    fn unknown_foreign_out_of_order_and_concurrent_journal_writes_fail_closed() {
        let fixture = Fixture::new().unwrap();
        let first = fixture.journal().unwrap();
        let second = fixture.journal().unwrap();
        assert!(
            first
                .append(Some(0), "item_completed_verified", serde_json::json!({}))
                .is_err()
        );
        assert!(
            first
                .append(
                    None,
                    "plan_completed_verified",
                    serde_json::json!({"files":2})
                )
                .is_err()
        );
        assert!(
            first
                .append(Some(0), "invented", serde_json::json!({}))
                .is_err()
        );
        let file = &fixture.plan.files[0];
        let intent = serde_json::json!({"stage":file.stage,"target":file.target,"sourceRevision":file.revision});
        first
            .append(Some(0), "item_intent", intent.clone())
            .unwrap();
        assert!(second.append(Some(0), "item_intent", intent).is_err());
        let mut wrong = fixture.plan.clone();
        wrong.files[0].prepared_stage = Some(super::super::PreparedStage {
            object: wrong.files[0].object.clone(),
            revision: wrong.files[0].revision.clone(),
        });
        wrong.digest = wrong.digest().unwrap();
        let error = CatalogJournal::bind(
            fixture.library.catalog.clone(),
            fixture.owner.clone(),
            fixture.manifest.clone(),
            wrong,
        )
        .err()
        .expect("Unbound prepared-stage authority was accepted");
        assert_eq!(error.code, ErrorCode::Conflict);
        assert!(error.message.contains("retained namespace reference"));
        let mut wrong = fixture.plan.clone();
        wrong.files[0].target = "different.JPG".into();
        wrong.digest = wrong.digest().unwrap();
        assert!(
            CatalogJournal::bind(
                fixture.library.catalog.clone(),
                fixture.owner.clone(),
                fixture.manifest.clone(),
                wrong
            )
            .is_err()
        );
        assert!(
            CatalogJournal::bind(
                fixture.library.catalog.clone(),
                fixture.owner.clone(),
                uuid::Uuid::new_v4().to_string(),
                fixture.plan.clone()
            )
            .is_err()
        );
        assert!(fixture.preserved().unwrap());
        assert_eq!(first.history_count().unwrap(), 1);
        assert_eq!(second.history_count().unwrap(), 0);
        fixture
            .library
            .catalog
            .call(|db| {
                db.execute("UPDATE roots SET mode='referenced'", [])?;
                Ok(())
            })
            .unwrap();
        assert!(fixture.journal().is_err());
        let duplicate = Fixture::new().unwrap();
        drop(duplicate.journal().unwrap());
        duplicate.library.catalog.call(|db| {
            db.execute_batch("DROP INDEX operation_copy_stream;INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) SELECT plan_id,item_id,phase,evidence,created_at FROM operation_journal WHERE phase='copy_execution_plan_v1';")?;Ok(())
        }).unwrap();
        let error = duplicate
            .journal()
            .err()
            .expect("Duplicate stream anchors were adopted");
        assert!(
            error
                .message
                .contains("Multiple immutable copy-stream anchors")
        );
        assert!(duplicate.preserved().unwrap());
    }
}
