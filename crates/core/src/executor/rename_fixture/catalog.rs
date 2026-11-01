//! Actual catalog actor/path updates, restricted to this constructor-owned
//! fixture. No existing application catalog or arbitrary root can be supplied.
use super::*;
use crate::{
    catalog::Catalog,
    library::{
        model::{RootMode, ScanFilter},
        native as paths, store,
    },
};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(super) struct GeneratedCatalog {
    pub(super) db: Catalog,
    root_id: String,
    volume: String,
    source: super::super::Object,
    source_path: Vec<u8>,
    assets: BTreeMap<String, String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Row {
    pub member: String,
    pub asset: String,
    pub path: String,
    pub revision: String,
    pub native_file: String,
    pub favorite: bool,
    pub rating: u32,
    pub tags: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    policy: String,
    plan: String,
    digest: String,
    root: String,
    rows: Vec<CommittedRow>,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CommittedRow {
    member: String,
    asset: String,
    path: String,
    revision: String,
    native_file: String,
}
pub(super) struct Observation {
    pub rows: Vec<Row>,
    pub committed: bool,
}
fn stream(plan: &str) -> String {
    format!("generated-rename-catalog:{plan}")
}
impl GeneratedCatalog {
    pub(super) fn create(fixture: &Fixture) -> Result<Self> {
        let path = fixture.run.path().canonicalize()?;
        let directory = root(&path.join("originals"))?;
        let db = Catalog::open(path.join("rename-catalog.stillharbour"))?;
        db.call(|db| {
            db.execute_batch("CREATE UNIQUE INDEX generated_rename_commit_once ON job_events(job_id) WHERE job_id LIKE 'generated-rename-catalog:%';
            CREATE TRIGGER generated_rename_commit_no_update BEFORE UPDATE ON job_events WHEN OLD.job_id LIKE 'generated-rename-catalog:%' OR NEW.job_id LIKE 'generated-rename-catalog:%' BEGIN SELECT RAISE(ABORT,'Retain generated rename commits'); END;
            CREATE TRIGGER generated_rename_commit_no_delete BEFORE DELETE ON job_events WHEN OLD.job_id LIKE 'generated-rename-catalog:%' BEGIN SELECT RAISE(ABORT,'Retain generated rename commits'); END;")?; Ok(())
        })?;
        let volume = source_native::volume_token(directory.leaf())?;
        let root_id = store::add_root(
            &db,
            path.join("originals"),
            filesystem::identity(directory.leaf())?,
            volume.clone(),
            "Generated rename fixture only".into(),
            RootMode::Managed,
            ScanFilter::default(),
        )?;
        let mut assets = BTreeMap::new();
        for member in &fixture.plan.members {
            let original = &fixture.originals[&member.id];
            let rows = store::discover_batch(
                &db,
                root_id.clone(),
                uuid(),
                vec![store::Discovery {
                    relative: member.before.clone().into(),
                    revision: member.revision.clone(),
                    native_file: original.identity.file.to_string(),
                    links: original.identity.links,
                    bytes: original.bytes as i64,
                    modified: Some(
                        chrono::DateTime::<chrono::Utc>::from(original.modified).to_rfc3339(),
                    ),
                    born: original
                        .created
                        .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339()),
                    detected: None,
                    media: !member.before.ends_with(".xmp"),
                    placeholder: false,
                }],
            )?;
            if rows.len() != 1 {
                return Err(failed("Generated catalog discovery was incomplete."));
            }
            let asset = rows[0].0.clone();
            let tags = serde_json::to_string(&["generated-annotation", &member.id])
                .map_err(|error| failed("Invalid generated tags.").caused_by(error))?;
            db.call({
                let asset = asset.clone();
                move |db| {
                    db.execute(
                        "UPDATE assets SET favorite=1,rating=4,tags=? WHERE id=?",
                        params![tags, asset],
                    )?;
                    Ok(())
                }
            })?;
            assets.insert(member.id.clone(), asset);
        }
        Ok(Self {
            db,
            root_id,
            volume,
            source: fixture.source_root.clone(),
            source_path: paths::encode_path(&path.join("originals")),
            assets,
        })
    }
    fn rows(&self) -> Result<Vec<Row>> {
        let assets = self.assets.clone();
        let root = self.root_id.clone();
        let volume = self.volume.clone();
        let source = self.source.clone();
        let source_path = self.source_path.clone();
        self.db.call(move |db| {
            let valid: bool = db.query_row("SELECT count(*)=1 FROM roots WHERE id=? AND mode='managed' AND availability='online' AND volume=? AND device=? AND inode=? AND native_path=?", params![root,volume,source.volume.to_string(),source.file.to_string(),source_path], |row| row.get(0))?;
            if !valid { return Err(failed("Generated catalog root authority changed.")); }
            assets.into_iter().map(|(member,asset)| {
                let row = db.query_row("SELECT relative_path,relative_native,revision,native_file,favorite,rating,tags FROM assets WHERE id=? AND root_id=? AND presence='present'", params![asset,root], |row| Ok((row.get::<_,String>(0)?,row.get::<_,Vec<u8>>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,bool>(4)?,row.get::<_,u32>(5)?,row.get::<_,String>(6)?)))?;
                if !crate::organization::portable_filename(&row.0) || row.1 != paths::encode_path(Path::new(&row.0)) || row.6.len() > 8192 { return Err(failed("Generated catalog path or annotations changed incompatibly.")); }
                Ok(Row { member, asset, path: row.0, revision: row.2, native_file: row.3, favorite: row.4, rating: row.5, tags: row.6 })
            }).collect()
        })
    }
    pub(super) fn observe(
        &self,
        fixture: &Fixture,
        plan: &Plan,
        revisions: &BTreeMap<String, String>,
    ) -> Result<Observation> {
        let rows = self.rows()?;
        let job = stream(&plan.id);
        let receipts = self.db.call(move |db| {
            Ok(db
                .prepare("SELECT state FROM job_events WHERE job_id=? ORDER BY sequence LIMIT 2")?
                .query_map([job], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })?;
        if receipts.len() > 1 {
            return Err(failed("Ambiguous generated catalog commit history."));
        }
        let committed = if let Some(payload) = receipts.first() {
            if payload.len() > 65536 {
                return Err(failed("Generated commit receipt exceeds its bound."));
            }
            let receipt: Receipt = serde_json::from_str(payload).map_err(|error| {
                failed("Unsupported generated commit receipt.").caused_by(error)
            })?;
            let expected = Self::committed_rows(&rows);
            if receipt.policy != "generated-rename-catalog-v1"
                || receipt.plan != plan.id
                || receipt.digest != plan.digest
                || receipt.root != self.root_id
                || receipt.rows != expected
            {
                return Err(failed(
                    "Generated catalog commit receipt differs from current paths.",
                ));
            }
            true
        } else {
            false
        };
        for row in &rows {
            let member = plan
                .members
                .iter()
                .find(|member| member.id == row.member)
                .ok_or_else(|| failed("Foreign generated catalog member."))?;
            let expected_path = if committed {
                &member.after
            } else {
                &member.before
            };
            let expected_revision = if committed {
                &revisions[&member.id]
            } else {
                &member.revision
            };
            if &row.path != expected_path
                || &row.revision != expected_revision
                || row.native_file != fixture.originals[&member.id].identity.file.to_string()
            {
                return Err(failed(
                    "Generated catalog path, revision or object binding changed.",
                ));
            }
        }
        Ok(Observation { rows, committed })
    }
    fn committed_rows(rows: &[Row]) -> Vec<CommittedRow> {
        rows.iter()
            .map(|row| CommittedRow {
                member: row.member.clone(),
                asset: row.asset.clone(),
                path: row.path.clone(),
                revision: row.revision.clone(),
                native_file: row.native_file.clone(),
            })
            .collect()
    }
    pub(super) fn commit(
        &self,
        fixture: &Fixture,
        plan: &Plan,
        revisions: BTreeMap<String, String>,
    ) -> Result<()> {
        let before = self.observe(fixture, plan, &revisions)?;
        if before.committed {
            return Ok(());
        }
        let root = self.root_id.clone();
        let plan = plan.clone();
        let volume = self.volume.clone();
        let source = self.source.clone();
        let source_path = self.source_path.clone();
        self.db.call(move |db| {
            let tx = db.transaction()?;
            let valid: bool = tx.query_row("SELECT count(*)=1 FROM roots WHERE id=? AND mode='managed' AND availability='online' AND volume=? AND device=? AND inode=? AND native_path=?", params![root,volume,source.volume.to_string(),source.file.to_string(),source_path], |row| row.get(0))?;
            if !valid { return Err(failed("Generated catalog root changed before commit.")); }
            // Compare exact rows inside the transaction; no cycle may briefly
            // replace another asset or escape through an unchanged row count.
            for row in &before.rows {
                let unchanged: bool = tx.query_row("SELECT count(*)=1 FROM assets WHERE id=? AND root_id=? AND relative_native=? AND revision=? AND native_file=? AND presence='present' AND favorite=? AND rating=? AND tags=?", params![row.asset,root,paths::encode_path(Path::new(&row.path)),row.revision,row.native_file,row.favorite,row.rating,row.tags], |row| row.get(0))?;
                if !unchanged { return Err(failed("Generated catalog changed before its commit.")); }
            }
            let stage = uuid();
            for row in &before.rows {
                let leaf = format!(".stillharbour-catalog-{stage}-{}", row.asset);
                tx.execute("UPDATE assets SET relative_native=? WHERE id=?", params![paths::encode_path(Path::new(&leaf)),row.asset])?;
            }
            let mut after = before.rows;
            for row in &mut after {
                let member = plan.members.iter().find(|member| member.id == row.member).ok_or_else(|| failed("Unknown generated catalog target."))?;
                let leaf = Path::new(&member.after);
                tx.execute("UPDATE assets SET relative_native=?,relative_path=?,name=?,stem=?,extension=?,revision=? WHERE id=?", params![paths::encode_path(leaf),member.after,member.after,leaf.file_stem().unwrap_or_default().to_string_lossy(),leaf.extension().unwrap_or_default().to_string_lossy().to_ascii_lowercase(),revisions[&member.id],row.asset])?;
                row.path = member.after.clone(); row.revision = revisions[&member.id].clone();
            }
            let receipt = Receipt { policy: "generated-rename-catalog-v1".into(), plan: plan.id.clone(), digest: plan.digest, root, rows: Self::committed_rows(&after) };
            let payload = serde_json::to_string(&receipt).map_err(|error| failed("Cannot retain generated catalog commit.").caused_by(error))?;
            if payload.len() > 65536 { return Err(failed("Generated catalog commit exceeds its bound.")); }
            tx.execute("INSERT INTO job_events(job_id,state) VALUES(?,?)", params![stream(&plan.id),payload])?;
            tx.execute("UPDATE library_state SET revision=revision+1 WHERE singleton=1", [])?;
            tx.commit()?; Ok(())
        })
    }
}
