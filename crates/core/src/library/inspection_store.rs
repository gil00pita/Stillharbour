//! Same-catalog read-only job history and append-only integrity evidence.
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    inspection::{invalid, model::*},
};
use rusqlite::{Connection, OptionalExtension, params};

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(r#"
    CREATE TABLE inspection_jobs(
      id TEXT PRIMARY KEY,input TEXT NOT NULL CHECK(json_valid(input) AND length(CAST(input AS BLOB))<=65536),
      state TEXT NOT NULL CHECK(state IN ('queued','running','cancel_requested','completed','partial','canceled','interrupted','failed')),
      started_at TEXT NOT NULL,updated_at TEXT NOT NULL,finished_at TEXT,
      files_observed INTEGER NOT NULL DEFAULT 0 CHECK(files_observed>=0),files_checked INTEGER NOT NULL DEFAULT 0 CHECK(files_checked>=0),
      bytes_read TEXT NOT NULL DEFAULT '0' CHECK(length(bytes_read)>0 AND bytes_read NOT GLOB '*[^0-9]*'),
      issues INTEGER NOT NULL DEFAULT 0 CHECK(issues>=0),limited INTEGER NOT NULL DEFAULT 0 CHECK(limited IN (0,1)),message TEXT NOT NULL CHECK(length(message)<=4096)
    );
    CREATE UNIQUE INDEX inspection_one_active ON inspection_jobs((1)) WHERE state IN ('queued','running','cancel_requested');
    CREATE INDEX inspection_job_history ON inspection_jobs(started_at DESC,id DESC);
    CREATE TRIGGER inspection_input_immutable BEFORE UPDATE ON inspection_jobs WHEN NEW.id!=OLD.id OR NEW.input!=OLD.input OR NEW.started_at!=OLD.started_at BEGIN SELECT RAISE(ABORT,'Retain inspection input'); END;
    CREATE TRIGGER inspection_terminal_immutable BEFORE UPDATE ON inspection_jobs WHEN OLD.state IN ('completed','partial','canceled','interrupted','failed') BEGIN SELECT RAISE(ABORT,'Retain terminal inspection history'); END;
    CREATE TRIGGER inspection_history_retained BEFORE DELETE ON inspection_jobs BEGIN SELECT RAISE(ABORT,'Retain inspection history'); END;
    CREATE TRIGGER inspection_state_order BEFORE UPDATE OF state ON inspection_jobs WHEN
      (OLD.state='queued' AND NEW.state NOT IN ('running','cancel_requested','canceled','interrupted','failed')) OR
      (OLD.state='running' AND NEW.state NOT IN ('running','cancel_requested','completed','partial','canceled','interrupted','failed')) OR
      (OLD.state='cancel_requested' AND NEW.state NOT IN ('cancel_requested','canceled','partial','interrupted','failed'))
      BEGIN SELECT RAISE(ABORT,'Invalid inspection transition'); END;
    CREATE TABLE inspection_job_roots(job_id TEXT NOT NULL REFERENCES inspection_jobs(id),root_id TEXT NOT NULL,native_path BLOB NOT NULL CHECK(length(native_path)<=65536),volume TEXT NOT NULL,device TEXT NOT NULL,inode TEXT NOT NULL,last_scan TEXT,PRIMARY KEY(job_id,root_id));
    CREATE TABLE inspection_items(job_id TEXT NOT NULL REFERENCES inspection_jobs(id),id TEXT NOT NULL,ordinal INTEGER NOT NULL CHECK(ordinal>=0),root_id TEXT NOT NULL,native_relative BLOB NOT NULL CHECK(length(native_relative)<=65536),object_key TEXT,revision TEXT NOT NULL,bytes TEXT NOT NULL,payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=32768),PRIMARY KEY(job_id,id),UNIQUE(job_id,ordinal));
    CREATE INDEX inspection_largest_files ON inspection_items(job_id,length(bytes) DESC,bytes DESC,id);
    CREATE TABLE inspection_groups(job_id TEXT NOT NULL REFERENCES inspection_jobs(id),id TEXT NOT NULL,ordinal INTEGER NOT NULL CHECK(ordinal>=0),payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=16384),PRIMARY KEY(job_id,id),UNIQUE(job_id,ordinal));
    CREATE TABLE inspection_group_members(job_id TEXT NOT NULL,group_id TEXT NOT NULL,file_id TEXT NOT NULL,ordinal INTEGER NOT NULL CHECK(ordinal>=0),PRIMARY KEY(job_id,group_id,file_id),UNIQUE(job_id,group_id,ordinal),FOREIGN KEY(job_id,group_id) REFERENCES inspection_groups(job_id,id),FOREIGN KEY(job_id,file_id) REFERENCES inspection_items(job_id,id));
    CREATE TABLE inspection_folders(job_id TEXT NOT NULL REFERENCES inspection_jobs(id),id TEXT NOT NULL,parent_id TEXT,logical_bytes TEXT NOT NULL,payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=8192),PRIMARY KEY(job_id,id));
    CREATE INDEX inspection_folder_children ON inspection_folders(job_id,parent_id,length(logical_bytes) DESC,logical_bytes DESC,id);
    CREATE TABLE inspection_keeper_reviews(id TEXT PRIMARY KEY,job_id TEXT NOT NULL REFERENCES inspection_jobs(id),group_id TEXT NOT NULL,payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=1048576),FOREIGN KEY(job_id,group_id) REFERENCES inspection_groups(job_id,id));
    CREATE TABLE inspection_summaries(job_id TEXT PRIMARY KEY REFERENCES inspection_jobs(id),payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=65536));
    CREATE TABLE inspection_issues(job_id TEXT NOT NULL REFERENCES inspection_jobs(id),ordinal INTEGER NOT NULL,payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=32768),PRIMARY KEY(job_id,ordinal));
    CREATE TABLE inspection_fingerprints(id TEXT PRIMARY KEY,file_id TEXT NOT NULL,object_key TEXT NOT NULL,revision TEXT NOT NULL,policy TEXT NOT NULL,algorithm TEXT NOT NULL CHECK(algorithm IN ('blake3','sha256','sha1','md5')),digest TEXT NOT NULL,bytes TEXT NOT NULL,observed_at TEXT NOT NULL,UNIQUE(file_id,object_key,revision,policy,algorithm));
    CREATE TABLE integrity_baselines(sequence INTEGER PRIMARY KEY,id TEXT NOT NULL UNIQUE,establishing_job_id TEXT NOT NULL REFERENCES inspection_jobs(id),file_id TEXT NOT NULL,previous_id TEXT REFERENCES integrity_baselines(id),root_id TEXT NOT NULL,object_key TEXT NOT NULL,native_relative BLOB NOT NULL,content_revision TEXT NOT NULL,algorithm TEXT NOT NULL CHECK(algorithm='blake3'),policy TEXT NOT NULL,digest TEXT NOT NULL CHECK(length(digest)=64 AND digest NOT GLOB '*[^0-9a-f]*'),bytes TEXT NOT NULL CHECK(length(bytes)>0 AND bytes NOT GLOB '*[^0-9]*'),established_at TEXT NOT NULL,origin TEXT NOT NULL CHECK(origin='explicit_user_establishment'),UNIQUE(establishing_job_id,file_id));
    CREATE INDEX integrity_baseline_history ON integrity_baselines(file_id,sequence DESC);
    CREATE TRIGGER integrity_previous_file BEFORE INSERT ON integrity_baselines WHEN NEW.previous_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM integrity_baselines WHERE id=NEW.previous_id AND file_id=NEW.file_id) BEGIN SELECT RAISE(ABORT,'Baseline history must belong to this file'); END;
    "#)?;
    for table in [
        "inspection_job_roots",
        "inspection_groups",
        "inspection_group_members",
        "inspection_folders",
        "inspection_fingerprints",
        "inspection_summaries",
        "inspection_keeper_reviews",
        "inspection_issues",
        "integrity_baselines",
    ] {
        db.execute_batch(&format!("CREATE TRIGGER {table}_no_update BEFORE UPDATE ON {table} BEGIN SELECT RAISE(ABORT,'Retain immutable inspection evidence'); END; CREATE TRIGGER {table}_no_delete BEFORE DELETE ON {table} BEGIN SELECT RAISE(ABORT,'Retain inspection evidence'); END;"))?;
    }
    db.execute_batch("CREATE TRIGGER inspection_item_identity BEFORE UPDATE ON inspection_items WHEN NEW.job_id!=OLD.job_id OR NEW.id!=OLD.id OR NEW.ordinal!=OLD.ordinal OR NEW.root_id!=OLD.root_id OR NEW.native_relative!=OLD.native_relative OR NEW.revision!=OLD.revision OR NEW.bytes!=OLD.bytes OR NEW.object_key IS NOT OLD.object_key BEGIN SELECT RAISE(ABORT,'Retain inspection source facts'); END; CREATE TRIGGER inspection_item_terminal BEFORE UPDATE ON inspection_items WHEN (SELECT state FROM inspection_jobs WHERE id=OLD.job_id) NOT IN ('queued','running','cancel_requested') BEGIN SELECT RAISE(ABORT,'Retain completed findings'); END; CREATE TRIGGER inspection_item_no_delete BEFORE DELETE ON inspection_items BEGIN SELECT RAISE(ABORT,'Retain inspection findings'); END;")?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn strip_schema(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("DROP TABLE inspection_keeper_reviews; DROP TABLE inspection_summaries; DROP TABLE inspection_issues; DROP TABLE inspection_group_members; DROP TABLE inspection_groups; DROP TABLE inspection_folders; DROP TABLE inspection_items; DROP TABLE inspection_job_roots; DROP TABLE inspection_jobs; DROP TABLE inspection_fingerprints; DROP TABLE integrity_baselines;")
}
fn decode<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    serde_json::from_str(text).map_err(|error| {
        AppError::new(
            ErrorCode::Catalog,
            "Retained inspection data is invalid; no source operation was started.",
        )
        .caused_by(error)
    })
}
fn state_text(state: InspectionState) -> &'static str {
    match state {
        InspectionState::Queued => "queued",
        InspectionState::Running => "running",
        InspectionState::CancelRequested => "cancel_requested",
        InspectionState::Completed => "completed",
        InspectionState::Partial => "partial",
        InspectionState::Canceled => "canceled",
        InspectionState::Interrupted => "interrupted",
        InspectionState::Failed => "failed",
    }
}
fn load(db: &Connection, id: &str) -> Result<InspectionJob> {
    let (input, state, started_at, updated_at, finished_at, files_observed, files_checked, bytes_read, issues, limited, message) = db.query_row("SELECT input,state,started_at,updated_at,finished_at,files_observed,files_checked,bytes_read,issues,limited,message FROM inspection_jobs WHERE id=?", [id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get::<_,String>(7)?,row.get(8)?,row.get(9)?,row.get(10)?)))?;
    let input: InspectionInput = decode(&input)?;
    input.validate()?;
    bytes_read
        .parse::<u128>()
        .map_err(|_| invalid("Retained inspection byte count is invalid."))?;
    let state = decode(
        &serde_json::to_string(&state)
            .map_err(|error| invalid("Invalid inspection state.").caused_by(error))?,
    )?;
    Ok(InspectionJob {
        id: id.into(),
        input,
        state,
        started_at,
        updated_at,
        finished_at,
        files_observed,
        files_checked,
        bytes_read,
        issues,
        limited,
        message,
    })
}
pub(crate) fn create(catalog: &Catalog, input: InspectionInput) -> Result<InspectionJob> {
    input.validate()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = super::store::now();
    let payload = serde_json::to_string(&input)
        .map_err(|error| invalid("Could not bind inspection input.").caused_by(error))?;
    catalog.call(move |db| { db.execute("INSERT INTO inspection_jobs(id,input,state,started_at,updated_at,message) VALUES(?1,?2,'queued',?3,?3,'Inspection queued; originals unchanged')", params![id,payload,now])?; load(db, &id) })
}
pub(crate) fn job(catalog: &Catalog, id: String) -> Result<InspectionJob> {
    crate::operations::validate_id(&id)?;
    catalog.call(move |db| load(db, &id))
}
pub(crate) fn jobs(catalog: &Catalog) -> Result<Vec<InspectionJob>> {
    catalog.call(|db| {
        let ids = db
            .prepare("SELECT id FROM inspection_jobs ORDER BY started_at DESC,id DESC LIMIT 64")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter().map(|id| load(db, id)).collect()
    })
}
pub(crate) fn recover(catalog: &Catalog) -> Result<()> {
    catalog.call(|db| { db.execute("UPDATE inspection_jobs SET state='interrupted',updated_at=?1,finished_at=?1,message='Inspection interrupted. Retained observations are history; start a new authorized read to inspect current files.' WHERE state IN ('queued','running','cancel_requested')", [super::store::now()])?; Ok(()) })
}
pub(crate) fn begin(catalog: &Catalog, id: String) -> Result<()> {
    catalog.call(move |db| { if db.execute("UPDATE inspection_jobs SET state='running',updated_at=?,message='Reading the authorized scope; originals unchanged' WHERE id=? AND state='queued'", params![super::store::now(), id])? != 1 { return Err(invalid("This inspection is no longer queued.")); } Ok(()) })
}
pub(crate) fn cancel(catalog: &Catalog, id: String) -> Result<InspectionJob> {
    crate::operations::validate_id(&id)?;
    catalog.call(move |db| { db.execute("UPDATE inspection_jobs SET state='cancel_requested',updated_at=?,message='Cancellation requested; stopping at the next read boundary' WHERE id=? AND state IN ('queued','running')",params![super::store::now(),id])?; load(db,&id) })
}
pub(crate) fn finish(catalog: &Catalog, mut job: InspectionJob) -> Result<InspectionJob> {
    if !matches!(
        job.state,
        InspectionState::Completed
            | InspectionState::Partial
            | InspectionState::Canceled
            | InspectionState::Failed
    ) {
        return Err(invalid("Invalid terminal inspection state."));
    }
    job.bytes_read
        .parse::<u128>()
        .map_err(|_| invalid("Invalid inspection byte count."))?;
    job.updated_at = super::store::now();
    job.finished_at = Some(job.updated_at.clone());
    catalog.call(move |db| { db.execute("UPDATE inspection_jobs SET state=CASE WHEN state='cancel_requested' THEN 'canceled' ELSE ?1 END,updated_at=?2,finished_at=?2,files_observed=?3,files_checked=?4,bytes_read=?5,issues=?6,limited=?7,message=CASE WHEN state='cancel_requested' THEN 'Inspection canceled; partial observations retained' ELSE ?8 END WHERE id=?9 AND state IN ('queued','running','cancel_requested')",params![state_text(job.state),job.updated_at,job.files_observed,job.files_checked,job.bytes_read,job.issues,job.limited,job.message,job.id])?; load(db,&job.id) })
}

