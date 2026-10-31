//! Append-only companion decisions bound to a retained draft and a fresh full
//! filename inventory. These records are not approvals to execute an operation.
use super::{
    Library,
    bundle_inventory::{InventoryObservation, NativeBundleEntry},
    operation_store, store,
};
use crate::{
    domain::{AppError, ErrorCode, Result},
    operations::{
        BundleConfirmation, BundleConfirmationInput, BundleEntryKind, BundleInventory,
        BundleRelation, ConfirmedBundleFile, MAX_PLAN_BYTES, validate_id,
    },
    organization::{self, PreviewSource},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StoredBundleConfirmation {
    pub policy: String,
    pub view: BundleConfirmation,
    pub idempotency_key: String,
    pub draft_digest: String,
    pub catalog_revision: String,
    pub source: PreviewSource,
    pub entries: Vec<NativeBundleEntry>,
    pub inventory: BundleInventory,
    pub reviewed_all_entries: bool,
}

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
fn conflict(message: &str) -> AppError {
    AppError::new(ErrorCode::Conflict, message)
}

impl StoredBundleConfirmation {
    pub(super) fn digest(&self) -> Result<String> {
        let mut value = self.clone();
        value.view.fingerprint.clear();
        Ok(blake3::hash(
            &serde_json::to_vec(&value)
                .map_err(|e| invalid("Could not fingerprint companion review.").caused_by(e))?,
        )
        .to_hex()
        .to_string())
    }

    pub(super) fn validate_for(&self, draft: &operation_store::StoredPlan) -> Result<()> {
        let row = draft
            .view
            .preview
            .rows
            .iter()
            .find(|row| row.asset_id == self.view.asset_id)
            .ok_or_else(|| conflict("The companion decision does not belong to this draft."))?;
        if self.policy != "bundle-confirmation-v1"
            || self.view.plan_id != draft.view.id
            || self.view.execution_available
            || self.inventory.execution_available
            || self.draft_digest != draft.view.digest
            || !self.reviewed_all_entries
            || !self.inventory.inventory_complete
            || self.inventory.truncated
            || self.inventory.indexed_limit_exceeded
            || !self.inventory.unobserved_candidates.is_empty()
            || self.inventory.entries.len() > crate::operations::MAX_BUNDLE_DIRECTORY_ENTRIES
            || self.inventory.fingerprint != self.view.inventory_fingerprint
            || self.inventory.asset_id != self.view.asset_id
            || self.inventory.source_revision != row.source_revision
            || self.catalog_revision != self.inventory.catalog_revision
            || self.catalog_revision != draft.view.preview.catalog_revision
            || self.digest()? != self.view.fingerprint
            || self.entries.is_empty()
            || self.entries.len() > 65
            || self.entries.len() != self.view.files.len()
        {
            return Err(conflict(
                "Retained companion review failed its identity or fingerprint check.",
            ));
        }
        let planned = draft
            .sources
            .iter()
            .find(|source| source.id == self.view.asset_id)
            .ok_or_else(|| conflict("The companion decision has no matching source."))?;
        let mut source = self.source.clone();
        source.companion_candidates = planned.companion_candidates;
        if serde_json::to_vec(&source).ok() != serde_json::to_vec(planned).ok() {
            return Err(conflict(
                "The companion source does not match its retained draft.",
            ));
        }
        let primary = row
            .target_relative_path
            .as_deref()
            .filter(|_| row.issues.is_empty())
            .ok_or_else(|| conflict("The retained draft source is blocked."))?;
        let selected: BTreeSet<_> = self
            .entries
            .iter()
            .map(|entry| entry.view.id.clone())
            .collect();
        if selected.len() != self.entries.len()
            || self.inventory.entries.iter().any(|entry| {
                !matches!(
                    entry.relation,
                    BundleRelation::Primary | BundleRelation::Additional
                ) && !selected.contains(&entry.id)
            })
        {
            return Err(conflict(
                "Retained companion review omits a possible companion.",
            ));
        }
        for entry in &self.entries {
            let observed = self
                .inventory
                .entries
                .iter()
                .find(|candidate| candidate.id == entry.view.id)
                .ok_or_else(|| {
                    conflict("The companion selection is outside its reviewed inventory.")
                })?;
            if serde_json::to_vec(observed).ok() != serde_json::to_vec(&entry.view).ok() {
                return Err(conflict(
                    "The companion selection differs from its reviewed inventory.",
                ));
            }
        }
        let observation = InventoryObservation {
            view: self.inventory.clone(),
            source: self.source.clone(),
            entries: self.entries.clone(),
        };
        let chosen = self
            .entries
            .iter()
            .filter(|entry| entry.view.relation != BundleRelation::Primary)
            .map(|entry| entry.view.id.clone())
            .collect();
        let (_, files, total) = selected_files(&observation, &chosen, primary)?;
        if total != self.view.total_bytes
            || serde_json::to_vec(&files).ok() != serde_json::to_vec(&self.view.files).ok()
        {
            return Err(conflict("Retained companion targets failed validation."));
        }
        Ok(())
    }
}

fn target(primary: &str, name: &str, relation: &BundleRelation) -> Result<String> {
    let (folder, filename) = primary
        .rsplit_once('/')
        .map_or(("", primary), |(parent, file)| (parent, file));
    let stem = filename.rsplit_once('.').map_or(filename, |(stem, _)| stem);
    let extension = Path::new(name).extension().and_then(|value| value.to_str());
    let leaf = match relation {
        BundleRelation::Primary => filename.to_owned(),
        BundleRelation::SameStem => {
            extension.map_or_else(|| stem.to_owned(), |ext| format!("{stem}.{ext}"))
        }
        BundleRelation::FilenameSidecar => format!(
            "{filename}.{}",
            extension.ok_or_else(|| invalid("A filename sidecar needs its original extension."))?
        ),
        BundleRelation::AppleDouble => format!("._{filename}"),
        BundleRelation::Additional => name.to_owned(),
    };
    let path = if folder.is_empty() {
        leaf
    } else {
        format!("{folder}/{leaf}")
    };
    if !organization::portable_relative_target(&path) {
        return Err(invalid(
            "A companion target violates the portable naming policy. No files were changed.",
        ));
    }
    Ok(path)
}

fn selected_files(
    observation: &InventoryObservation,
    selected: &BTreeSet<String>,
    primary: &str,
) -> Result<(Vec<NativeBundleEntry>, Vec<ConfirmedBundleFile>, String)> {
    if !observation.view.inventory_complete {
        return Err(conflict(
            "Complete the folder inventory before confirming companions. Reconnect unavailable files or resolve incomplete observations.",
        ));
    }
    if !observation.entries.iter().any(|entry| {
        entry.view.relation == BundleRelation::Primary && entry.view.kind == BundleEntryKind::File
    }) {
        return Err(conflict(
            "The selected photo was not present in the complete folder inventory.",
        ));
    }
    for entry in &observation.entries {
        if !matches!(
            entry.view.relation,
            BundleRelation::Primary | BundleRelation::Additional
        ) && (!entry.view.selectable || !selected.contains(&entry.view.id))
        {
            return Err(invalid(
                "Include every possible same-stem, filename-sidecar and AppleDouble companion. Unsupported or ambiguous companions block confirmation.",
            ));
        }
    }
    for id in selected {
        if !observation
            .entries
            .iter()
            .any(|entry| &entry.view.id == id && entry.view.selectable)
        {
            return Err(invalid(
                "The companion selection contains an unknown, changed or unsupported entry.",
            ));
        }
    }
    let mut entries = Vec::new();
    let mut files = Vec::new();
    let mut total = 0u128;
    let mut targets = BTreeMap::new();
    for entry in &observation.entries {
        if entry.view.relation != BundleRelation::Primary && !selected.contains(&entry.view.id) {
            continue;
        }
        let path = target(primary, &entry.view.name, &entry.view.relation)?;
        if targets.keys().any(|previous: &String| {
            previous.eq_ignore_ascii_case(&path)
                || previous
                    .strip_prefix(&path)
                    .is_some_and(|tail| tail.starts_with('/'))
                || path
                    .strip_prefix(previous)
                    .is_some_and(|tail| tail.starts_with('/'))
        }) {
            return Err(invalid(
                "Companion targets collide. Build a different organization preview before confirming this bundle.",
            ));
        }
        targets.insert(path.clone(), ());
        let bytes = entry
            .view
            .bytes
            .clone()
            .ok_or_else(|| invalid("The companion byte count is unavailable."))?;
        total += u128::from(
            bytes
                .parse::<u64>()
                .map_err(|_| invalid("Invalid companion byte count."))?,
        );
        files.push(ConfirmedBundleFile {
            entry_id: entry.view.id.clone(),
            source_name: entry.view.name.clone(),
            target_relative_path: path,
            relation: entry.view.relation.clone(),
            bytes,
        });
        entries.push(entry.clone());
    }
    Ok((entries, files, total.to_string()))
}

impl Library {
    pub fn organization_bundle_reviews(
        &self,
        plan_id: String,
        asset_id: String,
    ) -> Result<Vec<BundleConfirmation>> {
        validate_id(&plan_id)?;
        validate_id(&asset_id)?;
        self.catalog.call(move |db| {
            let draft = operation_store::load(db, &plan_id)?;
            if !draft.sources.iter().any(|source| source.id == asset_id) { return Err(invalid("This photo is not part of the retained plan.")); }
            let mut statement = db.prepare("SELECT evidence FROM operation_journal WHERE plan_id=?1 AND phase='bundle_confirmed' AND json_extract(evidence,'$.view.assetId')=?2 ORDER BY sequence DESC LIMIT 17")?;
            let rows = statement.query_map(rusqlite::params![plan_id, asset_id], |r| r.get::<_, String>(0))?;
            let mut result = Vec::new();
            for row in rows {
                let text = row?;
                if text.len() > MAX_PLAN_BYTES { return Err(invalid("Retained companion review exceeds its size limit.")); }
                let record: StoredBundleConfirmation = serde_json::from_str(&text).map_err(|e| invalid("Retained companion review is invalid.").caused_by(e))?;
                record.validate_for(&draft)?;
                if record.view.asset_id != asset_id { return Err(conflict("The retained review belongs to a different photo.")); }
                result.push(record.view);
            }
            if result.len() > 16 { return Err(invalid("Retained companion reviews exceed the supported count.")); }
            Ok(result)
        })
    }

    pub fn organization_bundle_confirm(
        &self,
        input: BundleConfirmationInput,
    ) -> Result<BundleConfirmation> {
        validate_id(&input.plan_id)?;
        validate_id(&input.asset_id)?;
        if !input.reviewed_all_entries
            || input.companion_ids.len() > 64
            || input.inventory_fingerprint.len() != 64
            || !input
                .inventory_fingerprint
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            || input
                .companion_ids
                .iter()
                .any(|id| id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(invalid(
                "Review all folder filenames and select at most 64 companion files before confirming.",
            ));
        }
        let selected: BTreeSet<_> = input.companion_ids.iter().cloned().collect();
        if selected.len() != input.companion_ids.len() {
            return Err(invalid("Select each companion only once."));
        }
        let plan_id = input.plan_id.clone();
        let draft = self
            .catalog
            .call(move |db| operation_store::load(db, &plan_id))?;
        let row = draft
            .view
            .preview
            .rows
            .iter()
            .find(|row| row.asset_id == input.asset_id)
            .ok_or_else(|| invalid("This photo is not part of the retained plan."))?;
        if row.source_revision != input.source_revision || !row.issues.is_empty() {
            return Err(conflict(
                "Build a fresh valid plan before confirming companions.",
            ));
        }
        let primary = row
            .target_relative_path
            .as_deref()
            .ok_or_else(|| invalid("The selected photo has no resolved target."))?;
        let observation =
            self.observe_bundle_inventory(input.asset_id.clone(), input.source_revision)?;
        let planned_source = draft
            .sources
            .iter()
            .find(|source| source.id == input.asset_id)
            .ok_or_else(|| {
                conflict("The retained draft has no matching native source evidence.")
            })?;
        let mut observed_source = observation.source.clone();
        observed_source.companion_candidates = planned_source.companion_candidates;
        if serde_json::to_vec(&observed_source).ok() != serde_json::to_vec(planned_source).ok() {
            return Err(conflict(
                "The native source evidence changed. Build a fresh saved plan.",
            ));
        }
        if observation.view.fingerprint != input.inventory_fingerprint
            || observation.view.catalog_revision != draft.view.preview.catalog_revision
        {
            return Err(conflict(
                "The retained plan or folder inventory changed. Build a fresh plan and review companions again.",
            ));
        }
        let (entries, files, total_bytes) = selected_files(&observation, &selected, primary)?;
        let key = serde_json::to_vec(&(
            &input.plan_id,
            &draft.view.digest,
            &input.asset_id,
            &input.inventory_fingerprint,
            &selected,
        ))
        .map_err(|e| invalid("Could not fingerprint the companion selection.").caused_by(e))?;
        let mut record = StoredBundleConfirmation {
            policy: "bundle-confirmation-v1".into(),
            view: BundleConfirmation {
                id: uuid::Uuid::new_v4().to_string(),
                plan_id: input.plan_id,
                asset_id: input.asset_id,
                inventory_fingerprint: input.inventory_fingerprint,
                fingerprint: String::new(),
                files,
                total_bytes,
                execution_available: false,
            },
            idempotency_key: blake3::hash(&key).to_hex().to_string(),
            draft_digest: draft.view.digest,
            catalog_revision: observation.view.catalog_revision.clone(),
            source: observation.source,
            entries,
            inventory: observation.view,
            reviewed_all_entries: true,
        };
        record.view.fingerprint = record.digest()?;
        self.catalog.call(move |db| {
            let transaction = db.transaction()?;
            let revision: i64 = transaction.query_row("SELECT revision FROM library_state WHERE singleton=1", [], |r| r.get(0))?;
            if revision.to_string() != record.catalog_revision { return Err(conflict("The catalog changed before companion confirmation. Review it again.")); }
            use rusqlite::OptionalExtension;
            let retained: Option<String> = transaction.query_row("SELECT evidence FROM operation_journal WHERE plan_id=?1 AND phase='bundle_confirmed' AND json_extract(evidence,'$.idempotencyKey')=?2 LIMIT 1",
                rusqlite::params![record.view.plan_id, record.idempotency_key], |r| r.get(0)).optional()?;
            if let Some(retained) = retained {
                if retained.len() > MAX_PLAN_BYTES { return Err(invalid("Retained companion review exceeds its size limit.")); }
                let existing: StoredBundleConfirmation = serde_json::from_str(&retained).map_err(|e| invalid("Retained companion review is invalid.").caused_by(e))?;
                if existing.policy != "bundle-confirmation-v1" || !existing.reviewed_all_entries || !existing.inventory.inventory_complete
                    || existing.inventory.fingerprint != existing.view.inventory_fingerprint
                    || existing.digest()? != existing.view.fingerprint || existing.idempotency_key != record.idempotency_key { return Err(conflict("Retained companion review failed its fingerprint check.")); }
                return Ok(existing.view);
            }
            let retained_count: u32 = transaction.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=?1 AND phase='bundle_confirmed' AND json_extract(evidence,'$.view.assetId')=?2",
                rusqlite::params![record.view.plan_id, record.view.asset_id], |r| r.get(0))?;
            if retained_count >= 16 { return Err(invalid("This draft already retains 16 companion decisions for this photo. Create a new draft to review a different selection.")); }
            let text = serde_json::to_string(&record).map_err(|e| invalid("Could not retain companion review.").caused_by(e))?;
            if text.len() > MAX_PLAN_BYTES { return Err(invalid("The confirmed companion evidence exceeds 8 MiB. Use a smaller selection.")); }
            transaction.execute("INSERT INTO operation_journal(plan_id,phase,evidence,created_at) VALUES(?1,'bundle_confirmed',?2,?3)", rusqlite::params![record.view.plan_id, text, store::now()])?;
            transaction.commit()?;
            Ok(record.view)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        filesystem,
        library::{bundle_inventory::tests::fixture, native},
        operations::PlanDraftInput,
        organization::{MissingDatePolicy, OrganizationTemplate},
    };

    #[test]
    fn companion_targets_preserve_variant_sidecar_and_appledouble_associations() {
        let primary = "2026/001_Photo.JPG";
        for (name, relation, expected) in [
            ("Photo.CR2", BundleRelation::SameStem, "2026/001_Photo.CR2"),
            (
                "Photo.JPG.XMP",
                BundleRelation::FilenameSidecar,
                "2026/001_Photo.JPG.XMP",
            ),
            (
                "._Photo.JPG",
                BundleRelation::AppleDouble,
                "2026/._001_Photo.JPG",
            ),
            ("notes.txt", BundleRelation::Additional, "2026/notes.txt"),
        ] {
            assert_eq!(target(primary, name, &relation).unwrap(), expected);
        }
        assert!(target(primary, "../escape", &BundleRelation::Additional).is_err());
    }

    #[test]
    fn companion_confirmation_requires_full_inventory_and_retains_immutable_retryable_decisions() {
        let fixture = fixture();
        for name in ["Photo.xmp", "Photo.JPG.AAE", "._Photo.JPG", "Other.bin"] {
            std::fs::write(fixture.source.join(name), name.as_bytes()).unwrap();
        }
        let destination = fixture.run.path().join("destination");
        std::fs::create_dir(&destination).unwrap();
        let template = OrganizationTemplate {
            pattern: "organized/{sequence:03}_{original_stem}.{ext}".into(),
            event: String::new(),
            missing_date: MissingDatePolicy::Block,
        };
        let preview = fixture
            .library
            .organization_preview(vec![fixture.asset_id.clone()], template.clone())
            .unwrap();
        let handle = native::open_root(&destination, None).unwrap();
        let expected = (
            filesystem::identity(&handle).unwrap(),
            native::volume_token(&handle).unwrap(),
        );
        drop(handle);
        let draft = fixture
            .library
            .organization_plan_create(
                PlanDraftInput {
                    asset_ids: vec![fixture.asset_id.clone()],
                    template,
                    preview_fingerprint: preview.fingerprint,
                    template_reference: None,
                },
                destination.clone(),
                expected,
            )
            .unwrap();
        let inventory = fixture
            .library
            .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
            .unwrap();
        let companions = inventory
            .entries
            .iter()
            .filter(|entry| entry.selectable && entry.relation != BundleRelation::Additional)
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>();
        let mut input = BundleConfirmationInput {
            plan_id: draft.plan.id.clone(),
            asset_id: fixture.asset_id.clone(),
            source_revision: fixture.revision.clone(),
            inventory_fingerprint: inventory.fingerprint.clone(),
            companion_ids: companions.clone(),
            reviewed_all_entries: false,
        };
        assert!(
            fixture
                .library
                .organization_bundle_confirm(input.clone())
                .is_err()
        );
        input.reviewed_all_entries = true;
        input.companion_ids.pop();
        assert!(
            fixture
                .library
                .organization_bundle_confirm(input.clone())
                .is_err()
        );
        input.companion_ids = companions;
        let confirmed = fixture
            .library
            .organization_bundle_confirm(input.clone())
            .unwrap();
        assert_eq!(confirmed.files.len(), 4);
        assert!(!confirmed.execution_available);
        assert!(
            confirmed
                .files
                .iter()
                .any(|file| file.target_relative_path == "organized/001_Photo.JPG.AAE")
        );
        let retry = fixture
            .library
            .organization_bundle_confirm(input.clone())
            .unwrap();
        assert_eq!(retry.id, confirmed.id);
        assert_eq!(retry.fingerprint, confirmed.fingerprint);
        let history = fixture
            .library
            .organization_bundle_reviews(draft.plan.id.clone(), fixture.asset_id.clone())
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].fingerprint, confirmed.fingerprint);
        let manifest_input = crate::operations::CopyManifestInput {
            plan_id: draft.plan.id.clone(),
            bundle_confirmation_ids: vec![confirmed.id.clone()],
        };
        let manifest = fixture
            .library
            .organization_copy_manifest(manifest_input.clone())
            .unwrap();
        assert_eq!(manifest.files.len(), 4);
        assert!(manifest.issues.is_empty());
        assert!(!manifest.execution_available);
        assert_eq!(manifest.selected_bytes, confirmed.total_bytes);
        assert_eq!(
            manifest.bundle_confirmation_ids,
            std::slice::from_ref(&confirmed.id)
        );
        assert_eq!(
            fixture
                .library
                .organization_copy_manifest(manifest_input.clone())
                .unwrap()
                .id,
            manifest.id
        );
        let mut invalid = manifest_input.clone();
        invalid.bundle_confirmation_ids.push(confirmed.id.clone());
        assert!(fixture.library.organization_copy_manifest(invalid).is_err());
        std::fs::write(fixture.source.join("Photo.xmp"), b"changed companion").unwrap();
        assert!(
            fixture
                .library
                .organization_copy_manifest(manifest_input)
                .is_err()
        );
        let retained = fixture
            .library
            .organization_copy_manifest_open(draft.plan.id.clone(), manifest.id.clone())
            .unwrap();
        assert_eq!(retained.digest, manifest.digest);
        assert!(!retained.execution_available);
        input.companion_ids.push("0".repeat(64));
        assert!(fixture.library.organization_bundle_confirm(input).is_err());
        assert!(
            !fixture
                .library
                .organization_plan(draft.plan.id)
                .unwrap()
                .plan
                .execution_available
        );
        assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 5);
        assert_eq!(std::fs::read_dir(destination).unwrap().count(), 0);
        assert_eq!(
            fixture
                .library
                .organization_bundle_reviews(confirmed.plan_id, uuid::Uuid::new_v4().to_string())
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
