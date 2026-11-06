//! Immutable display-only scan versions. No paths here are execution inputs.
use super::*;
use rusqlite::params;
use serde::{Deserialize, Serialize};

const SCOPE: &str = "utility-report-v1";
const LIMIT: usize = 16 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    version: u32,
    run: ComparisonRun,
    rows: Vec<ComparisonEntry>,
}

fn display_only(mut run: ComparisonRun) -> ComparisonRun {
    run.plan_digest = None;
    run.review_plan = None;
    run.capabilities = DestinationCapability::default();
    run
}

fn decode(text: String, id: &str, catalog_id: &str) -> Result<Report> {
    if text.len() > LIMIT {
        return Err(unavailable());
    }
    let mut report: Report = serde_json::from_str(&text).map_err(|_| unavailable())?;
    if report.version != 1
        || report.run.id != id
        || report.run.catalog_id != catalog_id
        || report.rows.len() > MAX_ENTRIES
        || report.run.state == ComparisonState::Comparing
    {
        return Err(unavailable());
    }
    crate::operations::validate_id(&report.run.id)?;
    report.run = display_only(report.run);
    Ok(report)
}

pub(super) fn save(
    catalog: &crate::catalog::Catalog,
    run: &ComparisonRun,
    rows: &[ComparisonEntry],
) -> Result<String> {
    let report = Report {
        version: 1,
        run: display_only(run.clone()),
        rows: rows.to_vec(),
    };
    let text = serde_json::to_string(&report).map_err(|_| unavailable())?;
    if text.len() > LIMIT || rows.len() > MAX_ENTRIES {
        return Err(error(
            ErrorCode::Unsupported,
            "Scan report exceeds its storage bound; earlier versions are retained.",
        ));
    }
    catalog.call(move |db| {
        let tx = db.transaction()?;
        let count: u32 = tx.query_row("SELECT count(*) FROM backup_comparison_history WHERE scope=?", [SCOPE], |r| r.get(0))?;
        if count >= 32 { return Ok("Scan history is full (32 saved reports); this report is session-only. Earlier versions are retained.".into()); }
        tx.execute("INSERT INTO backup_comparison_history(id,scope,record) VALUES(?,?,?)", params![format!("report:{}", report.run.id), SCOPE, text])?;
        tx.commit()?;
        Ok("Scan report saved in history; historical reports cannot approve copying.".into())
    })
}

pub(super) fn list(catalog: &crate::catalog::Catalog) -> Result<Vec<ComparisonRun>> {
    catalog.call(|db| {
        let catalog_id: String = db.query_row("SELECT uuid FROM catalog_identity WHERE singleton=1", [], |r| r.get(0))?;
        let mut statement = db.prepare("SELECT id,record FROM backup_comparison_history WHERE scope=? AND length(CAST(record AS BLOB))<=16777216 ORDER BY rowid DESC LIMIT 32")?;
        let records = statement.query_map([SCOPE], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        records.map(|row| { let (id, text) = row?; Ok(decode(text, id.strip_prefix("report:").ok_or_else(unavailable)?, &catalog_id)?.run) }).collect()
    })
}

pub(super) fn load(
    catalog: &crate::catalog::Catalog,
    id: &str,
) -> Result<(ComparisonRun, Vec<ComparisonEntry>)> {
    crate::operations::validate_id(id)?;
    let id = id.to_owned();
    catalog.call(move |db| {
        let catalog_id: String = db.query_row("SELECT uuid FROM catalog_identity WHERE singleton=1", [], |r| r.get(0))?;
        let text: String = db.query_row("SELECT record FROM backup_comparison_history WHERE id=? AND scope=? AND length(CAST(record AS BLOB))<=16777216", params![format!("report:{id}"), SCOPE], |r| r.get(0))?;
        let report = decode(text, &id, &catalog_id)?;
        Ok((report.run, report.rows))
    })
}
