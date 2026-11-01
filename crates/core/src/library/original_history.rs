//! Read-only recovery discovery. Retained plans never restore execution
//! authority, infer current file locations, or carry an imported approval.
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    operations::{original_jobs::*, rename_graph},
};
use rusqlite::OptionalExtension;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    policy: String,
    catalog_id: String,
    root_id: String,
    source_device: String,
    source_file: String,
    graph: rename_graph::Plan,
    reverse_of: Option<String>,
}
fn invalid() -> AppError {
    AppError::new(
        ErrorCode::Catalog,
        "Retained organization history is inconsistent. Preserve the catalog and files for recovery review.",
    )
}
#[cfg(test)]
#[path = "original_history_tests.rs"]
mod tests;

impl super::Library {
    pub fn organization_original_history(
        &self,
        job_id: String,
    ) -> Result<OriginalOperationHistory> {
        review(&self.catalog, job_id)
    }
}
pub(crate) fn review(catalog: &Catalog, job_id: String) -> Result<OriginalOperationHistory> {
    crate::operations::validate_id(&job_id)?;
    catalog.call(move |db| {
        let job = super::original_jobs::load_db(db, &job_id)?;
        let (payload, digest, catalog_id, root_id, kind, reverse_of, total_moves, scope):
            (Option<String>, String, String, String, String, Option<String>, u32, String) = db.query_row(
                "SELECT CASE WHEN length(CAST(payload AS BLOB))<=2097152 THEN payload END,digest,catalog_id,root_id,kind,reverse_of,total_moves,scope FROM original_operation_plans WHERE id=?",
                [&job.plan_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?)))?;
        let payload = payload.ok_or_else(invalid)?;
        if blake3::hash(payload.as_bytes()).to_hex().as_str() != digest || scope != "generated_fixture" {
            return Err(invalid());
        }
        let record: Record = serde_json::from_str(&payload).map_err(|_| invalid())?;
        let current_catalog: String = db.query_row("SELECT uuid FROM catalog_identity WHERE singleton=1", [], |row| row.get(0))?;
        if record.policy != "generated-original-plan-v1" || record.catalog_id != current_catalog || record.catalog_id != catalog_id
            || record.root_id != root_id || root_id != job.source_root_id || record.reverse_of != reverse_of
            || record.graph.id != job.plan_id || record.graph.steps.len() != total_moves as usize || total_moves != job.total_moves
            || kind != if reverse_of.is_some() { "undo" } else { "rename" }
            || !record.graph.issues.is_empty() || record.graph.execution_available || !record.graph.independent_recovery_required {
            return Err(invalid());
        }
        for value in [&record.source_device, &record.source_file] {
            let number = value.parse::<u64>().map_err(|_| invalid())?;
            if number.to_string() != *value { return Err(invalid()); }
        }
        if let Some(parent) = &reverse_of {
            crate::operations::validate_id(parent)?;
            let prior: Option<(String, String, String)> = db.query_row("SELECT catalog_id,root_id,kind FROM original_operation_plans WHERE id=?", [parent], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?;
            if prior != Some((current_catalog, root_id.clone(), "rename".into())) { return Err(invalid()); }
        }
        let canonical = rename_graph::build(record.graph.id.clone(), record.graph.members.clone(), record.graph.name_facts.clone())?;
        if serde_json::to_vec(&canonical).map_err(|_| invalid())? != serde_json::to_vec(&record.graph).map_err(|_| invalid())? {
            return Err(invalid());
        }
        Ok(OriginalOperationHistory {
            job, plan_digest: digest, reverse_of,
            members: record.graph.members.into_iter().map(|member| OriginalHistoryMember { id: member.id, before: member.before, after: member.after }).collect(),
            history_only: true, current_locations_checked: false, execution_available: false,
        })
    })
}
