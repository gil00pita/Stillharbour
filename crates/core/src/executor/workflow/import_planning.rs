//! Immutable decisions and native keeper bindings, separate from execution.
use super::*;
use crate::{
    executor::{fingerprints::Entry, sources::Snapshot},
    filesystem::DirectoryChain,
    jobs::operation_locks::{Access, Registry, Scope},
    library::{copy_manifest, native, operation_store},
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const PHASE: &str = "import_selection_plan_v1";
const MAX: usize = 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Root {
    pub id: String,
    pub native_path: Vec<u8>,
    pub object: Object,
    pub volume: String,
    pub scan: Option<String>,
}
#[derive(Clone)]
pub(super) struct Evidence {
    pub source: Snapshot,
    pub roots: Vec<Root>,
    pub keepers: Vec<Entry>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Stored {
    policy: String,
    pub view: ImportPlan,
    pub source: Snapshot,
    pub roots: Vec<Root>,
    pub keepers: Vec<Entry>,
    bundles: Vec<ImportBundleObservation>,
}
fn encode(value: &Stored) -> Result<String> {
    let text = serde_json::to_string(value)
        .map_err(|error| failed("Could not encode import decisions.").caused_by(error))?;
    if text.len() > MAX {
        return Err(failed(
            "Import evidence exceeds 1 MiB. Review a smaller selection.",
        ));
    }
    Ok(text)
}
impl Stored {
    fn digest(&self) -> Result<String> {
        let mut value = self.clone();
        // Identical source/keeper decisions reuse retained planning history.
        value.view.id.clear();
        value.view.digest.clear();
        value.view.created_at.clear();
        Ok(blake3::hash(encode(&value)?.as_bytes())
            .to_hex()
            .to_string())
    }
    fn validate(&self, db: &Connection) -> Result<()> {
        let view = &self.view;
        if self.source.import_selection.is_some() {
            return Err(failed(
                "An import decision cannot recursively contain an import copy projection.",
            ));
        }
        validate_id(&view.id)?;
        self.source.validate_mapping(db)?;
        if self.policy != "whole-bundle-import-plan-v1"
            || view.digest != self.digest()?
            || view.plan_id != self.source.owner_id
            || view.manifest_id != self.source.manifest_id
            || view.manifest_digest != self.source.manifest_digest
            || view.source_root_id != self.source.root_id
            || !view.sources_retained
            || view.execution_available != view.copy_file_ids.is_empty()
            || view.choices.len() != self.bundles.len()
            || self.bundles.is_empty()
            || self.bundles.len() > 512
            || self.roots.is_empty()
            || self.roots.len() > 16
            || self.keepers.len() > 512
            || self.source.files.len() > 512
        {
            return Err(failed(
                "Retained import decisions are invalid or changed. Compare again.",
            ));
        }
        let mut roots = self
            .roots
            .iter()
            .map(|root| root.id.clone())
            .collect::<Vec<_>>();
        roots.sort();
        if roots != view.scope_root_ids || roots.windows(2).any(|ids| ids[0] == ids[1]) {
            return Err(failed(
                "Import root evidence differs from its explicit scope.",
            ));
        }
        let manifest = copy_manifest::load(db, &view.plan_id, &view.manifest_id)?;
        // validate_mapping above proves this is every member of the selected
        // root (or the complete legacy single-root manifest), not an arbitrary
        // omission of companions from another source.
        let selected = self
            .source
            .files
            .iter()
            .map(|file| file.id.clone())
            .collect::<BTreeSet<_>>();
        let covered = self
            .bundles
            .iter()
            .flat_map(|bundle| bundle.file_ids.iter().cloned())
            .collect::<Vec<_>>();
        if covered.len() != selected.len()
            || covered.iter().cloned().collect::<BTreeSet<_>>() != selected
        {
            return Err(failed(
                "Import decisions must cover every confirmed source once.",
            ));
        }
        for bundle in &self.bundles {
            let assets = manifest
                .view
                .files
                .iter()
                .filter(|file| bundle.file_ids.contains(&file.id))
                .flat_map(|file| file.bundle_asset_ids.iter().cloned())
                .collect::<BTreeSet<_>>();
            if assets.into_iter().collect::<Vec<_>>() != bundle.bundle_asset_ids
                || manifest.view.files.iter().any(|file| {
                    file.bundle_asset_ids
                        .iter()
                        .any(|id| bundle.bundle_asset_ids.contains(id))
                        && !bundle.file_ids.contains(&file.id)
                })
            {
                return Err(failed(
                    "Connected companion bundles cannot be split by import decisions.",
                ));
            }
        }
        let (copy, skip) = resolve(&self.bundles, &view.choices)?;
        if copy != view.copy_file_ids
            || skip.len() != view.skip_files.len()
            || self.keepers.len() > skip.len()
        {
            return Err(failed(
                "Import file decisions differ from their whole-bundle scope.",
            ));
        }
        let mut unique = BTreeSet::new();
        for (source, asset, root) in skip {
            let matches = view
                .skip_files
                .iter()
                .filter(|file| {
                    file.file_id == source
                        && file.keeper_asset_id == asset
                        && file.keeper_root_id == root
                })
                .collect::<Vec<_>>();
            let entries = self
                .keepers
                .iter()
                .filter(|entry| entry.id == asset && entry.root_id == root)
                .collect::<Vec<_>>();
            if matches.len() != 1
                || entries.len() != 1
                || !unique.insert(source.clone())
                || !roots.contains(&root)
                || !self
                    .source
                    .files
                    .iter()
                    .any(|file| file.id == source && file.bytes == entries[0].bytes)
                || matches[0].digest.len() != 64
                || !matches[0]
                    .digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(failed("Import keeper evidence is incomplete or ambiguous."));
            }
        }
        // Reconstruct native grouping when reading retained/imported records.
        // A recomputed digest alone cannot make edited JSON into bundle proof.
        let draft = operation_store::load(db, &view.plan_id)?;
        let members = self
            .source
            .files
            .iter()
            .zip(&manifest.files)
            .map(|(file, member)| {
                Ok(super::import_bundles::Member {
                    id: file.id.clone(),
                    bundles: member.view.bundle_asset_ids.clone(),
                    path: native::decode_path(&file.location.relative_native)?,
                    primary: draft.sources.iter().any(|primary| {
                        native::decode_path(&primary.native_path).is_ok_and(|path| {
                            native::decode_path(&file.location.relative_native)
                                .is_ok_and(|member| path == member)
                        })
                    }),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let candidates = view
            .skip_files
            .iter()
            .map(|file| {
                let entry = self
                    .keepers
                    .iter()
                    .find(|entry| {
                        entry.id == file.keeper_asset_id && entry.root_id == file.keeper_root_id
                    })
                    .ok_or_else(|| failed("A retained import keeper is missing."))?;
                // Media eligibility is checked against the current index before
                // each actual comparison; this step reconstructs native grouping.
                Ok(super::import_bundles::Keeper {
                    source: file.file_id.clone(),
                    asset: entry.id.clone(),
                    root: entry.root_id.clone(),
                    path: native::decode_path(&entry.location.relative_native)?,
                    media: true,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let files = self
            .source
            .files
            .iter()
            .map(|file| ImportComparisonFile {
                file_id: file.id.clone(),
                source_path: String::new(),
                bytes: file.bytes.to_string(),
                state: ImportFileComparison::ExactBytesCompared,
                matches: Vec::new(),
                further_candidates_uncompared: false,
            })
            .collect::<Vec<_>>();
        let observed = super::import_bundles::observe(&members, &candidates, &files, true);
        for choice in &view.choices {
            let bundle = observed
                .iter()
                .find(|bundle| bundle.file_ids.first() == Some(&choice.file_id))
                .ok_or_else(|| failed("Retained import bundle boundaries changed."))?;
            if let Some(id) = &choice.existing_group_id
                && !bundle.existing_groups.iter().any(|group| {
                    &group.id == id
                        && group.members.iter().all(|member| {
                            view.skip_files.iter().any(|file| {
                                file.file_id == member.file_id
                                    && file.keeper_asset_id == member.asset_id
                                    && file.keeper_root_id == group.root_id
                            })
                        })
                })
            {
                return Err(failed(
                    "Retained keeper locations no longer form the chosen complete native bundle.",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn validate_native(&self, catalog: &Arc<Catalog>) -> Result<()> {
        let source = self.source.open_scope(catalog, Access::Read)?;
        self.source.validate_read_roots(catalog)?;
        for member in &self.source.files {
            let file = member.location.open(source.directory(0).leaf())?;
            if !member.object.matches(&file)?
                || native::revision(&file)? != member.revision
                || file.metadata()?.len() != member.bytes
            {
                return Err(failed(
                    "A source member changed after comparison. Compare again before saving import decisions.",
                ));
            }
        }
        let mut scopes = Vec::new();
        for root in &self.roots {
            let expected = root.clone();
            catalog.call(move |db| {
                let (path, object, volume) =
                    crate::executor::sources::root(db, &expected.id, false)?;
                let (scan, availability): (Option<String>, String) = db.query_row(
                    "SELECT CASE WHEN length(last_scan)<=128 THEN last_scan END,availability FROM roots WHERE id=?",
                    [&expected.id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                if path != expected.native_path
                    || object != expected.object
                    || volume != expected.volume
                    || scan != expected.scan
                    || availability != "online"
                {
                    return Err(failed(
                        "The reviewed import scope changed or is unavailable. Compare again.",
                    ));
                }
                Ok(())
            })?;
            let chain =
                DirectoryChain::open(&native::decode_path(&root.native_path)?.canonicalize()?)?;
            if !root.object.matches(chain.leaf())?
                || native::volume_token(chain.leaf())? != root.volume
            {
                return Err(failed("Import scope changed native identity."));
            }
            scopes.push(Scope::new(chain, Access::Read));
        }
        let lease = Registry::shared().try_acquire(scopes)?;
        for entry in &self.keepers {
            let index = self
                .roots
                .iter()
                .position(|root| root.id == entry.root_id)
                .ok_or_else(|| failed("Import keeper is outside its reviewed roots."))?;
            entry.open(lease.directory(index).leaf())?;
        }
        Ok(())
    }
}
fn projected(
    db: &Connection,
    owner: &str,
    id: &str,
    digest: &str,
    mode: crate::library::model::RootMode,
) -> Result<Snapshot> {
    let stored = load(db, owner, id)?;
    if stored.view.digest != digest || stored.view.copy_file_ids.is_empty() {
        return Err(failed(
            "Select an immutable import plan with files to copy.",
        ));
    }
    let mut snapshot = stored.source;
    snapshot
        .files
        .retain(|file| stored.view.copy_file_ids.contains(&file.id));
    if snapshot.files.len() != stored.view.copy_file_ids.len() {
        return Err(failed(
            "Import copy decisions do not match the original source file IDs.",
        ));
    }
    snapshot.import_selection = Some(crate::executor::sources::ImportSelection {
        id: id.into(),
        digest: digest.into(),
        source_mode: mode,
    });
    Ok(snapshot)
}
pub(in crate::executor) fn validate_projection(db: &Connection, snapshot: &Snapshot) -> Result<()> {
    let selection = snapshot
        .import_selection
        .as_ref()
        .ok_or_else(|| failed("An import copy projection needs its exact retained decisions."))?;
    let expected = projected(
        db,
        &snapshot.owner_id,
        &selection.id,
        &selection.digest,
        selection.source_mode,
    )?;
    let expected = serde_json::to_vec(&expected)
        .map_err(|error| failed("Could not encode expected import copy scope.").caused_by(error))?;
    let actual = serde_json::to_vec(snapshot)
        .map_err(|error| failed("Could not encode import copy scope.").caused_by(error))?;
    if expected != actual {
        return Err(failed(
            "Import copy scope differs from its exact retained source/target subset.",
        ));
    }
    Ok(())
}
impl CopyWorkflow {
    pub(in crate::executor) fn import_copy_sources(
        &self,
        owner: String,
        id: String,
    ) -> Result<Snapshot> {
        let snapshot = self.catalog.call(move |db| {
            let stored = load(db, &owner, &id)?;
            let mode: String = db.query_row(
                "SELECT mode FROM roots WHERE id=?",
                [&stored.source.root_id],
                |row| row.get(0),
            )?;
            let mode = match mode.as_str() {
                "managed" => crate::library::model::RootMode::Managed,
                "referenced" => crate::library::model::RootMode::Referenced,
                _ => {
                    return Err(failed(
                        "The import source has no readable registered grant.",
                    ));
                }
            };
            let snapshot = projected(db, &owner, &id, &stored.view.digest, mode)?;
            snapshot.validate(db)?;
            Ok(snapshot)
        })?;
        let scope = snapshot.open_scope(&self.catalog, Access::Read)?;
        for member in &snapshot.files {
            let file = member.location.open(scope.directory(0).leaf())?;
            if !member.object.matches(&file)?
                || native::revision(&file)? != member.revision
                || file.metadata()?.len() != member.bytes
            {
                return Err(failed(
                    "An import copy source changed. Compare and review again.",
                ));
            }
        }
        Ok(snapshot)
    }
}
type Skips = Vec<(String, String, String)>;
fn resolve(
    bundles: &[ImportBundleObservation],
    choices: &[ImportBundleChoice],
) -> Result<(Vec<String>, Skips)> {
    if choices.len() != bundles.len() {
        return Err(failed(
            "Choose copy or an existing group for every whole bundle.",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut copy = Vec::new();
    let mut skip = Vec::new();
    for choice in choices {
        if !seen.insert(&choice.file_id) {
            return Err(failed("Choose each import bundle exactly once."));
        }
        let bundle = bundles
            .iter()
            .find(|bundle| bundle.file_ids.first() == Some(&choice.file_id))
            .ok_or_else(|| failed("An import decision belongs to another bundle."))?;
        if let Some(id) = &choice.existing_group_id {
            if bundle.disposition != ImportBundleDisposition::ReviewExistingBundle {
                return Err(failed(
                    "Only a completely observed existing bundle can be selected for fresh skip verification.",
                ));
            }
            let group = bundle
                .existing_groups
                .iter()
                .find(|group| group.id == *id)
                .ok_or_else(|| failed("The selected existing bundle was not observed."))?;
            if group.members.len() != bundle.file_ids.len()
                || group
                    .members
                    .iter()
                    .map(|file| &file.file_id)
                    .collect::<BTreeSet<_>>()
                    != bundle.file_ids.iter().collect::<BTreeSet<_>>()
                || group
                    .members
                    .iter()
                    .map(|file| &file.asset_id)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != group.members.len()
            {
                return Err(failed(
                    "An existing bundle does not cover every selected member distinctly.",
                ));
            }
            skip.extend(group.members.iter().map(|file| {
                (
                    file.file_id.clone(),
                    file.asset_id.clone(),
                    group.root_id.clone(),
                )
            }));
        } else {
            copy.extend(bundle.file_ids.iter().cloned());
        }
    }
    copy.sort();
    skip.sort();
    Ok((copy, skip))
}

impl CopyWorkflow {
    pub(crate) fn release_import_plan(&self, owner: String, id: String) -> Result<()> {
        let review = self
            .catalog
            .call(move |db| Ok(load(db, &owner, &id)?.view.id))?;
        self.approvals.revoke_review(&review)
    }
    pub(crate) fn import_plan_summaries(
        &self,
        owner: String,
        manifest: Option<String>,
    ) -> Result<Vec<ImportPlanSummary>> {
        validate_id(&owner)?;
        if let Some(id) = &manifest {
            validate_id(id)?;
        }
        self.catalog.call(move|db| {
            operation_store::load(db,&owner)?;
            if let Some(id)=&manifest {copy_manifest::load(db,&owner,id)?;}
            let ids=db.prepare("SELECT item_id FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND (?3 IS NULL OR json_extract(evidence,'$.view.manifestId')=?3) ORDER BY sequence DESC LIMIT 32")?.query_map(params![owner,PHASE,manifest],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            ids.iter().map(|id| {let view=load(db,&owner,id)?.view;Ok(ImportPlanSummary{id:view.id,manifest_id:view.manifest_id,digest:view.digest,created_at:view.created_at,copy_files:view.copy_file_ids.len() as u32,skip_files:view.skip_files.len() as u32,source_root_id:Some(view.source_root_id)})}).collect()
        })
    }
    pub(crate) fn create_import_plan(
        &self,
        library: &Library,
        mut input: ImportPlanInput,
    ) -> Result<ImportPlan> {
        library.ensure_work_admission()?;
        if input.choices.is_empty() || input.choices.len() > 512 {
            return Err(failed(
                "Choose one decision per bundle within the 512-file comparison bound.",
            ));
        }
        let (job, evidence) = self.import_plan_evidence(&input.comparison_job_id)?;
        let report = job
            .report
            .ok_or_else(|| failed("Compare the import scope again."))?;
        input.choices.sort_by(|a, b| a.file_id.cmp(&b.file_id));
        let (copy, skip) = resolve(&report.bundles, &input.choices)?;
        let mut skip_files = Vec::new();
        let mut keepers = Vec::new();
        for (source, asset, root) in skip {
            let found = report
                .files
                .iter()
                .find(|file| file.file_id == source)
                .and_then(|file| {
                    file.matches
                        .iter()
                        .find(|found| found.asset_id == asset && found.root_id == root)
                })
                .ok_or_else(|| failed("The chosen keeper lacks complete byte evidence."))?;
            let entry = evidence
                .keepers
                .iter()
                .find(|entry| entry.id == asset && entry.root_id == root)
                .ok_or_else(|| failed("The chosen keeper lacks native version evidence."))?;
            skip_files.push(ImportSkipFile {
                file_id: source,
                keeper_asset_id: asset,
                keeper_root_id: root,
                keeper_relative_path: found.relative_path.clone(),
                digest: found.digest.clone(),
                same_native_object: found.same_native_object,
            });
            if !keepers
                .iter()
                .any(|keeper: &Entry| keeper.id == entry.id && keeper.root_id == entry.root_id)
            {
                keepers.push(entry.clone());
            }
        }
        let mut stored = Stored {
            policy: "whole-bundle-import-plan-v1".into(),
            view: ImportPlan {
                id: uuid::Uuid::new_v4().to_string(),
                plan_id: job.plan_id,
                manifest_id: job.manifest_id,
                manifest_digest: evidence.source.manifest_digest.clone(),
                digest: String::new(),
                created_at: crate::library::store::now(),
                source_root_id: job.source_root_id,
                scope_root_ids: job.scope_root_ids,
                choices: input.choices,
                execution_available: copy.is_empty(),
                copy_file_ids: copy,
                skip_files,
                sources_retained: true,
            },
            source: evidence.source,
            roots: evidence.roots,
            keepers,
            bundles: report.bundles,
        };
        stored.view.digest = stored.digest()?;
        stored.validate_native(&self.catalog)?;
        library.ensure_work_admission()?;
        self.catalog.call(move|db| {
            let tx=db.transaction()?;stored.validate(&tx)?;
            let existing=tx.prepare("SELECT item_id FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND json_extract(evidence,'$.view.digest')=?3 LIMIT 2")?.query_map(params![stored.view.plan_id,PHASE,stored.view.digest],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            if existing.len()>1 {return Err(failed("Import decision history is ambiguous; no new record was created."));}
            if let Some(id)=existing.first() {return Ok(load(&tx,&stored.view.plan_id,id)?.view);}
            let count:i64=tx.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=?1 AND phase=?2",params![stored.view.plan_id,PHASE],|row|row.get(0))?;
            if count>=64 {return Err(failed("This draft already retains 64 import decision plans. Start a new reviewed draft."));}
            tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![stored.view.plan_id,stored.view.id,PHASE,encode(&stored)?,stored.view.created_at])?;
            tx.commit()?;Ok(stored.view)
        })
    }
    pub(crate) fn open_import_plan(&self, owner: String, id: String) -> Result<ImportPlan> {
        self.catalog.call(move |db| Ok(load(db, &owner, &id)?.view))
    }
}
pub(super) fn load(db: &Connection, owner: &str, id: &str) -> Result<Stored> {
    validate_id(owner)?;
    validate_id(id)?;
    let values=db.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=?4 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase=?3 LIMIT 2")?.query_map(params![owner,id,PHASE,MAX as i64],|row|row.get::<_,Option<String>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    if values.len() != 1 {
        return Err(failed("The retained import plan is missing or ambiguous."));
    }
    let text = values[0]
        .as_ref()
        .ok_or_else(|| failed("Retained import evidence exceeds 1 MiB."))?;
    let stored: Stored = serde_json::from_str(text)
        .map_err(|error| failed("Retained import evidence is invalid.").caused_by(error))?;
    if stored.view.id != id || stored.view.plan_id != owner {
        return Err(failed("Import evidence belongs to another plan."));
    }
    stored.validate(db)?;
    Ok(stored)
}
