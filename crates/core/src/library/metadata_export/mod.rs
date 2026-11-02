//! Explicit new-output metadata adapter. Never writes an original or sidecar.
mod inventory;
pub mod model;
mod portable;
mod xmp;
use super::{Library, filters::invalid, model::AssetSelection, native, review_export, store};
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, DirectoryChain, FileIdentity},
};
use model::*;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    fs::File,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    root: String,
    relative: Vec<u8>,
    identity: FileIdentity,
    revision: String,
    digest: String,
    content_required: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    session: String,
    selection: String,
    path: Vec<u8>,
    identity: FileIdentity,
    volume: String,
    kind: MetadataExportKind,
    files: Vec<MetadataExportFile>,
    sources: Vec<Source>,
    inventories: Vec<inventory::Inventory>,
}
fn conflict() -> AppError {
    AppError::new(
        ErrorCode::Conflict,
        "The reviewed export, source metadata or destination changed. Preview again; every existing file remains retained.",
    )
}
fn equal(a: &FileIdentity, b: &FileIdentity) -> bool {
    a.volume == b.volume && a.file == b.file
}
fn hash(file: &mut File, limit: u64, cancel: &AtomicBool) -> Result<String> {
    if file.metadata()?.len() > limit {
        return Err(invalid("This metadata artifact exceeds its read bound."));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut h = blake3::Hasher::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = review_export::read_export(file, &mut buffer, Some(cancel))?;
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(h.finalize().to_hex().to_string())
}
fn retain(path: &Path, bytes: &[u8], cancel: &AtomicBool) -> Result<MetadataExportFile> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    review_export::write_export(&mut file, bytes, Some(cancel))?;
    file.sync_all()?;
    Ok(MetadataExportFile {
        name: path.file_name().unwrap().to_string_lossy().into_owned(),
        bytes: bytes.len().to_string(),
        digest: blake3::hash(bytes).to_hex().to_string(),
    })
}
impl Library {
    pub fn cancel_metadata_export(&self) {
        self.metadata_export_cancel.store(true, Ordering::Release);
    }
    fn export_lease(&self, path: &Path) -> Result<crate::jobs::operation_locks::Lease> {
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let mut scopes = vec![Scope::new(
            review_export::output_chain(&path.canonicalize()?)?,
            Access::Write,
        )];
        for root in store::roots(&self.catalog)? {
            if root.path.exists() {
                let chain = DirectoryChain::open(&root.path.canonicalize()?)?;
                if !equal(chain.identity(), &root.identity) {
                    return Err(conflict());
                }
                scopes.push(Scope::new(chain, Access::Read));
            }
        }
        Registry::shared().try_acquire(scopes)
    }
    fn export_directory(
        &self,
        path: &Path,
        expected: &(FileIdentity, String),
    ) -> Result<DirectoryChain> {
        let root = native::open_root(path, Some(&expected.0))?;
        if native::volume_token(&root)? != expected.1 {
            return Err(conflict());
        }
        drop(root);
        let directory = review_export::output_chain(&path.canonicalize()?)?;
        if !equal(directory.identity(), &expected.0) {
            return Err(conflict());
        }
        let app = DirectoryChain::open(&self.options.data_directory.canonicalize()?)?;
        if directory.overlaps(&app) {
            return Err(invalid("Choose an export folder outside application data."));
        }
        for root in store::roots(&self.catalog)? {
            match root
                .path
                .canonicalize()
                .and_then(|p| DirectoryChain::open(&p).map_err(std::io::Error::other))
            {
                Ok(chain) => {
                    if !equal(chain.identity(), &root.identity)
                        || native::volume_token(chain.leaf())? != root.volume
                    {
                        return Err(conflict());
                    }
                    if directory.overlaps(&chain) {
                        return Err(invalid(
                            "Export outside indexed libraries and their ancestors.",
                        ));
                    }
                }
                Err(_) => {
                    // An unavailable library is retained as an exclusion, never an
                    // empty source. Reconstruct only its missing namespace under
                    // a held, accessible ancestor; no original is enumerated.
                    let mut ancestor = root.path.clone();
                    let mut suffix = Vec::new();
                    let canonical = loop {
                        match ancestor.canonicalize() {
                            Ok(p) => break p,
                            Err(error) => {
                                if error.kind() != std::io::ErrorKind::NotFound {
                                    return Err(AppError::new(
                                        ErrorCode::Unsupported,
                                        "An inaccessible root prevents export overlap verification.",
                                    ));
                                }
                                let leaf = ancestor
                                    .file_name()
                                    .ok_or_else(|| {
                                        invalid("An offline root has no verifiable ancestor.")
                                    })?
                                    .to_owned();
                                suffix.push(leaf);
                                if !ancestor.pop() {
                                    return Err(invalid(
                                        "An offline root has no local verifiable ancestor.",
                                    ));
                                }
                            }
                        }
                    };
                    let _ancestor = DirectoryChain::open(&canonical)?;
                    let mut excluded = canonical;
                    for leaf in suffix.iter().rev() {
                        excluded.push(leaf);
                    }
                    use unicode_normalization::UnicodeNormalization;
                    let key = |value: &Path| {
                        value
                            .to_string_lossy()
                            .nfc()
                            .flat_map(char::to_lowercase)
                            .collect::<String>()
                    };
                    let a = key(&path.canonicalize()?);
                    let b = key(&excluded);
                    let sep = std::path::MAIN_SEPARATOR;
                    if a == b
                        || a.starts_with(&format!("{b}{sep}"))
                        || b.starts_with(&format!("{a}{sep}"))
                    {
                        return Err(invalid(
                            "The export folder overlaps an unavailable library namespace.",
                        ));
                    }
                }
            }
        }
        Ok(directory)
    }
    fn export_work(&self, id: &str) -> Result<PathBuf> {
        uuid::Uuid::parse_str(id).map_err(|_| invalid("Invalid export identity."))?;
        Ok(self
            .options
            .data_directory
            .join("metadata-exports")
            .join(id))
    }
    pub fn metadata_export_preview(
        &self,
        kind: MetadataExportKind,
        selection: Option<AssetSelection>,
        expected_count: u32,
        selection_id: String,
        path: PathBuf,
        expected: (FileIdentity, String),
    ) -> Result<MetadataExportReview> {
        let _gate = self.destination_gate.try_lock().map_err(|_| {
            AppError::new(ErrorCode::Busy, "Another destination operation is running.")
        })?;
        let _held = self.export_directory(&path, &expected)?;
        self.metadata_export_cancel.store(false, Ordering::Release);
        let _lease = self.export_lease(&path)?;
        let path = path.canonicalize()?;
        let id = uuid::Uuid::new_v4().to_string();
        let work = self.export_work(&id)?;
        let mut directory = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            directory.mode(0o700);
        }
        directory.recursive(true).create(work.parent().unwrap())?;
        directory.recursive(false).create(&work)?;
        let revision = store::revision(&self.catalog)?;
        let mut files = Vec::new();
        let mut sources = Vec::new();
        let mut inventories = Vec::new();
        match kind {
            MetadataExportKind::Catalog => {
                let name = format!("stillharbour-catalog-{id}.stillharbour");
                let output = work.join(&name);
                let folder = work.clone();
                let build = output.clone();
                let expected_revision = revision.clone();
                self.catalog.call(move |db| {
                    let current: String = db.query_row(
                        "SELECT CAST(revision AS TEXT) FROM library_state",
                        [],
                        |r| r.get(0),
                    )?;
                    if current != expected_revision {
                        return Err(conflict());
                    }
                    portable::build(db, &folder, &build)
                })?;
                let mut file = File::open(&output)?;
                files.push(MetadataExportFile {
                    name,
                    bytes: file.metadata()?.len().to_string(),
                    digest: hash(&mut file, 512 * 1024 * 1024, &self.metadata_export_cancel)?,
                });
            }
            MetadataExportKind::Xmp => {
                let selection = selection
                    .ok_or_else(|| invalid("Select catalog media before previewing XMP export."))?;
                if expected_count == 0 || expected_count > 200 {
                    return Err(invalid(
                        "Select between one and 200 media for an XMP export.",
                    ));
                }
                let ids=self.catalog.call(move|db|{match &selection {AssetSelection::Ids{ids}=>{if ids.iter().collect::<std::collections::HashSet<_>>().len()!=ids.len(){return Err(invalid("Selection repeats asset identities."));}},AssetSelection::Token{id}=>{if !db.prepare("SELECT 1 FROM selections WHERE id=? AND julianday(created_at)>=julianday('now','-1 hour')")?.exists([id])?{return Err(conflict());}}}let(scope,values)=store::selection_scope(db,selection)?;let ids=db.prepare(&format!("SELECT id FROM assets WHERE is_media=1 AND {scope} ORDER BY id LIMIT 201"))?.query_map(rusqlite::params_from_iter(values),|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;if ids.len()!=expected_count as usize{return Err(invalid("Selection changed; no partial export was prepared."));}Ok(ids)})?;
                for asset in ids {
                    self.prepare_xmp(
                        &id,
                        &asset,
                        &work,
                        &mut files,
                        &mut sources,
                        &mut inventories,
                    )?;
                    if files.len() > 1000
                        || files
                            .iter()
                            .map(|v| v.bytes.parse::<u64>().unwrap_or(u64::MAX))
                            .try_fold(0u64, |a, b| a.checked_add(b))
                            .is_none_or(|n| n > 64 * 1024 * 1024)
                    {
                        return Err(invalid(
                            "XMP export exceeds 1000 files or 64 MiB. Use a smaller reviewed selection.",
                        ));
                    }
                }
                let manifest = serde_json::json!({"version":1,"catalogId":self.catalog.summary()?.id,"catalogRevision":revision,"originalsIncluded":false,"association":"Every asset UUID has a distinct XMP filename. Use this mapping; no basename attachment is inferred.","files":files,"sources":sources.iter().map(|s|serde_json::json!({"rootId":s.root,"relativeNative":s.relative,"digest":s.digest,"revision":s.revision,"contentVerification":s.content_required})).collect::<Vec<_>>()});
                files.push(retain(
                    &work.join(format!("stillharbour-metadata-{id}-manifest.json")),
                    serde_json::to_string_pretty(&manifest)
                        .map_err(|_| invalid("Invalid export manifest."))?
                        .as_bytes(),
                    &self.metadata_export_cancel,
                )?);
            }
        }
        if store::revision(&self.catalog)? != revision {
            return Err(conflict());
        }
        let plan = Plan {
            session: self.metadata_export_session.clone(),
            selection: selection_id,
            path: native::encode_path(&path),
            identity: expected.0,
            volume: expected.1,
            kind,
            files: files.clone(),
            sources,
            inventories,
        };
        let payload =
            serde_json::to_string(&plan).map_err(|_| invalid("Invalid metadata review."))?;
        if payload.len() > 16 * 1024 * 1024 {
            return Err(invalid("The export manifest exceeds its review bound."));
        }
        let digest = blake3::hash(payload.as_bytes()).to_hex().to_string();
        let key = id.clone();
        let rev = revision.clone();
        let plan_digest = digest.clone();
        self.catalog.call(move |db| {
            db.execute(
                "INSERT INTO catalog_metadata_plans VALUES(?,'export',?,?,?,?,'review')",
                params![key, rev, plan_digest, payload, store::now()],
            )?;
            Ok(())
        })?;
        Ok(MetadataExportReview{id,digest,revision,export_kind:kind,destination:path.to_string_lossy().into_owned(),files,message:if kind==MetadataExportKind::Catalog{"This unencrypted catalog contains private paths, GPS and organization, but no originals, credentials, schedules, approvals or native root authority. Roots require reauthorization; snapshots are not live shared catalogs."}else{"New XMP copies and original companion artifacts only. Unknown XMP fields are preserved; existing source/destination files are never replaced. Review the UUID association manifest before importing into another application."}.into()})
    }
    fn prepare_xmp(
        &self,
        export: &str,
        asset: &str,
        work: &Path,
        files: &mut Vec<MetadataExportFile>,
        sources: &mut Vec<Source>,
        inventories: &mut Vec<inventory::Inventory>,
    ) -> Result<()> {
        let summary = store::asset(&self.catalog, asset.to_owned())?;
        let root = store::root(&self.catalog, summary.root_id.clone())?;
        if root.view.availability != "online" {
            return Err(invalid(
                "Reconnect the selected media root to verify its sidecar associations.",
            ));
        }
        let (_, relative, revision) = store::asset_native(&self.catalog, asset.to_owned())?;
        let original = native::open_asset(&root.path, &root.identity, &root.volume, &relative)?;
        if native::revision(&original)? != revision {
            return Err(conflict());
        }
        sources.push(Source {
            root: summary.root_id.clone(),
            relative: native::encode_path(&relative),
            identity: filesystem::identity(&original)?,
            revision,
            digest: String::new(),
            content_required: false,
        });
        let (companions, peers, signature) = inventory::inspect(&root, &relative, &summary.name)?;
        inventories.push(inventory::Inventory {
            root: summary.root_id.clone(),
            relative: native::encode_path(&relative),
            source_name: summary.name.clone(),
            signature,
        });
        let (caption,rejected)=self.catalog.call({let id=asset.to_owned();move|db|Ok(db.query_row("SELECT coalesce((SELECT caption FROM asset_notes WHERE asset_id=?1),''),coalesce((SELECT rejected FROM asset_notes WHERE asset_id=?1),0)",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,bool>(1)?)))?)})?;
        let mut xmp = None;
        for (index, companion) in companions.iter().enumerate() {
            let extension = &companion.extension;
            if extension == "xmp" && (xmp.is_some() || (peers > 1 && !companion.filename_specific))
            {
                return Err(invalid(
                    "XMP association is ambiguous across variants or sidecars. Resolve the source association before exporting; no basename guess was used.",
                ));
            }
            let root_id = summary.root_id.clone();
            let relative = companion.relative.clone();
            let mut file = native::open_asset(&root.path, &root.identity, &root.volume, &relative)?;
            let revision = native::revision(&file)?;
            let identity = filesystem::identity(&file)?;
            let limit = if extension == "xmp" {
                1024 * 1024
            } else {
                8 * 1024 * 1024
            };
            let digest = hash(&mut file, limit, &self.metadata_export_cancel)?;
            let mut bytes = Vec::new();
            let mut block = [0u8; 65536];
            loop {
                let n = review_export::read_export(
                    &mut file,
                    &mut block,
                    Some(&self.metadata_export_cancel),
                )?;
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&block[..n]);
                if bytes.len() > limit as usize {
                    return Err(conflict());
                }
            }
            if blake3::hash(&bytes).to_hex().as_str() != digest {
                return Err(conflict());
            }
            if native::revision(&file)? != revision {
                return Err(conflict());
            }
            if extension == "xmp" {
                xmp = Some(bytes.clone());
            }
            files.push(retain(
                &work.join(format!(
                    "stillharbour-metadata-{export}-{asset}-original-{index}.{}",
                    match extension.as_str() {
                        "xmp" => "xmp",
                        "json" => "json",
                        "acr" => "acr",
                        "aae" => "aae",
                        "dop" => "dop",
                        "pp3" => "pp3",
                        _ => "bin",
                    }
                )),
                &bytes,
                &self.metadata_export_cancel,
            )?);
            sources.push(Source {
                root: root_id,
                relative: native::encode_path(&relative),
                identity,
                revision,
                digest,
                content_required: true,
            });
        }
        let tags: Vec<String> = summary.tags;
        let bytes = xmp::merge(
            xmp.as_deref(),
            &tags,
            &caption,
            summary.rating,
            rejected,
            summary.favorite,
            asset,
        )?;
        files.push(retain(
            &work.join(format!("stillharbour-metadata-{export}-{asset}.xmp")),
            &bytes,
            &self.metadata_export_cancel,
        )?);
        let mapping = serde_json::json!({"assetId":asset,"rootId":summary.root_id,"relativePath":summary.relative_path,"xmpFile":format!("stillharbour-metadata-{export}-{asset}.xmp"),"sourceCompanions":companions.iter().map(|c|&c.name).collect::<Vec<_>>()});
        files.push(retain(
            &work.join(format!(
                "stillharbour-metadata-{export}-{asset}-association.json"
            )),
            serde_json::to_string_pretty(&mapping).unwrap().as_bytes(),
            &self.metadata_export_cancel,
        )?);
        Ok(())
    }
    pub fn metadata_export_save(
        &self,
        id: String,
        digest: String,
        selection_id: String,
        path: PathBuf,
        expected: (FileIdentity, String),
    ) -> Result<MetadataExportSaved> {
        let _gate = self.destination_gate.try_lock().map_err(|_| {
            AppError::new(ErrorCode::Busy, "Another destination operation is running.")
        })?;
        let directory = self.export_directory(&path, &expected)?;
        self.metadata_export_cancel.store(false, Ordering::Release);
        let _lease = self.export_lease(&path)?;
        let work = self.export_work(&id)?;
        let key = id.clone();
        let plan_digest = digest.clone();
        let (revision,raw)=self.catalog.call(move|db|db.query_row("SELECT revision,payload FROM catalog_metadata_plans WHERE length(CAST(payload AS BLOB))<=16777216 AND id=? AND digest=? AND kind='export' AND state='review' AND julianday(created_at)>=julianday('now','-1 hour')",params![key,plan_digest],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?.ok_or_else(conflict))?;
        let plan: Plan = serde_json::from_str(&raw).map_err(|_| conflict())?;
        if blake3::hash(raw.as_bytes()).to_hex().as_str() != digest
            || plan.session != self.metadata_export_session
            || plan.selection != selection_id
            || plan.path != native::encode_path(&path.canonicalize()?)
            || !equal(&plan.identity, &expected.0)
            || plan.volume != expected.1
            || revision != store::revision(&self.catalog)?
        {
            return Err(conflict());
        }
        for file in &plan.files {
            if !file.name.starts_with(&format!(
                "stillharbour-{}-{id}",
                if plan.kind == MetadataExportKind::Catalog {
                    "catalog"
                } else {
                    "metadata"
                }
            )) || file.name.contains(['/', '\\', ':'])
            {
                return Err(conflict());
            }
            let mut source = filesystem::open_readonly(&work.join(&file.name))?;
            if hash(&mut source, 512 * 1024 * 1024, &self.metadata_export_cancel)? != file.digest
                || source.metadata()?.len().to_string() != file.bytes
            {
                return Err(conflict());
            }
        }
        self.verify_export_sources(&plan.sources)?;
        self.verify_export_inventory(&plan.inventories)?;
        let key = id.clone();
        let approved_revision = revision.clone();
        self.catalog.call(move|db|{let tx=db.transaction()?;let current:String=tx.query_row("SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",[],|r|r.get(0))?;if current!=approved_revision{return Err(conflict());}let changed=tx.execute("UPDATE catalog_metadata_plans SET state='applied' WHERE id=? AND state='review'",[&key])?;if changed!=1{return Err(conflict());}tx.execute("INSERT INTO catalog_export_journal(export_id,phase,evidence,created_at) VALUES(?,'approved',?,?)",params![key,raw,store::now()])?;tx.commit()?;Ok(())})?;
        #[cfg(test)]
        self.export_test_exit("approved");
        for file in &plan.files {
            self.export_event(&id, "file_intent", serde_json::to_string(file).unwrap())?;
            let mut source = filesystem::open_readonly(&work.join(&file.name))?;
            let stage = format!(".stillharbour-review-{}.partial", uuid::Uuid::new_v4());
            if let Err(error) = review_export::save_owned_file(
                &path,
                &directory,
                &stage,
                &file.name,
                &mut source,
                &self.metadata_export_cancel,
            ) {
                self.export_event(&id, "needs_attention", format!("{:?}", error.code))?;
                return Err(error);
            }
            #[cfg(test)]
            self.export_test_exit("published");
            self.export_event(&id, "file_verified", file.digest.clone())?;
            #[cfg(test)]
            self.export_test_exit("file_verified");
        }
        self.verify_export_sources(&plan.sources)?;
        self.verify_export_inventory(&plan.inventories)?;
        let current = review_export::output_chain(&path.canonicalize()?)?;
        if !equal(current.identity(), directory.identity()) {
            return Err(conflict());
        }
        self.export_event(&id, "completed", digest)?;
        Ok(MetadataExportSaved{id,files:plan.files.len() as u32,destination:path.to_string_lossy().into_owned(),message:"New export files were independently verified. Existing files and originals remain unchanged.".into()})
    }
    #[cfg(test)]
    fn export_test_exit(&self, phase: &str) {
        if std::env::var("STILLHARBOUR_M5_EXPORT_EXIT").as_deref() != Ok(phase) {
            return;
        }
        let root = self.options.data_directory.parent().unwrap();
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("stillharbour-m5-export-")
        );
        let token = std::env::var("STILLHARBOUR_M5_EXPORT_TOKEN").unwrap();
        assert!(uuid::Uuid::parse_str(&token).is_ok());
        assert_eq!(
            std::fs::read_to_string(root.join("m5-owner")).unwrap(),
            token
        );
        std::process::exit(92);
    }
    fn export_event(&self, id: &str, phase: &str, evidence: String) -> Result<()> {
        let id = id.to_owned();
        let phase = phase.to_owned();
        self.catalog.call(move|db|{db.execute("INSERT INTO catalog_export_journal(export_id,phase,evidence,created_at) VALUES(?,?,?,?)",params![id,phase,evidence,store::now()])?;Ok(())})
    }
    fn verify_export_inventory(&self, inventories: &[inventory::Inventory]) -> Result<()> {
        for entry in inventories {
            let root = store::root(&self.catalog, entry.root.clone())?;
            let (_, _, signature) = inventory::inspect(
                &root,
                &native::decode_path(&entry.relative)?,
                &entry.source_name,
            )?;
            if signature != entry.signature {
                return Err(conflict());
            }
        }
        Ok(())
    }
    fn verify_export_sources(&self, sources: &[Source]) -> Result<()> {
        for source in sources {
            let root = store::root(&self.catalog, source.root.clone())?;
            let mut file = native::open_asset(
                &root.path,
                &root.identity,
                &root.volume,
                &native::decode_path(&source.relative)?,
            )?;
            if filesystem::identity(&file)? != source.identity
                || native::revision(&file)? != source.revision
                || (source.content_required
                    && hash(&mut file, 8 * 1024 * 1024, &self.metadata_export_cancel)?
                        != source.digest)
            {
                return Err(conflict());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
