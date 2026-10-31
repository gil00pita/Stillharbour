//! Catalog adapter for immutable plan content and append-only template versions.
use super::{Library, filters::invalid, native, store};
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::FileIdentity,
    operations::*,
    organization::{self, PreviewSource},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(r#"
    CREATE TABLE organization_templates(
      id TEXT NOT NULL,version INTEGER NOT NULL CHECK(version BETWEEN 1 AND 100),
      created_at TEXT NOT NULL,payload TEXT NOT NULL CHECK(json_valid(payload) AND length(payload)<=8192),
      PRIMARY KEY(id,version));
    CREATE TRIGGER template_no_update BEFORE UPDATE ON organization_templates BEGIN SELECT RAISE(ABORT,'Template versions are immutable'); END;
    CREATE TRIGGER template_no_delete BEFORE DELETE ON organization_templates BEGIN SELECT RAISE(ABORT,'Template history is retained'); END;
    CREATE TABLE organization_plans(
      id TEXT PRIMARY KEY,catalog_id TEXT NOT NULL,created_at TEXT NOT NULL,
      digest TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN ('draft','ready','approved','running','completed','completed_with_errors','canceled','needs_recovery','invalidated')),
      summary TEXT NOT NULL CHECK(json_valid(summary) AND length(summary)<=65536),
      payload TEXT NOT NULL CHECK(json_valid(payload) AND length(payload)<=8388608));
    CREATE INDEX organization_plan_order ON organization_plans(created_at DESC,id DESC);
    CREATE TRIGGER plan_content_immutable BEFORE UPDATE OF id,catalog_id,created_at,digest,summary,payload ON organization_plans BEGIN SELECT RAISE(ABORT,'Plan content is immutable'); END;
    CREATE TRIGGER plan_history_retained BEFORE DELETE ON organization_plans BEGIN SELECT RAISE(ABORT,'Plan history is retained'); END;
    CREATE TABLE operation_journal(sequence INTEGER PRIMARY KEY,plan_id TEXT NOT NULL REFERENCES organization_plans(id),
      item_id TEXT,phase TEXT NOT NULL,evidence TEXT NOT NULL CHECK(json_valid(evidence)),created_at TEXT NOT NULL);
    CREATE TRIGGER journal_no_update BEFORE UPDATE ON operation_journal BEGIN SELECT RAISE(ABORT,'Journal entries are immutable'); END;
    CREATE TRIGGER journal_no_delete BEFORE DELETE ON operation_journal BEGIN SELECT RAISE(ABORT,'Journal entries are retained'); END;
    "#)?;
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredPlan {
    pub view: OrganizationPlan,
    pub catalog_id: String,
    pub sources: Vec<PreviewSource>,
    pub destination_native: Vec<u8>,
    pub destination_volume: String,
    pub destination_device: u64,
    pub destination_file: u64,
}
impl StoredPlan {
    fn digest(&self) -> Result<String> {
        let mut content = self.clone();
        content.view.digest.clear();
        Ok(blake3::hash(
            &serde_json::to_vec(&content)
                .map_err(|e| invalid("Could not fingerprint the plan.").caused_by(e))?,
        )
        .to_hex()
        .to_string())
    }
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|e| invalid("Could not encode planning data.").caused_by(e))
}
fn decode<T: serde::de::DeserializeOwned>(value: &str, max: usize) -> Result<T> {
    if value.len() > max {
        return Err(invalid("Planning data exceeds its supported limit."));
    }
    serde_json::from_str(value).map_err(|e| {
        AppError::new(
            ErrorCode::Catalog,
            "Stored planning data is invalid or from an unsupported version.",
        )
        .caused_by(e)
    })
}

