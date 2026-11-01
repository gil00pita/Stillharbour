//! Immutable catalog-scoped comparison bindings. Never persisted approvals.
use super::*;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

const MAX_PROFILES: u32 = 32;
pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE backup_sync_profiles(id TEXT PRIMARY KEY,record TEXT NOT NULL CHECK(length(CAST(record AS BLOB))<=65536));
      CREATE TRIGGER backup_sync_profiles_immutable_update BEFORE UPDATE ON backup_sync_profiles BEGIN SELECT RAISE(ABORT,'Comparison profile bindings are immutable'); END;
      CREATE TRIGGER backup_sync_profiles_immutable_delete BEFORE DELETE ON backup_sync_profiles BEGIN SELECT RAISE(ABORT,'Retain comparison profile bindings'); END;")?;
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supersedes: Option<String>,
    pub id: String,
    version: u32,
    catalog_id: String,
    platform: String,
    label: String,
    created_at: String,
    pub source_root_id: String,
    source_label: String,
    source_native: Vec<u8>,
    source_identity: FileIdentity,
    source_volume: String,
    destination_native: Vec<u8>,
    destination_identity: FileIdentity,
    destination_volume: String,
}
fn valid_label(label: &str) -> Result<String> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > 80 || label.chars().any(char::is_control) {
        return Err(error(
            ErrorCode::InvalidInput,
            "Use a profile name with 1–80 visible characters.",
        ));
    }
    Ok(label.into())
}
impl StoredProfile {
    fn validate(&self, catalog_id: &str) -> Result<()> {
        if let Some(id) = &self.supersedes {
            crate::operations::validate_id(id)?;
        }
        for id in [&self.id, &self.source_root_id, &self.catalog_id] {
            crate::operations::validate_id(id)?;
        }
        if self.version != 1
            || self.catalog_id != catalog_id
            || self.platform != std::env::consts::OS
        {
            return Err(error(
                ErrorCode::Unsupported,
                "This profile needs native reauthorization in its original catalog and platform.",
            ));
        }
        valid_label(&self.label)?;
        for bytes in [&self.source_native, &self.destination_native] {
            let path = native::decode_path(bytes)?;
            if !path.is_absolute()
                || path.components().count() > 96
                || path.components().any(|c| {
                    matches!(
                        c,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )
                })
            {
                return Err(changed());
            }
        }
        Ok(())
    }
    pub(super) fn validate_source(&self, source: &store::RootRecord) -> Result<()> {
        if source.view.id != self.source_root_id
            || native::encode_path(&source.path) != self.source_native
            || !same_directory(&source.identity, &self.source_identity)
            || source.volume != self.source_volume
        {
            return Err(error(
                ErrorCode::Conflict,
                "The profile's source binding changed. Review and save a new profile; no default-library fallback is allowed.",
            ));
        }
        Ok(())
    }
    fn view(&self) -> Result<SyncProfile> {
        Ok(SyncProfile {
            supersedes: self.supersedes.clone(),
            id: self.id.clone(),
            version: self.version,
            catalog_id: self.catalog_id.clone(),
            label: self.label.clone(),
            source_root_id: self.source_root_id.clone(),
            source_label: self.source_label.clone(),
            source_display_path: native::decode_path(&self.source_native)?
                .to_string_lossy()
                .into_owned(),
            destination_display_path: native::decode_path(&self.destination_native)?
                .to_string_lossy()
                .into_owned(),
            created_at: self.created_at.clone(),
        })
    }
}
fn load(catalog: &crate::catalog::Catalog, id: String) -> Result<StoredProfile> {
    crate::operations::validate_id(&id)?;
    let catalog_id = catalog.summary()?.id;
    catalog.call(move |db| {
        let text: String = db.query_row("SELECT record FROM backup_sync_profiles WHERE id=? AND length(CAST(record AS BLOB))<=65536", [&id], |row| row.get(0)).optional()?.ok_or_else(|| error(ErrorCode::InvalidInput, "Choose an existing comparison profile."))?;
        if text.len() > 65536 { return Err(changed()); }
        let profile: StoredProfile = serde_json::from_str(&text).map_err(|_| changed())?;
        if profile.id != id { return Err(changed()); }
        profile.validate(&catalog_id)?;
        Ok(profile)
    })
}
impl Library {
    pub fn backup_profiles(&self) -> Result<Vec<SyncProfile>> {
        let catalog_id = self.catalog.summary()?.id;
        self.catalog.call(move |db| {
            let mut statement = db.prepare("SELECT id,length(CAST(record AS BLOB)),CASE WHEN length(CAST(record AS BLOB))<=65536 THEN record END FROM backup_sync_profiles ORDER BY rowid LIMIT 33")?;
            let mut rows = statement.query([])?;
            let mut profiles = Vec::new();
            while let Some(row) = rows.next()? {
                if profiles.len() >= MAX_PROFILES as usize { return Err(error(ErrorCode::Unsupported, "Comparison profile inventory exceeds the supported limit.")); }
                let id: String = row.get(0)?;
                if !(0..=65536).contains(&row.get::<_, i64>(1)?) { return Err(changed()); }
                let text: String = row.get(2)?;
                if text.len() > 65536 { return Err(changed()); }
                let profile: StoredProfile = serde_json::from_str(&text).map_err(|_| changed())?;
                profile.validate(&catalog_id)?;
                if profile.id != id { return Err(changed()); }
                profiles.push(profile.view()?);
            }
            Ok(profiles)
        })
    }
    pub fn backup_profile_save(
        &self,
        selection: PathBuf,
        expected: (FileIdentity, String),
        label: String,
    ) -> Result<SyncProfile> {
        self.save_comparison_profile(selection, expected, label, None)
    }
    pub fn backup_profile_rebind(
        &self,
        prior_id: String,
        selection: PathBuf,
        expected: (FileIdentity, String),
        label: String,
        reviewed: bool,
    ) -> Result<SyncProfile> {
        if !reviewed {
            return Err(error(
                ErrorCode::InvalidInput,
                "Review the old and new profile scope before rebinding.",
            ));
        }
        let prior = load(&self.catalog, prior_id)?;
        self.save_comparison_profile(selection, expected, label, Some(prior.id))
    }
    fn save_comparison_profile(
        &self,
        selection: PathBuf,
        expected: (FileIdentity, String),
        label: String,
        supersedes: Option<String>,
    ) -> Result<SyncProfile> {
        let label = valid_label(&label)?;
        let scope = self.comparison_scope(selection, expected, None)?;
        let profile = StoredProfile {
            supersedes,
            id: uuid::Uuid::new_v4().to_string(),
            version: 1,
            catalog_id: scope.catalog_id.clone(),
            platform: std::env::consts::OS.into(),
            label,
            created_at: store::now(),
            source_root_id: scope.base.view.id.clone(),
            source_label: scope.base.view.label.clone(),
            source_native: native::encode_path(&scope.source.path),
            source_identity: scope.source.identity.clone(),
            source_volume: scope.source.volume.clone(),
            destination_native: native::encode_path(&scope.destination.path),
            destination_identity: scope.destination.identity.clone(),
            destination_volume: scope.destination.volume.clone(),
        };
        profile.validate(&scope.catalog_id)?;
        let text = serde_json::to_string(&profile).map_err(|_| changed())?;
        if text.len() > 65536 {
            return Err(error(
                ErrorCode::Unsupported,
                "Profile native-path record exceeds 64 KiB.",
            ));
        }
        let result = profile.view()?;
        self.catalog.call(move |db| {
            let tx = db.transaction()?;
            let count: u32 = tx.query_row("SELECT count(*) FROM backup_sync_profiles", [], |row| row.get(0))?;
            if count >= MAX_PROFILES { return Err(error(ErrorCode::InvalidInput, "This catalog already has 32 immutable comparison profiles.")); }
            // The picker is not a grant to retarget a concurrent catalog relink.
            let inserted = tx.execute("INSERT INTO backup_sync_profiles(id,record) SELECT ?1,?2 FROM roots WHERE id=?3 AND native_path=?4 AND volume=?5 AND device=?6 AND inode=?7 AND is_base=1 AND availability='online'", params![profile.id,text,profile.source_root_id,profile.source_native,profile.source_volume,profile.source_identity.volume.to_string(),profile.source_identity.file.to_string()])?;
            if inserted != 1 { return Err(changed()); }
            tx.commit()?;
            Ok(())
        })?;
        // Holding this lease through the transaction does not grant write access.
        drop(scope);
        Ok(result)
    }
    pub fn backup_profile_compare(
        self: &Arc<Self>,
        id: String,
        mode: CompareMode,
    ) -> Result<ComparisonRun> {
        let profile = load(&self.catalog, id)?;
        let path = native::decode_path(&profile.destination_native)?;
        let expected = (
            profile.destination_identity.clone(),
            profile.destination_volume.clone(),
        );
        self.start_comparison(path, expected, mode, Some(profile))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_comparison_records_reject_foreign_catalog_platform_version_and_approvals() {
        let owned = tempfile::tempdir().unwrap();
        let catalog =
            crate::catalog::Catalog::open(owned.path().join("Fixture.stillharbour")).unwrap();
        let catalog_id = catalog.summary().unwrap().id;
        let path = owned.path().canonicalize().unwrap();
        for mutation in ["catalog", "platform", "version", "approval", "path"] {
            let id = uuid::Uuid::new_v4().to_string();
            let record = StoredProfile {
                supersedes: None,
                id: id.clone(),
                version: 1,
                catalog_id: catalog_id.clone(),
                platform: std::env::consts::OS.into(),
                label: "Generated record".into(),
                created_at: store::now(),
                source_root_id: uuid::Uuid::new_v4().to_string(),
                source_label: "Generated source".into(),
                source_native: native::encode_path(&path),
                source_identity: FileIdentity {
                    volume: 1,
                    file: 2,
                    links: 1,
                },
                source_volume: "fixture".into(),
                destination_native: native::encode_path(&path),
                destination_identity: FileIdentity {
                    volume: 1,
                    file: 3,
                    links: 1,
                },
                destination_volume: "fixture".into(),
            };
            let mut value = serde_json::to_value(record).unwrap();
            match mutation {
                "catalog" => value["catalog_id"] = uuid::Uuid::new_v4().to_string().into(),
                "platform" => value["platform"] = "foreign".into(),
                "version" => value["version"] = 2.into(),
                "approval" => value["approval"] = true.into(),
                _ => {
                    value["source_native"] =
                        serde_json::to_value(native::encode_path(Path::new("../outside"))).unwrap()
                }
            }
            let text = serde_json::to_string(&value).unwrap();
            let inserted_id = id.clone();
            catalog
                .call(move |db| {
                    db.execute(
                        "INSERT INTO backup_sync_profiles VALUES(?,?)",
                        params![inserted_id, text],
                    )?;
                    Ok(())
                })
                .unwrap();
            assert!(load(&catalog, id).is_err(), "{mutation}");
        }
    }
}
