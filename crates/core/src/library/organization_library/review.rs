use super::*;
use organization::{
    LibraryOrganizationSummary, OrganizationPreviewFilter, OrganizationPreviewQuery,
};

const ELIGIBLE: &str = "blocked=0 AND collision=0 AND case_collision=0 AND skipped=0";
const ATTENTION: &str = "blocked=1 OR collision=1 OR case_collision=1 OR missing_date=1";

pub(super) fn report(
    db: &Connection,
    session: &Session,
    offset: u32,
    query: OrganizationPreviewQuery,
) -> Result<LibraryOrganizationPreview> {
    if query.search.len() > 1024 || query.search.chars().count() > 256 {
        return Err(invalid("Search the preview with at most 256 characters."));
    }
    let complete = session.processed == session.total;
    let mut summary = None;
    let mut rows = Vec::new();
    let mut matching = 0;
    let mut blocked = 0;
    if complete {
        let totals = totals(db, &session.id, session.total)?;
        blocked = totals.blocked;
        summary = Some(totals.summary);
        let filter = match query.filter {
            OrganizationPreviewFilter::All => "1".into(),
            OrganizationPreviewFilter::Changes => {
                format!("({ELIGIBLE}) AND (moved=1 OR renamed=1)")
            }
            OrganizationPreviewFilter::NeedsAttention => ATTENTION.into(),
            OrganizationPreviewFilter::MissingDates => "missing_date=1".into(),
            OrganizationPreviewFilter::Skipped => "skipped=1".into(),
        };
        let clause = format!("({filter}) AND (?1='' OR instr(search_text,?1)>0)");
        let search = query.search.to_lowercase();
        matching = db.query_row(
            &format!("SELECT count(*) FROM structure_rows WHERE {clause}"),
            [&search],
            |r| r.get::<_, u32>(0),
        )?;
        if offset > matching {
            return Err(invalid("Invalid filtered preview offset."));
        }
        let mut statement = db.prepare(&format!("SELECT data,collision,case_collision FROM structure_rows WHERE {clause} ORDER BY sequence LIMIT ?2 OFFSET ?3"))?;
        let mut records = statement.query(params![search, PREVIEW_LIMIT as u32, offset])?;
        while let Some(record) = records.next()? {
            let data: String = record.get(0)?;
            let mut row: OrganizationRow = serde_json::from_str(&data)
                .map_err(|e| invalid("Invalid preview row.").caused_by(e))?;
            if record.get::<_, bool>(1)? {
                row.issues.push(OrganizationIssue::Collision);
            }
            if record.get::<_, bool>(2)? {
                row.issues.push(OrganizationIssue::PossibleCaseCollision);
            }
            rows.push(row);
        }
    } else if offset != 0 {
        return Err(invalid("Invalid whole-library preview offset."));
    }
    let skipped = summary.as_ref().map_or(0, |s| s.skipped);
    Ok(LibraryOrganizationPreview {
        preview_id: session.id.clone(),
        catalog_revision: session.revision.clone(),
        total: session.total,
        processed: session.processed,
        proposed: if complete {
            session.total - blocked - skipped
        } else {
            0
        },
        blocked,
        complete,
        offset,
        matching,
        summary,
        rows,
        execution_available: false,
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Totals {
    summary: LibraryOrganizationSummary,
    blocked: u32,
    total: u32,
}

fn totals(db: &Connection, id: &str, expected_total: u32) -> Result<Totals> {
    let cached = db.query_row("SELECT CASE WHEN length(data)<=2048 THEN data END FROM structure_summary WHERE preview_id=?", [id], |r| r.get::<_,String>(0)).optional()?;
    if let Some(cached) = cached {
        let totals: Totals = serde_json::from_str(&cached)
            .map_err(|e| invalid("Invalid preview summary.").caused_by(e))?;
        if totals.total == expected_total {
            return Ok(totals);
        }
    }
    let sql = format!(
        "SELECT COALESCE(sum(CASE WHEN {ELIGIBLE} THEN moved ELSE 0 END),0),count(DISTINCT CASE WHEN {ELIGIBLE} THEN folder END),COALESCE(sum(CASE WHEN {ELIGIBLE} THEN renamed ELSE 0 END),0),COALESCE(sum(CASE WHEN {ELIGIBLE} AND moved=0 AND renamed=0 THEN 1 ELSE 0 END),0),COALESCE(sum(CASE WHEN {ATTENTION} THEN 1 ELSE 0 END),0),COALESCE(sum(missing_date),0),COALESCE(sum(skipped),0),COALESCE(sum(CASE WHEN skipped=0 AND (blocked=1 OR collision=1 OR case_collision=1) THEN 1 ELSE 0 END),0),count(*) FROM structure_rows"
    );
    let totals = db.query_row(&sql, [], |r| {
        Ok(Totals {
            summary: LibraryOrganizationSummary {
                moves: r.get(0)?,
                folders: r.get(1)?,
                renames: r.get(2)?,
                unchanged: r.get(3)?,
                attention: r.get(4)?,
                missing_dates: r.get(5)?,
                skipped: r.get(6)?,
            },
            blocked: r.get(7)?,
            total: r.get(8)?,
        })
    })?;
    if totals.total != expected_total {
        return Err(invalid(
            "The complete preview membership changed. Build a fresh preview.",
        ));
    }
    db.execute(
        "INSERT INTO structure_summary VALUES(?,?) ON CONFLICT(preview_id) DO UPDATE SET data=excluded.data",
        params![
            id,
            serde_json::to_string(&totals)
                .map_err(|e| invalid("Could not encode preview summary.").caused_by(e))?
        ],
    )?;
    Ok(totals)
}