impl Library {
    pub fn organization_templates(&self) -> Result<Vec<SavedTemplate>> {
        self.catalog.call(|db| {
            let values=db.prepare("SELECT t.payload FROM organization_templates t WHERE t.version=(SELECT max(v.version) FROM organization_templates v WHERE v.id=t.id) ORDER BY t.created_at DESC,t.id LIMIT 101")?
                .query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            if values.len()>MAX_TEMPLATES {return Err(invalid("Stored template count exceeds the supported limit."));}
            values.iter().map(|v|decode(v,8192)).collect()
        })
    }
    pub fn organization_template_save(&self, save: TemplateSave) -> Result<SavedTemplate> {
        validate_template_save(&save)?;
        self.catalog.call(move |db| save_template(db, save))
    }
    pub fn organization_plan_create(
        &self,
        input: PlanDraftInput,
        path: PathBuf,
        expected: (FileIdentity, String),
    ) -> Result<OrganizationPlanDetail> {
        let destination = self.organization_destination(
            input.asset_ids.clone(),
            input.template.clone(),
            input.preview_fingerprint.clone(),
            path.clone(),
            expected.clone(),
        )?;
        let ids = input.asset_ids.clone();
        let (revision, sources) = self
            .catalog
            .call(move |db| super::organization::snapshot(db, ids))?;
        let preview = organization::preview(revision.clone(), input.template, sources.clone())?;
        if preview.fingerprint != input.preview_fingerprint
            || preview.catalog_revision != destination.catalog_revision
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The catalog changed while saving the plan. Build a fresh preview.",
            ));
        }
        let catalog_id = self.catalog.summary()?.id;
        let mut record = StoredPlan {
            view: OrganizationPlan {
                id: uuid::Uuid::new_v4().to_string(),
                policy: PLAN_POLICY.into(),
                created_at: store::now(),
                digest: String::new(),
                state: PlanState::Draft,
                template_reference: input.template_reference,
                preview,
                destination_path: destination.destination_path,
                destination_fingerprint: destination.fingerprint,
                blockers: vec![
                    PlanBlocker::NativeTargetChecks,
                    PlanBlocker::CompanionInventory,
                    PlanBlocker::CapacityAndFidelity,
                    PlanBlocker::ExecutionAndRecovery,
                ],
                execution_available: false,
            },
            catalog_id,
            sources,
            destination_native: native::encode_path(&path),
            destination_volume: expected.1,
            destination_device: expected.0.volume,
            destination_file: expected.0.file,
        };
        record.view.digest = record.digest()?;
        let result = record.view.clone();
        self.catalog
            .call(move |db| insert_plan(db, &record, &revision))?;
        Ok(OrganizationPlanDetail {
            plan: result,
            stale: false,
        })
    }
    pub fn organization_plan(&self, id: String) -> Result<OrganizationPlanDetail> {
        validate_id(&id)?;
        let stored = self.catalog.call(move |db| load(db, &id))?;
        let current = self.organization_preview(
            stored.sources.iter().map(|s| s.id.clone()).collect(),
            stored.view.preview.template.clone(),
        );
        let stale = match current {
            Ok(value) => value.fingerprint != stored.view.preview.fingerprint,
            Err(error) if matches!(error.code, ErrorCode::InvalidInput | ErrorCode::Conflict) => {
                true
            }
            Err(error) => return Err(error),
        };
        Ok(OrganizationPlanDetail {
            plan: stored.view,
            stale,
        })
    }
    pub fn organization_plans(&self, before: Option<String>) -> Result<OrganizationPlanPage> {
        if let Some(id) = &before {
            validate_id(id)?;
        }
        self.catalog.call(move|db| {
            if let Some(id)=&before {
                let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM organization_plans WHERE id=?)",[id],|r|r.get(0))?;
                if !exists {return Err(invalid("The plan history cursor was not found."));}
            }
            let mut result=Vec::new();
            let mut statement=db.prepare("SELECT summary FROM organization_plans WHERE (?1 IS NULL OR (created_at,id)<(SELECT created_at,id FROM organization_plans WHERE id=?1)) ORDER BY created_at DESC,id DESC LIMIT 51")?;
            let values=statement.query_map([before],|r|r.get::<_,String>(0))?;
            for value in values {result.push(decode::<OrganizationPlanSummary>(&value?,65536)?);}
            let next=if result.len()>50 {result.truncate(50);result.last().map(|p|p.id.clone())} else {None};
            Ok(OrganizationPlanPage {plans:result,next})
        })
    }
    pub fn organization_plan_export(
        &self,
        id: String,
        format: PlanExportFormat,
    ) -> Result<PlanExport> {
        let detail = self.organization_plan(id)?;
        crate::operations::export(&detail.plan, format)
    }
}

fn save_template(db: &mut Connection, save: TemplateSave) -> Result<SavedTemplate> {
    let tx = db.transaction()?;
    let (id, version) = if let Some(previous) = save.previous {
        let latest: Option<u32> = tx.query_row(
            "SELECT max(version) FROM organization_templates WHERE id=?",
            [&previous.id],
            |r| r.get(0),
        )?;
        if latest != Some(previous.version) {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The template changed. Reload it before saving a new version.",
            ));
        }
        (previous.id, previous.version + 1)
    } else {
        let count: u32 = tx.query_row(
            "SELECT count(DISTINCT id) FROM organization_templates",
            [],
            |r| r.get(0),
        )?;
        if count >= MAX_TEMPLATES as u32 {
            return Err(invalid(
                "This catalog supports at most 100 saved templates.",
            ));
        }
        (uuid::Uuid::new_v4().to_string(), 1)
    };
    let result = SavedTemplate {
        reference: TemplateRef { id, version },
        name: save.name.trim().into(),
        description: save.description,
        created_at: store::now(),
        template: save.template,
        policies: TemplatePolicies::default(),
    };
    tx.execute(
        "INSERT INTO organization_templates VALUES(?1,?2,?3,?4)",
        params![
            result.reference.id,
            result.reference.version,
            result.created_at,
            encode(&result)?
        ],
    )?;
    tx.commit()?;
    Ok(result)
}

