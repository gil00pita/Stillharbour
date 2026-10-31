//! An effect is the native source/policy/target mapping, independently of draft,
//! ticket, namespace UUID, creation time or unrelated catalog revision.
use super::{Object, failed, locations, sources::Snapshot};
#[cfg(any(test, feature = "copy-workflow"))]
use crate::catalog::Catalog;
use crate::{
    domain::Result,
    library::{copy_manifest, operation_store},
    organization::OrganizationTemplate,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
#[cfg(any(test, feature = "copy-workflow"))]
use std::sync::Arc;

const GROUP: &str = "copy_effect_group_v1";
const CANDIDATE: &str = "copy_effect_candidate_v1";
const MAX: usize = 65536;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    location: locations::Source,
    object: Object,
    revision: String,
    bytes: u64,
    target: String,
    bundle_asset_ids: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    policy: String,
    catalog_id: String,
    root_id: String,
    source: Object,
    source_volume: String,
    destination: Object,
    destination_volume: String,
    template: OrganizationTemplate,
    files: Vec<File>,
}
#[derive(Clone)]
pub(super) struct Effect {
    pub digest: String,
    encoded: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Candidate {
    policy: String,
    pub effect_digest: String,
    pub owner_id: String,
    pub manifest_id: String,
    pub token: String,
    pub namespace: String,
    pub scope_digest: String,
}
fn encoded<T: Serialize>(value: &T) -> Result<String> {
    let text = serde_json::to_string(value)
        .map_err(|error| failed("Could not encode copy effect.").caused_by(error))?;
    if text.len() > MAX {
        return Err(failed(
            "Copy effect exceeds 64 KiB. Use a smaller selection.",
        ));
    }
    Ok(text)
}
fn hash(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}
impl Effect {
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn lookup(
        catalog: &Arc<Catalog>,
        snapshot: &Snapshot,
    ) -> Result<(Self, Vec<Candidate>)> {
        let snapshot = snapshot.clone();
        catalog.call(move |db| {
            let tx = db.transaction()?;
            let effect = Self::capture(&tx, &snapshot)?;
            let seeds =
                super::catalog_journal::namespace_journal::legacy_effect_seeds(&tx, &snapshot)?;
            for seed in seeds {
                let legacy = Self::capture(&tx, &seed.sources)?;
                legacy.insert_candidate(
                    &tx,
                    &seed.sources,
                    &seed.token,
                    &seed.namespace,
                    &seed.digest,
                )?;
            }
            let candidates = effect.candidates(&tx)?;
            tx.commit()?;
            Ok((effect, candidates))
        })
    }
    pub(super) fn capture(db: &Connection, snapshot: &Snapshot) -> Result<Self> {
        snapshot.validate(db)?;
        let draft = operation_store::load(db, &snapshot.owner_id)?;
        let manifest = copy_manifest::load(db, &snapshot.owner_id, &snapshot.manifest_id)?;
        let files = snapshot
            .files
            .iter()
            .zip(&manifest.files)
            .map(|(file, member)| File {
                location: file.location.clone(),
                object: file.object.clone(),
                revision: file.revision.clone(),
                bytes: file.bytes,
                target: file.target.clone(),
                bundle_asset_ids: member.view.bundle_asset_ids.clone(),
            })
            .collect();
        let definition = Definition {
            policy: "source-retaining-copy-effect-v1".into(),
            catalog_id: snapshot.catalog_id.clone(),
            root_id: snapshot.root_id.clone(),
            source: snapshot.source.clone(),
            source_volume: snapshot.source_volume.clone(),
            destination: snapshot.destination.clone(),
            destination_volume: snapshot.destination_volume.clone(),
            template: draft.view.preview.template,
            files,
        };
        let text = encoded(&definition)?;
        Ok(Self {
            digest: hash(&text),
            encoded: text,
        })
    }
    fn check_group(&self, db: &Connection) -> Result<bool> {
        let text:Option<Option<String>>=db.query_row("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE phase='copy_effect_group_v1' AND item_id=?",[&self.digest],|row|row.get(0)).optional()?;
        let Some(text) = text else {
            return Ok(false);
        };
        let text = text.ok_or_else(|| failed("Retained effect group exceeds its bound."))?;
        let definition: Definition = serde_json::from_str(&text)
            .map_err(|error| failed("Invalid retained effect definition.").caused_by(error))?;
        if encoded(&definition)? != self.encoded || hash(&text) != self.digest {
            return Err(failed("Retained effect digest or native mapping differs."));
        }
        Ok(true)
    }
    pub(super) fn candidates(&self, db: &Connection) -> Result<Vec<Candidate>> {
        if !self.check_group(db)? {
            return Ok(Vec::new());
        }
        let mut query=db.prepare("SELECT plan_id,item_id,CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE phase='copy_effect_candidate_v1' AND json_extract(evidence,'$.effect_digest')=? ORDER BY sequence LIMIT 65")?;
        let mut rows = query.query([&self.digest])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            let text: Option<String> = row.get(2)?;
            let candidate: Candidate = serde_json::from_str(
                &text.ok_or_else(|| failed("Retained effect candidate exceeds its bound."))?,
            )
            .map_err(|error| failed("Invalid retained effect candidate.").caused_by(error))?;
            if candidate.policy != "copy-effect-candidate-v1"
                || candidate.effect_digest != self.digest
                || candidate.owner_id != row.get::<_, String>(0)?
                || candidate.token != row.get::<_, String>(1)?
            {
                return Err(failed(
                    "Effect candidate differs from its immutable anchor.",
                ));
            }
            crate::operations::validate_id(&candidate.owner_id)?;
            crate::operations::validate_id(&candidate.manifest_id)?;
            crate::operations::validate_id(&candidate.token)?;
            if candidate.scope_digest.len() != 64
                || !candidate
                    .scope_digest
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
            {
                return Err(failed("Invalid effect namespace digest."));
            }
            super::preparation::validate_namespace(&candidate.token, &candidate.namespace)?;
            let (seed_sources, seed_namespace) =
                super::catalog_journal::namespace_journal::effect_seed(
                    db,
                    &candidate.owner_id,
                    &candidate.token,
                    &candidate.scope_digest,
                )?;
            let actual = Self::capture(db, &seed_sources)?;
            if seed_sources.manifest_id != candidate.manifest_id
                || seed_namespace != candidate.namespace
                || actual.encoded != self.encoded
                || actual.digest != self.digest
            {
                return Err(failed(
                    "Effect candidate does not match its exact retained source/template mapping.",
                ));
            }
            result.push(candidate);
            if result.len() > 64 {
                return Err(failed(
                    "More than 64 retained folders match this effect. Retain all history for explicit review.",
                ));
            }
        }
        if result.is_empty() {
            return Err(failed("Effect group has no retained namespace candidate."));
        }
        Ok(result)
    }
    pub(super) fn register(
        &self,
        db: &Connection,
        snapshot: &Snapshot,
        token: &str,
        namespace: &str,
        scope_digest: &str,
    ) -> Result<()> {
        let previous = self.candidates(db)?;
        if previous.iter().any(|candidate| {
            candidate.owner_id == snapshot.owner_id
                && candidate.manifest_id == snapshot.manifest_id
                && candidate.token == token
                && candidate.namespace == namespace
                && candidate.scope_digest == scope_digest
        }) {
            return Ok(());
        }
        if !previous.is_empty() {
            return Err(failed(
                "This exact source/template mapping has a retained output folder. Review its existing operation instead of allocating another.",
            ));
        }
        self.insert_candidate(db, snapshot, token, namespace, scope_digest)
    }
    fn insert_candidate(
        &self,
        db: &Connection,
        snapshot: &Snapshot,
        token: &str,
        namespace: &str,
        scope_digest: &str,
    ) -> Result<()> {
        if !self.check_group(db)? {
            db.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![snapshot.owner_id,self.digest,GROUP,self.encoded,crate::library::store::now()])?;
        }
        let candidate = Candidate {
            policy: "copy-effect-candidate-v1".into(),
            effect_digest: self.digest.clone(),
            owner_id: snapshot.owner_id.clone(),
            manifest_id: snapshot.manifest_id.clone(),
            token: token.into(),
            namespace: namespace.into(),
            scope_digest: scope_digest.into(),
        };
        db.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![snapshot.owner_id,token,CANDIDATE,encoded(&candidate)?,crate::library::store::now()])?;
        Ok(())
    }
}

impl Candidate {
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn preparation_recorded(&self, catalog: &Catalog) -> Result<bool> {
        let owner = self.owner_id.clone();
        let token = self.token.clone();
        catalog.call(move|db|Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase='namespace_preparation_fact_v2')",params![owner,token],|row|row.get(0))?))
    }
}
