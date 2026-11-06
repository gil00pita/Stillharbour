//! Immutable bounded observation history; never recovery or execution evidence.
use super::*;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

const LIMIT: usize = 16 * 1024 * 1024;
pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE backup_comparison_history(id TEXT PRIMARY KEY,scope TEXT NOT NULL,record TEXT NOT NULL CHECK(length(CAST(record AS BLOB))<=16777216));
      CREATE INDEX backup_comparison_history_scope ON backup_comparison_history(scope);
      CREATE TRIGGER backup_comparison_history_update BEFORE UPDATE ON backup_comparison_history BEGIN SELECT RAISE(ABORT,'Retain comparison observations'); END;
      CREATE TRIGGER backup_comparison_history_delete BEFORE DELETE ON backup_comparison_history BEGIN SELECT RAISE(ABORT,'Retain comparison observations'); END;")?;
    Ok(())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub id: String,
    scope: String,
    version: u32,
    /// Hashes of complete freshly revalidated destination reads, not byte equality.
    pub files: BTreeMap<String, String>,
}
pub(super) fn scope(source: &Binding, destination: &Binding, namespace: &Path) -> Result<String> {
    let mut h = blake3::Hasher::new();
    h.update(b"m4a-observation-history-v1");
    for bytes in [
        native::encode_path(&source.path),
        native::encode_path(&destination.path),
        native::encode_path(namespace),
        format!("{}:{}", source.identity.volume, source.identity.file).into_bytes(),
        format!(
            "{}:{}",
            destination.identity.volume, destination.identity.file
        )
        .into_bytes(),
    ] {
        h.update(&(bytes.len() as u64).to_le_bytes());
        h.update(&bytes);
    }
    h.update(source.volume.as_bytes());
    h.update(destination.volume.as_bytes());
    Ok(h.finalize().to_hex().to_string())
}
pub(super) fn key(path: &Path) -> String {
    blake3::hash(&native::encode_path(path))
        .to_hex()
        .to_string()
}
pub(super) fn load(catalog: &crate::catalog::Catalog, scope: &str) -> Result<Option<Manifest>> {
    let scope = scope.to_owned();
    catalog.call(move |db| {
        let record: Option<(String,i64,Option<String>)> = db.query_row("SELECT id,length(CAST(record AS BLOB)),CASE WHEN length(CAST(record AS BLOB))<=16777216 THEN record END FROM backup_comparison_history WHERE scope=? ORDER BY rowid DESC LIMIT 1", [&scope], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let Some((id,length,text)) = record else { return Ok(None) };
        if !(0..=LIMIT as i64).contains(&length) {return Err(changed());}
        let text=text.ok_or_else(changed)?;
        if text.len()>LIMIT { return Err(changed()); }
        let manifest: Manifest = serde_json::from_str(&text).map_err(|_| changed())?;
        crate::operations::validate_id(&manifest.id)?;
        if manifest.id!=id || manifest.version!=1 || manifest.scope!=scope || manifest.files.len()>MAX_ENTRIES || manifest.files.iter().any(|(a,b)| [a,b].iter().any(|s| s.len()!=64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))) { return Err(changed()); }
        Ok(Some(manifest))
    })
}
pub(super) fn save(
    catalog: &crate::catalog::Catalog,
    id: &str,
    scope: String,
    files: BTreeMap<String, String>,
) -> Result<String> {
    let manifest = Manifest {
        id: id.into(),
        scope: scope.clone(),
        version: 1,
        files,
    };
    let text = serde_json::to_string(&manifest).map_err(|_| changed())?;
    if text.len() > LIMIT {
        return Err(error(
            ErrorCode::Unsupported,
            "Comparison history exceeds its size bound.",
        ));
    }
    catalog.call(move |db| {
        let tx=db.transaction()?;
        let count:u32=tx.query_row("SELECT count(*) FROM backup_comparison_history WHERE scope!='utility-report-v1'",[],|r|r.get(0))?;
        if count>=32 { return Ok("History storage is full (32 immutable observations); existing history is retained. This comparison was not saved.".into()); }
        tx.execute("INSERT INTO backup_comparison_history(id,scope,record) VALUES(?,?,?)",params![manifest.id,scope,text])?;
        tx.commit()?;
        Ok("Saved complete destination byte-digest observations as historical context only; not equality, fidelity, approval or recovery evidence.".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparison_history_is_bounded_immutable_and_rejects_imported_authority() {
        let owned = tempfile::tempdir().unwrap();
        let catalog =
            crate::catalog::Catalog::open(owned.path().join("History.stillharbour")).unwrap();
        let scope = "a".repeat(64);
        let id = uuid::Uuid::new_v4().to_string();
        save(&catalog, &id, scope.clone(), BTreeMap::new()).unwrap();
        assert_eq!(load(&catalog, &scope).unwrap().unwrap().id, id);
        let foreign = scope.clone();
        catalog.call(move |db| {
            let forged=serde_json::json!({"id":uuid::Uuid::new_v4().to_string(),"scope":foreign,"version":1,"files":{},"approved":true});
            db.execute("INSERT INTO backup_comparison_history(id,scope,record) VALUES(?,?,?)",params![uuid::Uuid::new_v4().to_string(),foreign,forged.to_string()])?; Ok(())
        }).unwrap();
        assert!(load(&catalog, &scope).is_err());
        for n in 2..32 {
            save(
                &catalog,
                &uuid::Uuid::new_v4().to_string(),
                format!("scope-{n}"),
                BTreeMap::new(),
            )
            .unwrap();
        }
        let message = save(
            &catalog,
            &uuid::Uuid::new_v4().to_string(),
            "full".into(),
            BTreeMap::new(),
        )
        .unwrap();
        assert!(message.contains("not saved"));
        catalog
            .call(|db| {
                assert_eq!(
                    db.query_row("SELECT count(*) FROM backup_comparison_history", [], |r| {
                        r.get::<_, u32>(0)
                    })?,
                    32
                );
                Ok(())
            })
            .unwrap();
    }
}