fn insert_plan(db: &mut Connection, plan: &StoredPlan, revision: &str) -> Result<()> {
    let tx = db.transaction()?;
    let actual: i64 = tx.query_row(
        "SELECT revision FROM library_state WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    if actual.to_string() != revision {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "The catalog changed before the plan was saved.",
        ));
    }
    if let Some(reference) = &plan.view.template_reference {
        validate_id(&reference.id)?;
        let template: String = tx.query_row(
            "SELECT payload FROM organization_templates WHERE id=? AND version=?",
            params![reference.id, reference.version],
            |r| r.get(0),
        )?;
        let template: SavedTemplate = decode(&template, 8192)?;
        if encode(&template.template)? != encode(&plan.view.preview.template)? {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The selected template version does not match this preview.",
            ));
        }
    }
    let count: u32 = tx.query_row("SELECT count(*) FROM organization_plans", [], |r| r.get(0))?;
    if count >= 1000 {
        return Err(invalid(
            "This catalog supports at most 1,000 retained plans.",
        ));
    }
    let summary = OrganizationPlanSummary {
        id: plan.view.id.clone(),
        created_at: plan.view.created_at.clone(),
        digest: plan.view.digest.clone(),
        state: plan.view.state.clone(),
        destination_path: plan.view.destination_path.clone(),
        proposed: plan.view.preview.proposed,
        blocked: plan.view.preview.blocked,
        selected_bytes: plan.view.preview.selected_bytes.clone(),
    };
    let payload = encode(plan)?;
    if payload.len() > MAX_PLAN_BYTES {
        return Err(invalid("The plan exceeds its supported size."));
    }
    tx.execute(
        "INSERT INTO organization_plans VALUES(?1,?2,?3,?4,'draft',?5,?6)",
        params![
            plan.view.id,
            plan.catalog_id,
            plan.view.created_at,
            plan.view.digest,
            encode(&summary)?,
            payload
        ],
    )?;
    tx.execute("INSERT INTO operation_journal(plan_id,phase,evidence,created_at) VALUES(?1,'draft_created',?2,?3)",params![plan.view.id,encode(&serde_json::json!({"digest":plan.view.digest}))?,plan.view.created_at])?;
    tx.commit()?;
    Ok(())
}
pub(crate) fn load(db: &mut Connection, id: &str) -> Result<StoredPlan> {
    let (catalog,digest,payload):(String,String,String)=db.query_row("SELECT catalog_id,digest,payload FROM organization_plans WHERE id=? AND length(payload)<=8388608",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?.ok_or_else(||invalid("The saved plan was not found or exceeds its supported size."))?;
    let current: String = db.query_row(
        "SELECT uuid FROM catalog_identity WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    let plan: StoredPlan = decode(&payload, MAX_PLAN_BYTES)?;
    if catalog != current
        || plan.catalog_id != current
        || plan.view.id != id
        || plan.view.policy != PLAN_POLICY
        || plan.view.digest != digest
        || plan.digest()? != digest
        || plan.sources.is_empty()
        || plan.sources.len() > organization::PREVIEW_LIMIT
        || plan.view.preview.rows.len() != plan.sources.len()
        || plan.view.state != PlanState::Draft
        || plan.view.execution_available
        || plan.view.preview.execution_available
        || plan.view.blockers
            != [
                PlanBlocker::NativeTargetChecks,
                PlanBlocker::CompanionInventory,
                PlanBlocker::CapacityAndFidelity,
                PlanBlocker::ExecutionAndRecovery,
            ]
        || plan.destination_native.is_empty()
        || plan.destination_volume.is_empty()
    {
        return Err(AppError::new(
            ErrorCode::Catalog,
            "Saved plan identity or immutable content validation failed.",
        ));
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::{MissingDatePolicy, OrganizationTemplate};
    #[test]
    fn plan_content_is_retained_and_digest_and_catalog_binding_are_verified() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE catalog_identity(singleton INTEGER PRIMARY KEY,uuid TEXT);INSERT INTO catalog_identity VALUES(1,'fixture-catalog');CREATE TABLE library_state(singleton INTEGER PRIMARY KEY,revision INTEGER);INSERT INTO library_state VALUES(1,1);").unwrap();
        migrate(&db).unwrap();
        let sources = vec![PreviewSource {
            id: uuid::Uuid::new_v4().to_string(),
            root_id: uuid::Uuid::new_v4().to_string(),
            root_identity: "fixture".into(),
            native_path: vec![1, 2],
            relative_path: Some("photo.JPG".into()),
            display_path: "source/photo.JPG".into(),
            revision: "1".into(),
            bytes: 42,
            capture: None,
            available: true,
            companion_candidates: 0,
        }];
        let preview = organization::preview(
            "1".into(),
            OrganizationTemplate {
                pattern: "{original_stem}.{ext}".into(),
                event: String::new(),
                missing_date: MissingDatePolicy::Block,
            },
            sources.clone(),
        )
        .unwrap();
        let mut record = StoredPlan {
            view: OrganizationPlan {
                id: uuid::Uuid::new_v4().to_string(),
                policy: PLAN_POLICY.into(),
                created_at: store::now(),
                digest: String::new(),
                state: PlanState::Draft,
                template_reference: None,
                preview,
                destination_path: "fixture destination".into(),
                destination_fingerprint: "destination evidence".into(),
                blockers: vec![
                    PlanBlocker::NativeTargetChecks,
                    PlanBlocker::CompanionInventory,
                    PlanBlocker::CapacityAndFidelity,
                    PlanBlocker::ExecutionAndRecovery,
                ],
                execution_available: false,
            },
            catalog_id: "fixture-catalog".into(),
            sources,
            destination_native: vec![3, 4],
            destination_volume: "local fixture".into(),
            destination_device: 1,
            destination_file: 2,
        };
        record.view.digest = record.digest().unwrap();
        assert!(insert_plan(&mut db, &record, "2").is_err());
        assert_eq!(
            db.query_row("SELECT count(*) FROM organization_plans", [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            0
        );
        insert_plan(&mut db, &record, "1").unwrap();
        assert_eq!(
            load(&mut db, &record.view.id).unwrap().view.digest,
            record.view.digest
        );
        assert!(
            db.execute("UPDATE organization_plans SET payload='{}'", [])
                .is_err()
        );
        assert!(db.execute("DELETE FROM organization_plans", []).is_err());
        assert!(db.execute("DELETE FROM operation_journal", []).is_err());
        assert!(
            db.execute("UPDATE operation_journal SET phase='completed'", [])
                .is_err()
        );
        db.execute("UPDATE catalog_identity SET uuid='other-catalog'", [])
            .unwrap();
        assert!(load(&mut db, &record.view.id).is_err());
        db.execute("UPDATE catalog_identity SET uuid='fixture-catalog'", [])
            .unwrap();
        // Simulate a damaged/imported catalog only inside this in-memory fixture.
        db.execute_batch("DROP TRIGGER plan_content_immutable")
            .unwrap();
        record.view.destination_path = "tampered".into();
        db.execute(
            "UPDATE organization_plans SET payload=?",
            [encode(&record).unwrap()],
        )
        .unwrap();
        assert!(load(&mut db, &record.view.id).is_err());
    }
    #[test]
    fn template_versions_compare_and_swap_and_cannot_be_rewritten() {
        let mut db = Connection::open_in_memory().unwrap();
        migrate(&db).unwrap();
        let input = TemplateSave {
            previous: None,
            name: "Retained".into(),
            description: String::new(),
            template: OrganizationTemplate {
                pattern: "{original_stem}.{ext}".into(),
                event: String::new(),
                missing_date: MissingDatePolicy::Block,
            },
        };
        let first = save_template(&mut db, input.clone()).unwrap();
        let mut next = input;
        next.previous = Some(first.reference.clone());
        next.template.pattern = "copy/{original_stem}.{ext}".into();
        let second = save_template(&mut db, next.clone()).unwrap();
        assert_eq!(second.reference.version, 2);
        assert_eq!(
            save_template(&mut db, next).unwrap_err().code,
            ErrorCode::Conflict
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM organization_templates", [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            2
        );
        assert!(
            db.execute("DELETE FROM organization_templates", [])
                .is_err()
        );
        assert!(
            db.execute("UPDATE organization_templates SET payload='{}'", [])
                .is_err()
        );
        let old: String = db
            .query_row(
                "SELECT payload FROM organization_templates WHERE version=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            decode::<SavedTemplate>(&old, 8192)
                .unwrap()
                .template
                .pattern,
            first.template.pattern
        );
    }
}
