//! Compile explicit, fresh companion decisions without changing originals or
//! granting execution authority. Private native locators remain backend owned.
use super::{Library, bundle_confirmation::StoredBundleConfirmation, operation_store, store};
use crate::{
    domain::{AppError, ErrorCode, Result},
    operations::{
        CopyManifest, CopyManifestFile, CopyManifestInput, MAX_PLAN_BYTES,
        copy_manifest::{ManifestFile, reconcile},
        validate_id,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredCopyManifest {
    pub policy: String,
    pub scope_key: String,
    pub view: CopyManifest,
    pub files: Vec<ManifestFile>,
    pub decision_fingerprints: Vec<String>,
    pub destination_fingerprint: String,
}
impl StoredCopyManifest {
    fn digest(&self) -> Result<String> {
        let mut content = self.clone();
        content.view.digest.clear();
        Ok(blake3::hash(
            &serde_json::to_vec(&content)
                .map_err(|e| invalid("Could not fingerprint copy manifest.").caused_by(e))?,
        )
        .to_hex()
        .to_string())
    }

    fn validate_for(&self, draft: &operation_store::StoredPlan) -> Result<()> {
        if self.policy != "copy-manifest-v1"
            || self.view.plan_id != draft.view.id
            || self.view.draft_digest != draft.view.digest
            || self.destination_fingerprint != draft.view.destination_fingerprint
            || self.view.execution_available
            || self.digest()? != self.view.digest
            || self.view.bundle_confirmation_ids.len() != draft.sources.len()
            || self.decision_fingerprints.len() != self.view.bundle_confirmation_ids.len()
        {
            return Err(changed());
        }
        let mut ids = self.view.bundle_confirmation_ids.clone();
        ids.sort();
        if ids != self.view.bundle_confirmation_ids || ids.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(changed());
        }
        for id in &ids {
            validate_id(id)?;
        }
        validate_id(&self.view.id)?;
        let key = blake3::hash(
            &serde_json::to_vec(&(&draft.view.digest, &ids, &self.decision_fingerprints))
                .map_err(|e| invalid("Could not validate manifest scope.").caused_by(e))?,
        )
        .to_hex()
        .to_string();
        if key != self.scope_key {
            return Err(changed());
        }
        let (files, issues, total) = reconcile(self.files.clone())?;
        if serde_json::to_vec(&files).ok() != serde_json::to_vec(&self.files).ok()
            || serde_json::to_vec(
                &files
                    .iter()
                    .map(|file| file.view.clone())
                    .collect::<Vec<_>>(),
            )
            .ok()
                != serde_json::to_vec(&self.view.files).ok()
            || serde_json::to_vec(&issues).ok() != serde_json::to_vec(&self.view.issues).ok()
            || total != self.view.selected_bytes
        {
            return Err(changed());
        }
        Ok(())
    }
}
fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
fn changed() -> AppError {
    AppError::new(
        ErrorCode::Conflict,
        "The draft or reviewed companions changed. Build a fresh plan and review the affected folders again.",
    )
}

impl Library {
    pub fn organization_copy_manifest_open(
        &self,
        plan_id: String,
        manifest_id: String,
    ) -> Result<CopyManifest> {
        validate_id(&plan_id)?;
        validate_id(&manifest_id)?;
        self.catalog
            .call(move |db| Ok(load(db, &plan_id, &manifest_id)?.view))
    }

    pub fn organization_copy_manifest(&self, input: CopyManifestInput) -> Result<CopyManifest> {
        validate_id(&input.plan_id)?;
        if input.bundle_confirmation_ids.is_empty()
            || input.bundle_confirmation_ids.len() > crate::organization::PREVIEW_LIMIT
        {
            return Err(invalid(
                "Choose one companion decision for each selected photo.",
            ));
        }
        let mut ids = input.bundle_confirmation_ids;
        for id in &ids {
            validate_id(id)?;
        }
        ids.sort();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid("Choose each companion decision only once."));
        }
        let plan_id = input.plan_id;
        let selected = ids.clone();
        let (draft, decisions)=self.catalog.call(move |db| {
            use rusqlite::OptionalExtension;
            let draft=operation_store::load(db,&plan_id)?;
            if selected.len()!=draft.sources.len() {return Err(invalid("Choose exactly one companion decision per photo in this draft."));}
            let mut decisions=Vec::new(); let mut assets=BTreeSet::new(); let mut retained_bytes=0usize;
            for id in selected {
                let text: Option<String>=db.query_row("SELECT evidence FROM operation_journal WHERE plan_id=?1 AND phase='bundle_confirmed' AND json_extract(evidence,'$.view.id')=?2 AND length(evidence)<=8388608",rusqlite::params![plan_id,id],|row|row.get(0)).optional()?;
                let text=text.ok_or_else(||invalid("A selected companion decision was not found in this draft."))?;
                retained_bytes=retained_bytes.saturating_add(text.len());
                if retained_bytes>MAX_PLAN_BYTES {return Err(invalid("Selected companion evidence exceeds 8 MiB. Use a smaller photo selection."));}
                let decision:StoredBundleConfirmation=serde_json::from_str(&text).map_err(|e|invalid("Retained companion evidence is invalid.").caused_by(e))?;
                decision.validate_for(&draft)?;
                if !assets.insert(decision.view.asset_id.clone()) {return Err(invalid("Choose only one decision for each selected photo."));}
                decisions.push(decision);
            }
            Ok((draft,decisions))
        })?;
        // No 'latest decision' lookup and no implicit rebasing of old records.
        let mut inputs = Vec::new();
        let mut decision_fingerprints = Vec::new();
        for decision in decisions {
            let observation = self.observe_bundle_inventory(
                decision.view.asset_id.clone(),
                decision.inventory.source_revision.clone(),
            )?;
            if observation.view.fingerprint != decision.view.inventory_fingerprint {
                return Err(changed());
            }
            for (entry, file) in decision.entries.iter().zip(&decision.view.files) {
                let observed = observation
                    .entries
                    .iter()
                    .find(|observed| observed.view.id == entry.view.id)
                    .ok_or_else(changed)?;
                if observed.relative_native != entry.relative_native
                    || observed.identity != entry.identity
                    || observed.revision != entry.revision
                {
                    return Err(changed());
                }
                inputs.push(ManifestFile {
                    view: CopyManifestFile {
                        id: String::new(),
                        root_id: decision.source.root_id.clone(),
                        source_name: entry.view.name.clone(),
                        source_revision: entry.revision.clone().ok_or_else(changed)?,
                        target_relative_path: file.target_relative_path.clone(),
                        bytes: file.bytes.clone(),
                        bundle_asset_ids: vec![decision.view.asset_id.clone()],
                    },
                    relative_native: entry.relative_native.clone(),
                    identity: entry.identity.ok_or_else(changed)?,
                });
            }
            decision_fingerprints.push(decision.view.fingerprint);
        }
        let (files, issues, selected_bytes) = reconcile(inputs)?;
        let current = self.organization_preview(
            draft
                .sources
                .iter()
                .map(|source| source.id.clone())
                .collect(),
            draft.view.preview.template.clone(),
        )?;
        if current.fingerprint != draft.view.preview.fingerprint {
            return Err(changed());
        }
        let scope_key = blake3::hash(
            &serde_json::to_vec(&(&draft.view.digest, &ids, &decision_fingerprints))
                .map_err(|e| invalid("Could not fingerprint manifest scope.").caused_by(e))?,
        )
        .to_hex()
        .to_string();
        let mut record = StoredCopyManifest {
            policy: "copy-manifest-v1".into(),
            scope_key,
            view: CopyManifest {
                id: uuid::Uuid::new_v4().to_string(),
                plan_id: draft.view.id.clone(),
                draft_digest: draft.view.digest.clone(),
                digest: String::new(),
                created_at: store::now(),
                bundle_confirmation_ids: ids,
                files: files.iter().map(|file| file.view.clone()).collect(),
                issues,
                selected_bytes,
                execution_available: false,
            },
            files,
            decision_fingerprints,
            destination_fingerprint: draft.view.destination_fingerprint,
        };
        record.view.digest = record.digest()?;
        let revision = current.catalog_revision;
        self.catalog.call(move |db| {
            let transaction=db.transaction()?;
            let actual:i64=transaction.query_row("SELECT revision FROM library_state WHERE singleton=1",[],|row|row.get(0))?;
            if actual.to_string()!=revision {return Err(changed());}
            use rusqlite::OptionalExtension;
            let existing:Option<String>=transaction.query_row("SELECT evidence FROM operation_journal WHERE plan_id=?1 AND phase='copy_manifest_compiled' AND json_extract(evidence,'$.scope_key')=?2 AND length(evidence)<=8388608",rusqlite::params![record.view.plan_id,record.scope_key],|row|row.get(0)).optional()?;
            if let Some(existing)=existing {
                if existing.len()>MAX_PLAN_BYTES {return Err(invalid("Retained manifest exceeds 8 MiB."));}
                let existing:StoredCopyManifest=serde_json::from_str(&existing).map_err(|e|invalid("Retained copy manifest is invalid.").caused_by(e))?;
                if existing.policy!=record.policy || existing.scope_key!=record.scope_key || existing.view.execution_available || existing.view.draft_digest!=record.view.draft_digest
                    || existing.view.plan_id!=record.view.plan_id || existing.digest()?!=existing.view.digest
                    || serde_json::to_vec(&existing.files).ok()!=serde_json::to_vec(&record.files).ok()
                    || serde_json::to_vec(&existing.view.files).ok()!=serde_json::to_vec(&record.view.files).ok()
                    || serde_json::to_vec(&existing.view.issues).ok()!=serde_json::to_vec(&record.view.issues).ok()
                    || existing.view.selected_bytes!=record.view.selected_bytes
                    || existing.view.bundle_confirmation_ids!=record.view.bundle_confirmation_ids
                    || existing.decision_fingerprints!=record.decision_fingerprints
                    || existing.destination_fingerprint!=record.destination_fingerprint {
                    return Err(changed());
                }
                return Ok(existing.view);
            }
            let count:u32=transaction.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=? AND phase='copy_manifest_compiled'",[&record.view.plan_id],|row|row.get(0))?;
            if count>=64 {return Err(invalid("This draft already retains 64 compiled manifests. Create a new draft to change scope."));}
            let evidence=serde_json::to_string(&record).map_err(|e|invalid("Could not retain the copy manifest.").caused_by(e))?;
            if evidence.len()>MAX_PLAN_BYTES {return Err(invalid("The compiled manifest exceeds 8 MiB. Use a smaller photo selection."));}
            transaction.execute("INSERT INTO operation_journal(plan_id,phase,evidence,created_at) VALUES(?1,'copy_manifest_compiled',?2,?3)",rusqlite::params![record.view.plan_id,evidence,record.view.created_at])?;
            transaction.commit()?;
            Ok(record.view)
        })
    }
}

pub(super) fn load(
    db: &mut rusqlite::Connection,
    plan_id: &str,
    manifest_id: &str,
) -> Result<StoredCopyManifest> {
    let draft = operation_store::load(db, plan_id)?;
    let mut statement=db.prepare("SELECT evidence FROM operation_journal WHERE plan_id=?1 AND phase='copy_manifest_compiled' AND json_extract(evidence,'$.view.id')=?2 AND length(evidence)<=8388608 LIMIT 2")?;
    let mut rows = statement.query(rusqlite::params![plan_id, manifest_id])?;
    let text: String = rows
        .next()?
        .ok_or_else(|| invalid("The compiled manifest was not found in this draft."))?
        .get(0)?;
    if rows.next()?.is_some() || text.len() > MAX_PLAN_BYTES {
        return Err(invalid(
            "Retained copy manifest has ambiguous identity or exceeds 8 MiB.",
        ));
    }
    let record: StoredCopyManifest = serde_json::from_str(&text)
        .map_err(|e| invalid("Retained copy manifest is invalid.").caused_by(e))?;
    record.validate_for(&draft)?;
    if record.view.id != manifest_id {
        return Err(changed());
    }
    Ok(record)
}