fn baseline_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IntegrityBaseline> {
    Ok(IntegrityBaseline {
        id: row.get("id")?,
        file_id: row.get("file_id")?,
        previous_id: row.get("previous_id")?,
        content_revision: row.get("content_revision")?,
        algorithm: row.get("algorithm")?,
        policy: row.get("policy")?,
        digest: row.get("digest")?,
        bytes: row.get("bytes")?,
        established_at: row.get("established_at")?,
        origin: row.get("origin")?,
    })
}
pub(crate) fn latest_baseline(
    catalog: &Catalog,
    file_id: String,
) -> Result<Option<IntegrityBaseline>> {
    catalog.call(move |db| {
        Ok(db
            .query_row(
                "SELECT * FROM integrity_baselines WHERE file_id=? ORDER BY sequence DESC LIMIT 1",
                [file_id],
                baseline_row,
            )
            .optional()?)
    })
}

pub(crate) fn save_roots(
    catalog: &Catalog,
    job_id: String,
    roots: Vec<super::store::RootRecord>,
) -> Result<()> {
    catalog.call(move |db| {
        let job = load(db, &job_id)?;
        if job.state != InspectionState::Running || roots.len() != job.input.root_ids.len() {
            return Err(invalid(
                "Inspection root snapshot does not match its running job.",
            ));
        }
        let tx = db.transaction()?;
        for root in roots {
            if !job.input.root_ids.contains(&root.view.id) {
                return Err(invalid("A root is outside this inspection."));
            }
            tx.execute(
                "INSERT INTO inspection_job_roots VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    job_id,
                    root.view.id,
                    super::native::encode_path(&root.path),
                    root.volume,
                    root.identity.volume.to_string(),
                    root.identity.file.to_string(),
                    root.view.last_scan
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
}
pub(crate) fn insert_item(
    catalog: &Catalog,
    job_id: String,
    ordinal: u32,
    snapshot: &super::inspection_sources::Snapshot,
    finding: InspectionFinding,
) -> Result<()> {
    if snapshot.id != finding.id || snapshot.bytes.to_string() != finding.bytes {
        return Err(invalid("Inspection source and public finding differ."));
    }
    let relative = super::native::encode_path(&snapshot.relative);
    let object = snapshot.object_key();
    let revision = snapshot.revision.clone();
    let bytes = snapshot.bytes.to_string();
    let payload = serde_json::to_string(&finding)
        .map_err(|error| invalid("Could not retain the finding.").caused_by(error))?;
    catalog.call(move |db| {
        let job = load(db, &job_id)?;
        if job.state != InspectionState::Running || !job.input.root_ids.contains(&finding.root_id) {
            return Err(invalid("The finding is outside this running inspection."));
        }
        db.execute(
            "INSERT INTO inspection_items VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                job_id,
                finding.id,
                ordinal,
                finding.root_id,
                relative,
                object,
                revision,
                bytes,
                payload
            ],
        )?;
        Ok(())
    })
}
pub(crate) fn update_finding(
    catalog: &Catalog,
    job_id: String,
    finding: InspectionFinding,
) -> Result<()> {
    let payload = serde_json::to_string(&finding)
        .map_err(|error| invalid("Could not retain the finding.").caused_by(error))?;
    catalog.call(move |db| {
        if db.execute("UPDATE inspection_items SET payload=?1 WHERE job_id=?2 AND id=?3 AND root_id=?4 AND bytes=?5",params![payload,job_id,finding.id,finding.root_id,finding.bytes])? != 1 { return Err(invalid("The inspected finding does not match its retained membership.")); } Ok(())
    })
}
pub(super) fn fingerprint(
    catalog: &Catalog,
    entry: &super::inspection_sources::Snapshot,
    fingerprint: crate::inspection::candidates::Fingerprint,
) -> Result<()> {
    let file = entry.id.clone();
    let object = entry.object_key();
    let revision = entry.revision.clone();
    let bytes = entry.bytes.to_string();
    let algorithm = match fingerprint.algorithm {
        ContentAlgorithm::Blake3 => "blake3",
        ContentAlgorithm::Sha256 => "sha256",
        ContentAlgorithm::Sha1 => "sha1",
        ContentAlgorithm::Md5 => "md5",
        ContentAlgorithm::DirectBytes => {
            return Err(invalid("Direct comparison has no fingerprint."));
        }
    };
    let policy = format!("{}:{algorithm}:adapter-v1", fingerprint.policy);
    let signature = serde_json::to_vec(&(&file, &object, &revision, &policy, algorithm))
        .map_err(|error| invalid("Could not bind fingerprint.").caused_by(error))?;
    let id = blake3::hash(&signature).to_hex().to_string();
    catalog.call(move |db| {
        db.execute(
            "INSERT OR IGNORE INTO inspection_fingerprints VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                id,
                file,
                object,
                revision,
                policy,
                algorithm,
                fingerprint.digest,
                bytes,
                super::store::now()
            ],
        )?;
        Ok(())
    })
}
pub(super) fn progress(catalog: &Catalog, job: &InspectionJob) -> Result<()> {
    let job = job.clone();
    catalog.call(move |db| {
        db.execute("UPDATE inspection_jobs SET files_observed=?1,files_checked=?2,bytes_read=?3,updated_at=?4 WHERE id=?5 AND state IN ('running','cancel_requested')",params![job.files_observed,job.files_checked,job.bytes_read,super::store::now(),job.id])?;
        Ok(())
    })
}
pub(crate) fn findings(
    catalog: &Catalog,
    job_id: String,
    after: Option<String>,
) -> Result<FindingPage> {
    crate::operations::validate_id(&job_id)?;
    catalog.call(move |db| {
        load(db, &job_id)?;
        if after.as_ref().is_some_and(|id| id.len()>128) { return Err(invalid("Invalid finding cursor.")); }
        let cursor = match after { Some(id) => Some(db.query_row("SELECT bytes,id FROM inspection_items WHERE job_id=? AND id=?",params![job_id,id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?), None => None };
        let records = if let Some((bytes,id)) = cursor {
            db.prepare("SELECT id,root_id,bytes,payload FROM inspection_items WHERE job_id=?1 AND (length(bytes)<length(?2) OR (length(bytes)=length(?2) AND bytes<?2) OR (bytes=?2 AND id>?3)) ORDER BY length(bytes) DESC,bytes DESC,id LIMIT 241")?.query_map(params![job_id,bytes,id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            db.prepare("SELECT id,root_id,bytes,payload FROM inspection_items WHERE job_id=? ORDER BY length(bytes) DESC,bytes DESC,id LIMIT 241")?.query_map([&job_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let more = records.len() > 240;
        let mut rows = Vec::new();
        for (id,root,bytes,payload) in records.into_iter().take(240) {
            let finding: InspectionFinding = decode(&payload)?;
            if finding.id != id || finding.root_id != root || finding.bytes != bytes { return Err(invalid("The retained finding does not match its source identity.")); }
            rows.push(finding);
        }
        let next = if more { rows.last().map(|row| row.id.clone()) } else { None };
        Ok(FindingPage { job_id, rows, next })
    })
}
pub(crate) fn append_baseline(
    catalog: &Catalog,
    job_id: String,
    snapshot: &super::inspection_sources::Snapshot,
    root: &super::store::RootRecord,
    digest: String,
) -> Result<IntegrityBaseline> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid("A baseline requires a complete BLAKE3 digest."));
    }
    let file_id = snapshot.id.clone();
    let root_id = root.view.id.clone();
    let object = snapshot.object_key();
    let relative = super::native::encode_path(&snapshot.relative);
    let revision = snapshot.revision.clone();
    let bytes = snapshot.bytes.to_string();
    let root_path = super::native::encode_path(&root.path);
    let device = root.identity.volume.to_string();
    let inode = root.identity.file.to_string();
    let volume = root.volume.clone();
    catalog.call(move |db| {
        let job=load(db,&job_id)?;
        if job.state!=InspectionState::Running || !matches!(job.input.task,InspectionTask::Health { mode: HealthMode::EstablishBaseline }) { return Err(invalid("Only an explicitly started baseline-establishment job may append evidence.")); }
        let tx=db.transaction()?;
        let binding:(Vec<u8>,String,String,String,String)=tx.query_row("SELECT native_path,device,inode,volume,availability FROM roots WHERE id=?",[&root_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)))?;
        if binding!=(root_path,device,inode,volume,"online".into()) { return Err(crate::inspection::changed("The baseline root binding changed before recording.")); }
        let retained:(String,String,String,Vec<u8>)=tx.query_row("SELECT object_key,revision,bytes,native_relative FROM inspection_items WHERE job_id=? AND id=? AND root_id=?",params![job_id,file_id,root_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))?;
        if retained!=(object.clone(),revision.clone(),bytes.clone(),relative.clone()) { return Err(invalid("Baseline evidence differs from the retained source snapshot.")); }
        let previous:Option<String>=tx.query_row("SELECT id FROM integrity_baselines WHERE file_id=? ORDER BY sequence DESC LIMIT 1",[&file_id],|row|row.get(0)).optional()?;
        let id=uuid::Uuid::new_v4().to_string(); let now=super::store::now(); let policy=format!("{}:blake3-1.8.7",crate::inspection::hashing::POLICY);
        tx.execute("INSERT INTO integrity_baselines(id,establishing_job_id,file_id,previous_id,root_id,object_key,native_relative,content_revision,algorithm,policy,digest,bytes,established_at,origin) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'blake3',?9,?10,?11,?12,'explicit_user_establishment')",params![id,job_id,file_id,previous,root_id,object,relative,revision,policy,digest,bytes,now])?;
        let baseline=tx.query_row("SELECT * FROM integrity_baselines WHERE id=?",[id],baseline_row)?;
        tx.commit()?; Ok(baseline)
    })
}

pub(crate) fn save_report(
    catalog: &Catalog,
    report: InspectionReport,
    folders: Vec<crate::inspection::FolderSize>,
    groups: Vec<(DuplicateEvidence, Vec<String>)>,
    issues: Vec<InspectionIssue>,
) -> Result<()> {
    catalog.call(move|db| {
        let job=load(db,&report.job_id)?;
        if job.state!=InspectionState::Running && job.state!=InspectionState::CancelRequested {return Err(invalid("Retain results only for this active inspection."));}
        let tx=db.transaction()?;
        for (ordinal,(evidence,mut ids)) in groups.into_iter().enumerate() {
            ids.sort(); ids.dedup(); if ids.len()<2 {continue;}
            let signature=serde_json::to_vec(&(&report.job_id,evidence,&ids)).map_err(|error|invalid("Could not bind the duplicate group.").caused_by(error))?;
            let id=blake3::hash(&signature).to_hex().to_string();
            let header=DuplicateGroup {id:id.clone(),evidence,member_ids:ids.iter().take(64).cloned().collect(),member_count:ids.len() as u32,members_truncated:ids.len()>64,mutation_available:false,explanation:match evidence {DuplicateEvidence::ExactBytes=>"Complete bytes compared; fresh verification and independent recovery are still required for quarantine.",DuplicateEvidence::PropertyCandidate=>"Property-rule candidates; this is not proof of equal content.",DuplicateEvidence::NormalizedPixels=>"Supported normalized pixels; manual review only, whole-file bytes may differ.",DuplicateEvidence::HardLinkAliases=>"Linked paths share one native object; they are not independent redundant copies."}.into()};
            tx.execute("INSERT INTO inspection_groups VALUES(?1,?2,?3,?4)",params![report.job_id,id,ordinal as i64,serde_json::to_string(&header).map_err(|error|invalid("Could not retain group header.").caused_by(error))?])?;
            for (index,file_id) in ids.into_iter().enumerate(){tx.execute("INSERT INTO inspection_group_members VALUES(?1,?2,?3,?4)",params![report.job_id,id,file_id,index as i64])?;}
        }
        for folder in folders {tx.execute("INSERT INTO inspection_folders VALUES(?1,?2,?3,?4,?5)",params![report.job_id,folder.id,folder.parent_id,folder.logical_bytes,serde_json::to_string(&folder).map_err(|error|invalid("Could not retain folder totals.").caused_by(error))?])?;}
        for (index,issue) in issues.into_iter().enumerate(){tx.execute("INSERT INTO inspection_issues VALUES(?1,?2,?3)",params![report.job_id,index as i64,serde_json::to_string(&issue).map_err(|error|invalid("Could not retain inspection issue.").caused_by(error))?])?;}
        tx.execute("INSERT INTO inspection_summaries VALUES(?1,?2)",params![report.job_id,serde_json::to_string(&report).map_err(|error|invalid("Could not retain inspection summary.").caused_by(error))?])?;
        tx.commit()?;Ok(())
    })
}
pub(crate) fn report(catalog: &Catalog, id: String) -> Result<Option<InspectionReport>> {
    crate::operations::validate_id(&id)?;
    catalog.call(move |db| {
        load(db, &id)?;
        let payload: Option<String> = db
            .query_row(
                "SELECT payload FROM inspection_summaries WHERE job_id=?",
                [&id],
                |row| row.get(0),
            )
            .optional()?;
        let result: Option<InspectionReport> =
            payload.map(|payload| decode(&payload)).transpose()?;
        if result.as_ref().is_some_and(|report| report.job_id != id) {
            return Err(invalid("The retained summary belongs to another job."));
        }
        Ok(result)
    })
}
pub(crate) fn groups(
    catalog: &Catalog,
    job_id: String,
    after: Option<String>,
) -> Result<DuplicatePage> {
    crate::operations::validate_id(&job_id)?;
    catalog.call(move|db|{load(db,&job_id)?;let cursor=match after{Some(id)=>db.query_row("SELECT ordinal FROM inspection_groups WHERE job_id=? AND id=?",params![job_id,id],|row|row.get::<_,i64>(0))?,None=>-1};let records=db.prepare("SELECT id,payload FROM inspection_groups WHERE job_id=? AND ordinal>? ORDER BY ordinal LIMIT 65")?.query_map(params![job_id,cursor],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;let more=records.len()>64;let mut groups=Vec::new();for(id,payload)in records.into_iter().take(64){let group:DuplicateGroup=decode(&payload)?;if group.id!=id||group.mutation_available||group.member_ids.len()>64||group.member_count<group.member_ids.len() as u32{return Err(invalid("The retained group header failed validation."));}groups.push(group);}let next=if more{groups.last().map(|group|group.id.clone())}else{None};Ok(DuplicatePage{job_id,groups,next})})
}
pub(crate) fn folders(
    catalog: &Catalog,
    job_id: String,
    parent: String,
    after: Option<String>,
) -> Result<FolderPage> {
    crate::operations::validate_id(&job_id)?;
    catalog.call(move|db|{
        load(db,&job_id)?;
        let payload:String=db.query_row("SELECT payload FROM inspection_folders WHERE job_id=? AND id=?",params![job_id,parent],|row|row.get(0))?;
        let parent_row:crate::inspection::FolderSize=decode(&payload)?;
        if parent_row.id!=parent{return Err(invalid("Retained folder identity differs."));}
        let cursor=match after {Some(id)=>Some(db.query_row("SELECT logical_bytes,id FROM inspection_folders WHERE job_id=? AND parent_id=? AND id=?",params![job_id,parent,id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?),None=>None};
        let records=if let Some((bytes,id))=cursor {
            db.prepare("SELECT id,payload FROM inspection_folders WHERE job_id=?1 AND parent_id=?2 AND (length(logical_bytes)<length(?3) OR (length(logical_bytes)=length(?3) AND logical_bytes<?3) OR (logical_bytes=?3 AND id>?4)) ORDER BY length(logical_bytes) DESC,logical_bytes DESC,id LIMIT 241")?.query_map(params![job_id,parent,bytes,id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?
        }else{db.prepare("SELECT id,payload FROM inspection_folders WHERE job_id=? AND parent_id=? ORDER BY length(logical_bytes) DESC,logical_bytes DESC,id LIMIT 241")?.query_map(params![job_id,parent],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?};
        let more=records.len()>240;let mut rows=Vec::new();for(id,payload)in records.into_iter().take(240){let row:crate::inspection::FolderSize=decode(&payload)?;if row.id!=id||row.parent_id.as_ref()!=Some(&parent){return Err(invalid("Retained child folder scope differs."));}rows.push(row);}
        let next=if more{rows.last().map(|row|row.id.clone())}else{None};Ok(FolderPage{job_id,parent:parent_row,rows,next})
    })
}

pub(crate) fn group_members(
    catalog: &Catalog,
    job_id: String,
    group_id: String,
    after: Option<String>,
) -> Result<FindingPage> {
    crate::operations::validate_id(&job_id)?;
    if group_id.len() != 64
        || !group_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid("Invalid retained group identity."));
    }
    catalog.call(move|db|{
        load(db,&job_id)?;
        let _:String=db.query_row("SELECT id FROM inspection_groups WHERE job_id=? AND id=?",params![job_id,group_id],|row|row.get(0))?;
        let cursor=match after{Some(id)=>db.query_row("SELECT ordinal FROM inspection_group_members WHERE job_id=? AND group_id=? AND file_id=?",params![job_id,group_id,id],|row|row.get::<_,i64>(0))?,None=>-1};
        let records=db.prepare("SELECT i.id,i.root_id,i.bytes,i.payload FROM inspection_group_members m JOIN inspection_items i ON i.job_id=m.job_id AND i.id=m.file_id WHERE m.job_id=? AND m.group_id=? AND m.ordinal>? ORDER BY m.ordinal LIMIT 241")?.query_map(params![job_id,group_id,cursor],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let more=records.len()>240;let mut rows=Vec::new();for(id,root,bytes,payload)in records.into_iter().take(240){let finding:InspectionFinding=decode(&payload)?;if finding.id!=id||finding.root_id!=root||finding.bytes!=bytes{return Err(invalid("Group member evidence differs from its retained source."));}rows.push(finding);}
        let next=if more{rows.last().map(|row|row.id.clone())}else{None};Ok(FindingPage{job_id,rows,next})
    })
}

pub(super) struct ReviewedMember {
    pub finding: InspectionFinding,
    pub relative: std::path::PathBuf,
    pub revision: String,
    pub object: String,
}
pub(super) fn reviewed_members(
    catalog: &Catalog,
    job: String,
    group: String,
) -> Result<(DuplicateEvidence, Vec<ReviewedMember>)> {
    crate::operations::validate_id(&job)?;
    if group.len() != 64 || !group.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid("Invalid group identity."));
    }
    catalog.call(move |db| {
        let task = load(db, &job)?;
        if !matches!(task.state, InspectionState::Completed | InspectionState::Partial) { return Err(invalid("Review a finished inspection report.")); }
        let header: String = db.query_row("SELECT payload FROM inspection_groups WHERE job_id=? AND id=?", params![job,group], |row|row.get(0))?;
        let header: DuplicateGroup = decode(&header)?;
        let mut statement = db.prepare("SELECT i.payload,i.native_relative,i.revision,i.object_key FROM inspection_group_members m JOIN inspection_items i ON i.job_id=m.job_id AND i.id=m.file_id WHERE m.job_id=? AND m.group_id=? ORDER BY m.ordinal LIMIT 1025")?;
        let mut query = statement.query(params![job,group])?;
        let mut members = Vec::new();
        let mut budget = 0usize;
        while let Some(row) = query.next()? {
            let payload: String = row.get(0)?;
            let relative: Vec<u8> = row.get(1)?;
            budget = budget.saturating_add(payload.len()).saturating_add(relative.len());
            if members.len() >= 1024 || budget > 16 * 1024 * 1024 { return Err(AppError::new(ErrorCode::Unsupported,"Keeper review exceeds its bounded membership budget; retain the group for manual review.")); }
            members.push(ReviewedMember { finding: decode(&payload)?, relative: super::native::decode_path(&relative)?, revision: row.get(2)?, object: row.get(3)? });
        }
        if members.len() != header.member_count as usize { return Err(invalid("Keeper membership differs from the retained group.")); }
        Ok((header.evidence, members))
    })
}
pub(super) fn save_keeper_review(
    catalog: &Catalog,
    review: crate::inspection::KeeperReview,
) -> Result<crate::inspection::KeeperReview> {
    let payload = serde_json::to_string(&review)
        .map_err(|error| invalid("Could not retain keeper review.").caused_by(error))?;
    catalog.call(move |db| {
        db.execute(
            "INSERT INTO inspection_keeper_reviews VALUES(?1,?2,?3,?4)",
            params![
                review.id,
                review.input.job_id,
                review.input.group_id,
                payload
            ],
        )?;
        Ok(review)
    })
}

pub(super) fn issues(
    catalog: &Catalog,
    job: String,
    after: Option<String>,
) -> Result<InspectionIssuePage> {
    crate::operations::validate_id(&job)?;
    let cursor = after
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| invalid("Invalid issue cursor."))
        })
        .transpose()?;
    catalog.call(move |db| {
        load(db,&job)?;
        let records=db.prepare("SELECT ordinal,payload FROM inspection_issues WHERE job_id=? AND ordinal>? ORDER BY ordinal LIMIT 65")?.query_map(params![job,cursor.map_or(-1,i64::from)],|row|Ok((row.get::<_,u32>(0)?,row.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let more=records.len()>64;let mut last=None;let mut rows=Vec::new();
        for(ordinal,payload)in records.into_iter().take(64){rows.push(decode(&payload)?);last=Some(ordinal.to_string());}
        Ok(InspectionIssuePage{job_id:job,rows,next:if more{last}else{None}})
    })
}
pub(super) fn latest_keeper_review(
    catalog: &Catalog,
    job: String,
    group: String,
) -> Result<Option<crate::inspection::KeeperReview>> {
    crate::operations::validate_id(&job)?;
    if group.len() != 64 || !group.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid("Invalid group identity."));
    }
    catalog.call(move |db| {
        let payload:Option<String>=db.query_row("SELECT payload FROM inspection_keeper_reviews WHERE job_id=? AND group_id=? ORDER BY rowid DESC LIMIT 1",params![job,group],|row|row.get(0)).optional()?;
        let review:Option<crate::inspection::KeeperReview>=payload.map(|payload|decode(&payload)).transpose()?;
        if review.as_ref().is_some_and(|review|review.input.job_id!=job||review.input.group_id!=group||review.mutation_available){return Err(invalid("Retained keeper review differs from its bound group."));}
        Ok(review)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(mode: HealthMode) -> InspectionInput {
        InspectionInput {
            root_ids: vec![uuid::Uuid::new_v4().to_string()],
            scope: crate::inspection::InspectionScope::IndexedMediaCompanions,
            task: InspectionTask::Health { mode },
        }
    }
    #[test]
    fn inspection_jobs_reconcile_metadata_without_replaying_and_preserve_input_and_terminal_history()
     {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        let created = create(&catalog, input(HealthMode::Quick)).unwrap();
        begin(&catalog, created.id.clone()).unwrap();
        let id = created.id.clone();
        assert!(
            catalog
                .call(move |db| {
                    db.execute("UPDATE inspection_jobs SET input='{}' WHERE id=?", [id])?;
                    Ok(())
                })
                .is_err()
        );
        recover(&catalog).unwrap();
        let interrupted = job(&catalog, created.id.clone()).unwrap();
        assert_eq!(interrupted.state, InspectionState::Interrupted);
        assert_eq!(
            serde_json::to_string(&created.input).unwrap(),
            serde_json::to_string(&interrupted.input).unwrap()
        );
        assert!(begin(&catalog, created.id.clone()).is_err());
        assert_eq!(jobs(&catalog).unwrap().len(), 1);
        let id = created.id;
        assert!(
            catalog
                .call(move |db| {
                    db.execute("DELETE FROM inspection_jobs WHERE id=?", [id])?;
                    Ok(())
                })
                .is_err()
        );
        assert_eq!(
            std::fs::read_dir(owned.path())
                .unwrap()
                .filter(|entry| entry
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "JPG"))
                .count(),
            0
        );
    }
    #[test]
    fn cancel_wins_the_terminal_commit_race_and_cannot_be_rewritten_as_success() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        let mut created = create(&catalog, input(HealthMode::Deep)).unwrap();
        begin(&catalog, created.id.clone()).unwrap();
        cancel(&catalog, created.id.clone()).unwrap();
        created.state = InspectionState::Completed;
        let terminal = finish(&catalog, created).unwrap();
        assert_eq!(terminal.state, InspectionState::Canceled);
        assert!(begin(&catalog, terminal.id.clone()).is_err());
        assert_eq!(
            cancel(&catalog, terminal.id).unwrap().state,
            InspectionState::Canceled
        );
    }
    #[test]
    fn baseline_history_appends_in_commit_order_and_never_replaces_previous_evidence() {
        use crate::{
            filesystem,
            inspection::candidates::Sources,
            jobs::resources::ResourceScheduler,
            library::{
                inspection_sources::NativeSources,
                model::{RootMode, ScanFilter},
                native,
            },
        };
        use std::sync::{Arc, atomic::AtomicBool};
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("source");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("Photo.JPG"), b"abc").unwrap();
        let catalog = Arc::new(Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap());
        let root = native::open_root(&path, None).unwrap();
        let root_id = super::super::store::add_root(
            &catalog,
            path,
            filesystem::identity(&root).unwrap(),
            native::volume_token(&root).unwrap(),
            "Generated baseline".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
        let mut sources = NativeSources::new(
            catalog.clone(),
            vec![root_id.clone()],
            ResourceScheduler::default(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        sources
            .capture(0, "Photo.JPG".into(), uuid::Uuid::new_v4().to_string())
            .unwrap();
        let entry = &sources.entries[0];
        let reader = sources.open(0).unwrap();
        sources.validate(0, &reader).unwrap();
        let mut previous = None;
        for _ in 0..2 {
            let input = InspectionInput {
                root_ids: vec![root_id.clone()],
                scope: crate::inspection::InspectionScope::IndexedMediaCompanions,
                task: InspectionTask::Health {
                    mode: HealthMode::EstablishBaseline,
                },
            };
            let job = create(&catalog, input).unwrap();
            begin(&catalog, job.id.clone()).unwrap();
            save_roots(&catalog, job.id.clone(), sources.roots.clone()).unwrap();
            let finding = entry.finding(&sources.roots[0], None, true, None).unwrap();
            insert_item(&catalog, job.id.clone(), 0, entry, finding).unwrap();
            let baseline = append_baseline(
                &catalog,
                job.id.clone(),
                entry,
                &sources.roots[0],
                blake3::hash(b"abc").to_hex().to_string(),
            )
            .unwrap();
            assert_eq!(baseline.previous_id, previous);
            previous = Some(baseline.id.clone());
            assert_eq!(
                latest_baseline(&catalog, entry.id.clone())
                    .unwrap()
                    .unwrap()
                    .id,
                baseline.id
            );
            assert!(
                append_baseline(
                    &catalog,
                    job.id.clone(),
                    entry,
                    &sources.roots[0],
                    blake3::hash(b"abc").to_hex().to_string()
                )
                .is_err()
            );
            let id = baseline.id.clone();
            assert!(
                catalog
                    .call(move |db| {
                        db.execute(
                            "UPDATE integrity_baselines SET digest=? WHERE id=?",
                            params!["0".repeat(64), id],
                        )?;
                        Ok(())
                    })
                    .is_err()
            );
            let id = baseline.id;
            assert!(
                catalog
                    .call(move |db| {
                        db.execute("DELETE FROM integrity_baselines WHERE id=?", [id])?;
                        Ok(())
                    })
                    .is_err()
            );
            let mut completed = job;
            completed.state = InspectionState::Completed;
            finish(&catalog, completed).unwrap();
        }
        let count: i64 = catalog
            .call(|db| {
                Ok(
                    db.query_row("SELECT count(*) FROM integrity_baselines", [], |row| {
                        row.get(0)
                    })?,
                )
            })
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            std::fs::read(owned.path().join("source/Photo.JPG")).unwrap(),
            b"abc"
        );
    }
}
