//! Catalog-actor adapter for the central engine. Currently constructor-owned
//! fixture capability only; retained records are never execution approvals.
use super::{CopyJournal, CopyPlan, Object, failed};
use crate::{
    catalog::Catalog,
    domain::{ErrorCode, Result},
    filesystem::{self, FileIdentity},
    library::{copy_manifest, operation_store},
    organization::portable_filename,
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

const PLAN_PHASE: &str = "copy_execution_plan_v1";
const EVENT_PHASE: &str = "copy_execution_event_v1";
const MAX_RECORD: usize = 65_536;
const MAX_EVENT: usize = 8192;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    policy: String,
    catalog_id: String,
    owner_id: String,
    owner_digest: String,
    manifest_id: String,
    manifest_digest: String,
    plan: CopyPlan,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    ordinal: u64,
    stream_digest: String,
    item: Option<usize>,
    phase: String,
    evidence: serde_json::Value,
}
#[derive(Clone, Default)]
struct Item {
    phase: u8,
    object: Option<Object>,
    digest: Option<String>,
}
#[derive(Clone)]
struct History {
    count: u64,
    items: Vec<Item>,
    terminal: bool,
}
fn hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn decode<T: serde::de::DeserializeOwned>(text: &str, max: usize) -> Result<T> {
    if text.len() > max {
        return Err(failed("Retained copy journal evidence exceeds its bound."));
    }
    serde_json::from_str(text)
        .map_err(|error| failed("Invalid retained copy journal evidence.").caused_by(error))
}
impl History {
    fn new(plan: &CopyPlan) -> Self {
        Self {
            count: 0,
            items: vec![Item::default(); plan.files.len()],
            terminal: false,
        }
    }
    fn apply(&mut self, event: &Event, plan: &CopyPlan) -> Result<()> {
        if self.terminal
            || event.ordinal != self.count + 1
            || event.stream_digest != plan.digest
            || event.ordinal > plan.files.len() as u64 * 6 + 3
        {
            return Err(failed("Invalid copy journal order or stream binding."));
        }
        if let Some(index) = event.item {
            let file = plan
                .files
                .get(index)
                .ok_or_else(|| failed("Unknown journal file index."))?;
            let item = &mut self.items[index];
            let expected = match event.phase.as_str() {
                "item_intent" => 0,
                "stage_created" => 1,
                "stage_flushed" => 2,
                "stage_verified" => 3,
                "publish_intent" => 4,
                "item_completed_verified" => 5,
                _ => return Err(failed("Unknown file journal phase.")),
            };
            if item.phase != expected {
                return Err(failed(
                    "Copy journal skipped or repeated a required file phase.",
                ));
            }
            match event.phase.as_str() {
                "item_intent" => {
                    if event.evidence
                        != serde_json::json!({"stage":file.stage,"target":file.target,"sourceRevision":file.revision})
                    {
                        return Err(failed("Copy intent differs from its immutable plan."));
                    }
                }
                "stage_created" => {
                    let object: Object = serde_json::from_value(event.evidence.clone())
                        .map_err(|error| failed("Invalid stage identity.").caused_by(error))?;
                    if object.volume != plan.destination.volume
                        || serde_json::to_value(&object).ok() != Some(event.evidence.clone())
                    {
                        return Err(failed("Unexpected stage volume or identity evidence."));
                    }
                    item.object = Some(object);
                }
                "stage_flushed" => {
                    if event.evidence != serde_json::json!({"bytes":file.bytes.to_string()}) {
                        return Err(failed("Flushed bytes differ from the reviewed size."));
                    }
                }
                "stage_verified" => {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields, rename_all = "camelCase")]
                    struct Verified {
                        bytes: String,
                        digest: String,
                        copy_revision: String,
                    }
                    let verified: Verified = serde_json::from_value(event.evidence.clone())
                        .map_err(|error| {
                            failed("Invalid verification evidence.").caused_by(error)
                        })?;
                    if verified.bytes != file.bytes.to_string()
                        || !hex(&verified.digest)
                        || !hex(&verified.copy_revision)
                    {
                        return Err(failed("Invalid byte verification scope."));
                    }
                    item.digest = Some(verified.digest);
                }
                "publish_intent" => {
                    let digest = item
                        .digest
                        .as_ref()
                        .ok_or_else(|| failed("Missing staged verification."))?;
                    if event.evidence
                        != serde_json::json!({"stage":file.stage,"target":file.target,"digest":digest})
                    {
                        return Err(failed("Publication differs from verified staged content."));
                    }
                }
                "item_completed_verified" => {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields, rename_all = "camelCase")]
                    struct Completed {
                        digest: String,
                        copy_revision: String,
                        object: Object,
                    }
                    let completed: Completed = serde_json::from_value(event.evidence.clone())
                        .map_err(|error| {
                            failed("Invalid completed verification.").caused_by(error)
                        })?;
                    if item.digest.as_ref() != Some(&completed.digest)
                        || item.object.as_ref() != Some(&completed.object)
                        || !hex(&completed.copy_revision)
                    {
                        return Err(failed(
                            "Completed output differs from the verified owned object.",
                        ));
                    }
                }
                _ => unreachable!(),
            }
            item.phase += 1;
        } else {
            match event.phase.as_str() {
                "plan_completed_verified"
                    if self.items.iter().all(|item| item.phase == 6)
                        && event.evidence == serde_json::json!({"files":plan.files.len()}) => {}
                "canceled_with_partial_copies" | "needs_attention" => {}
                _ => return Err(failed("Invalid plan completion or terminal journal phase.")),
            }
            self.terminal = true;
        }
        self.count = event.ordinal;
        Ok(())
    }
}

pub(super) struct CatalogJournal {
    catalog: Arc<Catalog>,
    binding: Binding,
    encoded: String,
    anchor: i64,
    history: Mutex<History>,
}
fn validate_binding(db: &Connection, binding: &Binding) -> Result<()> {
    crate::operations::validate_id(&binding.owner_id)?;
    crate::operations::validate_id(&binding.manifest_id)?;
    crate::operations::validate_id(&binding.plan.id)?;
    let draft = operation_store::load(db, &binding.owner_id)?;
    let manifest = copy_manifest::load(db, &binding.owner_id, &binding.manifest_id)?;
    let plan = &binding.plan;
    if binding.policy != "catalog-copy-stream-v1"
        || binding.catalog_id != draft.catalog_id
        || binding.owner_digest != draft.view.digest
        || binding.manifest_digest != manifest.view.digest
        || plan.digest()? != plan.digest
        || !manifest.view.issues.is_empty()
        || manifest.files.len() != plan.files.len()
        || plan.files.is_empty()
        || plan.destination.volume != draft.destination_device
        || plan.destination.file != draft.destination_file
        || plan.destination_volume != draft.destination_volume
    {
        return Err(failed(
            "Copy stream no longer matches its retained draft/manifest.",
        ));
    }
    let root_id = &manifest.files[0].view.root_id;
    let (device, inode, volume, mode): (String, String, String, String) = db.query_row(
        "SELECT device,inode,volume,mode FROM roots WHERE id=?",
        [root_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    if device != plan.source.volume.to_string()
        || inode != plan.source.file.to_string()
        || volume != plan.source_volume
        || mode != "managed"
    {
        return Err(failed(
            "The copy source root requires its unchanged managed write grant.",
        ));
    }
    for (index, (expected, file)) in manifest.files.iter().zip(&plan.files).enumerate() {
        let native_source = if let Some(location) = &file.source_location {
            location.validate()?;
            location.relative_native.clone()
        } else {
            crate::library::native::encode_path(Path::new(&file.source))
        };
        let target = if let Some(location) = &file.destination_location {
            location.validate()?;
            format!("{}/{}", location.relative_parent, file.target)
        } else {
            file.target.clone()
        };
        if &expected.view.root_id != root_id
            || (file.source_location.is_none() && !portable_filename(&file.source))
            || !portable_filename(&file.target)
            || expected.relative_native != native_source
            || expected.view.target_relative_path != target
            || expected.view.bytes != file.bytes.to_string()
            || expected.view.source_revision != file.revision
            || expected.identity != (file.object.volume, file.object.file)
            || file.stage != format!(".stillharbour-{}-{index}.partial", plan.id)
        {
            return Err(failed(
                "This copy stream requires exact single-root native source and destination mappings.",
            ));
        }
    }
    Ok(())
}
impl CatalogJournal {
    pub(super) fn bind(
        catalog: Arc<Catalog>,
        owner_id: String,
        manifest_id: String,
        plan: CopyPlan,
    ) -> Result<Self> {
        let supplied = plan.clone();
        let owner = owner_id.clone();
        let manifest = manifest_id.clone();
        let (binding,encoded,anchor,history)=catalog.call(move |db| {
            let tx=db.transaction()?;
            let draft=operation_store::load(&tx,&owner)?;
            let scope=copy_manifest::load(&tx,&owner,&manifest)?;
            let binding=Binding{policy:"catalog-copy-stream-v1".into(),catalog_id:draft.catalog_id,owner_id:owner,owner_digest:draft.view.digest,manifest_id:manifest,manifest_digest:scope.view.digest,plan:supplied};
            validate_binding(&tx,&binding)?;
            let encoded=serde_json::to_string(&binding).map_err(|error|failed("Could not retain copy stream.").caused_by(error))?;
            if encoded.len()>MAX_RECORD {return Err(failed("Copy stream exceeds its 64-KiB retained record bound."));}
            let previous={
                let mut query=tx.prepare("SELECT sequence,CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND item_id=?3 ORDER BY sequence LIMIT 2")?;
                let mut rows=query.query(params![binding.owner_id,PLAN_PHASE,binding.plan.id])?;
                let previous=if let Some(row)=rows.next()? {Some((row.get::<_,i64>(0)?,row.get::<_,Option<String>>(1)?))} else {None};
                if rows.next()?.is_some() {return Err(failed("Multiple immutable copy-stream anchors require review."));}
                previous
            };
            let anchor=if let Some((sequence,previous))=previous {
                if previous.as_ref()!=Some(&encoded) {return Err(failed("The immutable copy stream changed or exceeds its bound."));} sequence
            } else {
                tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![binding.owner_id,binding.plan.id,PLAN_PHASE,encoded,crate::library::store::now()])?;
                tx.last_insert_rowid()
            };
            let mut history=History::new(&binding.plan);
            {
                let mut query=tx.prepare("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=8192 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND item_id=?3 ORDER BY sequence LIMIT ?4")?;
                let mut rows=query.query(params![binding.owner_id,EVENT_PHASE,binding.plan.id,binding.plan.files.len() as i64*6+4])?;
                while let Some(row)=rows.next()? {
                    let text:Option<String>=row.get(0)?;
                    let text=text.ok_or_else(||failed("Retained copy event exceeds 8 KiB."))?;
                    let event:Event=decode(&text,MAX_EVENT)?;
                    history.apply(&event,&binding.plan)?;
                }
            }
            tx.commit()?;Ok((binding,encoded,anchor,history))
        })?;
        Ok(Self {
            catalog,
            binding,
            encoded,
            anchor,
            history: Mutex::new(history),
        })
    }
}
impl CopyJournal for CatalogJournal {
    fn append(&self, item: Option<usize>, phase: &str, evidence: serde_json::Value) -> Result<()> {
        let mut history = self
            .history
            .lock()
            .map_err(|_| failed("The copy journal state is unavailable."))?;
        let event = Event {
            ordinal: history.count + 1,
            stream_digest: self.binding.plan.digest.clone(),
            item,
            phase: phase.into(),
            evidence,
        };
        let mut proposed = history.clone();
        proposed.apply(&event, &self.binding.plan)?;
        let text = serde_json::to_string(&event)
            .map_err(|error| failed("Could not encode copy event.").caused_by(error))?;
        if text.len() > MAX_EVENT {
            return Err(failed("Copy event exceeds its 8-KiB bound."));
        }
        let anchor = self.anchor;
        let encoded = self.encoded.clone();
        let owner = self.binding.owner_id.clone();
        let stream = self.binding.plan.id.clone();
        let count = history.count;
        self.catalog.call(move |db| {
            let tx=db.transaction()?;
            let unchanged:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operation_journal WHERE sequence=?1 AND plan_id=?2 AND phase=?3 AND item_id=?4 AND evidence=?5)",params![anchor,owner,PLAN_PHASE,stream,encoded],|row|row.get(0))?;
            if !unchanged {return Err(failed("Copy stream anchor changed before journaling."));}
            let existing:i64=tx.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=?1 AND phase=?2 AND item_id=?3",params![owner,EVENT_PHASE,stream],|row|row.get(0))?;
            if u64::try_from(existing).ok()!=Some(count) {return Err(failed("Another writer changed this copy stream. Reopen recovery inspection."));}
            tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![owner,stream,EVENT_PHASE,text,crate::library::store::now()])?;
            tx.commit()?;Ok(())
        })?;
        *history = proposed;
        Ok(())
    }
    fn history_count(&self) -> Result<u64> {
        Ok(self
            .history
            .lock()
            .map_err(|_| failed("Copy history is unavailable."))?
            .count)
    }
    fn stage_identity(&self, item: usize) -> Result<Option<FileIdentity>> {
        let history = self
            .history
            .lock()
            .map_err(|_| failed("Copy history is unavailable."))?;
        let item = history
            .items
            .get(item)
            .ok_or_else(|| failed("Unknown copy history item."))?;
        Ok(item.object.as_ref().map(|object| FileIdentity {
            volume: object.volume,
            file: object.file,
            links: 1,
        }))
    }
}

fn generated_photo_bytes() -> Result<Vec<u8>> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(16, 8, image::Rgb([19, 43, 67]))
        .write_to(&mut bytes, image::ImageFormat::Jpeg)
        .map_err(|error| failed("Could not generate owned journal media.").caused_by(error))?;
    Ok(bytes.into_inner())
}
const COMPANION_BYTES: &[u8] = b"<x:xmpmeta>generated owned companion</x:xmpmeta>";
const NEIGHBOR_BYTES: &[u8] = b"retained unrelated destination";
struct Fixture {
    library: Arc<crate::library::Library>,
    _run: tempfile::TempDir,
    source: std::path::PathBuf,
    destination: std::path::PathBuf,
    owner: String,
    manifest: String,
    plan: CopyPlan,
    baseline: Vec<(String, Vec<u8>)>,
}
impl Fixture {
    #[cfg(test)]
    fn new() -> Result<Self> {
        Self::with_nested(false)
    }
    fn with_nested(nested: bool) -> Result<Self> {
        use crate::library::{
            Library, LibraryOptions, WorkerPaths,
            model::{RootMode, ScanFilter},
            native, store,
        };
        use crate::operations::{
            BundleConfirmationInput, BundleRelation, CopyManifestInput, PlanDraftInput,
        };
        use crate::organization::{MissingDatePolicy, OrganizationTemplate};
        let run = tempfile::Builder::new()
            .prefix("stillharbour-catalog-copy-")
            .tempdir()?;
        let base = run.path().canonicalize()?;
        let source = base.join("source");
        let data = base.join("data");
        let destination = base.join("output");
        for path in [&source, &data, &destination] {
            std::fs::create_dir(path)?;
        }
        let source_folder = if nested {
            source.join("source-folder")
        } else {
            source.clone()
        };
        if nested {
            std::fs::create_dir(&source_folder)?;
            std::fs::create_dir(destination.join("organized"))?;
        }
        std::fs::write(source_folder.join("Photo.JPG"), generated_photo_bytes()?)?;
        std::fs::write(source_folder.join("Photo.xmp"), COMPANION_BYTES)?;
        std::fs::write(destination.join("destination-only.bin"), NEIGHBOR_BYTES)?;
        let baseline = ["Photo.JPG", "Photo.xmp"]
            .into_iter()
            .map(|name| {
                Ok((
                    if nested {
                        format!("source-folder/{name}")
                    } else {
                        name.to_owned()
                    },
                    std::fs::read(source_folder.join(name))?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour"))?);
        let library = Library::new(
            catalog.clone(),
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: std::env::current_exe()?,
                    exiftool: None,
                    working_directory: data.join("worker"),
                },
            },
        )?;
        let root = native::open_root(&source, None)?;
        let source_object = Object::of(&root)?;
        let identity = filesystem::identity(&root)?;
        let source_volume = native::volume_token(&root)?;
        let root_id = store::add_root(
            &catalog,
            source.clone(),
            identity.clone(),
            source_volume.clone(),
            "Owned catalog copy fixture".into(),
            RootMode::Managed,
            ScanFilter::default(),
        )?;
        let relative = if nested {
            "source-folder/Photo.JPG"
        } else {
            "Photo.JPG"
        };
        let file = native::open_asset(&source, &identity, &source_volume, Path::new(relative))?;
        let revision = native::revision(&file)?;
        let file_id = filesystem::identity(&file)?;
        let bytes = file.metadata()?.len();
        drop(file);
        let asset = uuid::Uuid::new_v4().to_string();
        let inserted = asset.clone();
        let inserted_revision = revision.clone();
        // Fixture-only catalog inventory, just as the existing generated metadata
        // tests use. This does not claim a scan/decoder/user-flow acceptance.
        catalog.call(move |db| {
            db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,?2,?3,?8,?4,?9,'Photo.JPG','Photo','jpg',?5,?6,?7,'now','2026-10-31T00:00:00','filesystem_modified','ready','pending','present',1,'owned-fixture')",params![inserted,root_id,native::encode_path(Path::new(relative)),native::encode_path(Path::new(if nested {"source-folder"} else {""})),format!("{}:{}",file_id.volume,file_id.file),bytes as i64,inserted_revision,relative,if nested {"source-folder"}else{""}])?;
            db.execute("UPDATE library_state SET revision=revision+1",[])?;Ok(())
        })?;
        let template = OrganizationTemplate {
            pattern: if nested {
                "organized/{original_stem}.{ext}"
            } else {
                "{original_stem}.{ext}"
            }
            .into(),
            event: String::new(),
            missing_date: MissingDatePolicy::Block,
        };
        let preview = library.organization_preview(vec![asset.clone()], template.clone())?;
        let target = crate::library::native::open_root(&destination, None)?;
        let target_identity = filesystem::identity(&target)?;
        let destination_volume = native::volume_token(&target)?;
        let draft = library.organization_plan_create(
            PlanDraftInput {
                asset_ids: vec![asset.clone()],
                template,
                preview_fingerprint: preview.fingerprint,
                template_reference: None,
            },
            destination.clone(),
            (target_identity, destination_volume.clone()),
        )?;
        let inventory = library.organization_bundle_inventory(asset.clone(), revision.clone())?;
        let confirmation = library.organization_bundle_confirm(BundleConfirmationInput {
            plan_id: draft.plan.id.clone(),
            asset_id: asset,
            source_revision: revision,
            inventory_fingerprint: inventory.fingerprint,
            companion_ids: inventory
                .entries
                .iter()
                .filter(|entry| entry.selectable && entry.relation != BundleRelation::Additional)
                .map(|entry| entry.id.clone())
                .collect(),
            reviewed_all_entries: true,
        })?;
        let manifest = library.organization_copy_manifest(CopyManifestInput {
            plan_id: draft.plan.id.clone(),
            bundle_confirmation_ids: vec![confirmation.id],
        })?;
        let stream = uuid::Uuid::new_v4().to_string();
        let mut files = Vec::new();
        let owner = draft.plan.id.clone();
        let selected = manifest.id.clone();
        let private = catalog.call(move |db| copy_manifest::load(db, &owner, &selected))?;
        for (index, (item, native_file)) in manifest.files.iter().zip(&private.files).enumerate() {
            let source_location = if nested {
                Some(super::locations::Source::capture(
                    &root,
                    &native::decode_path(&native_file.relative_native)?,
                )?)
            } else {
                None
            };
            let file = if let Some(location) = &source_location {
                location.open(&root)?
            } else {
                super::native::read(&root, &item.source_name)?
            };
            files.push(super::CopyFile {
                source: item.source_name.clone(),
                target: if nested {
                    Path::new(&item.target_relative_path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| failed("Invalid fixture target."))?
                        .into()
                } else {
                    item.target_relative_path.clone()
                },
                stage: format!(".stillharbour-{stream}-{index}.partial"),
                object: Object::of(&file)?,
                revision: native::revision(&file)?,
                bytes: file.metadata()?.len(),
                source_location,
                destination_location: if nested {
                    Some(super::locations::Destination::capture(
                        &target,
                        "organized",
                    )?)
                } else {
                    None
                },
            });
        }
        let mut plan = CopyPlan {
            id: stream,
            digest: String::new(),
            source: source_object,
            destination: Object::of(&target)?,
            source_volume,
            destination_volume,
            files,
        };
        plan.digest = plan.digest()?;
        Ok(Self {
            library,
            _run: run,
            source,
            destination,
            owner: draft.plan.id,
            manifest: manifest.id,
            plan,
            baseline,
        })
    }
    fn scope(
        &self,
    ) -> Result<(
        crate::filesystem::DirectoryChain,
        crate::filesystem::DirectoryChain,
    )> {
        let source = crate::filesystem::DirectoryChain::open(&self.source)?;
        let output = super::output_chain(&self.destination)?;
        if !self.plan.source.matches(source.leaf())?
            || !self.plan.destination.matches(output.leaf())?
            || crate::library::native::volume_token(source.leaf())? != self.plan.source_volume
            || crate::library::native::volume_token(output.leaf())? != self.plan.destination_volume
        {
            return Err(failed("Owned catalog fixture scope changed."));
        }
        Ok((source, output))
    }
    fn journal(&self) -> Result<CatalogJournal> {
        CatalogJournal::bind(
            self.library.catalog.clone(),
            self.owner.clone(),
            self.manifest.clone(),
            self.plan.clone(),
        )
    }
    fn preserved(&self) -> Result<bool> {
        for (name, bytes) in &self.baseline {
            if std::fs::read(self.source.join(name))? != *bytes {
                return Ok(false);
            }
        }
        Ok(
            std::fs::read(self.destination.join("destination-only.bin"))?
                == b"retained unrelated destination",
        )
    }
    fn execute(
        &self,
        journal: &CatalogJournal,
        fault: Option<super::CopyFault>,
    ) -> Result<super::CopyOutcome> {
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let (source, output) = self.scope()?;
        let lease = Registry::shared().try_acquire(vec![
            Scope::new(source, Access::Read),
            Scope::new(output, Access::Write),
        ])?;
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let validate = || self.scope().map(|_| ());
        let io = crate::jobs::paced_io::PacedIo::shared();
        super::CopyContext {
            plan: &self.plan,
            journal,
            source_root: lease.directory(0).leaf(),
            destination: lease.directory(1).leaf(),
            io: &io,
            cancel: &cancel,
            validate_scope: &validate,
        }
        .execute(fault)
    }
    fn inspect(&self, journal: &CatalogJournal) -> Result<Vec<super::RecoveredItem>> {
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let (source, output) = self.scope()?;
        let lease = Registry::shared().try_acquire(vec![
            Scope::new(source, Access::Read),
            Scope::new(output, Access::Read),
        ])?;
        let io = crate::jobs::paced_io::PacedIo::shared();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        self.plan
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                super::inspect_item(
                    file,
                    index,
                    journal,
                    lease.directory(0).leaf(),
                    lease.directory(1).leaf(),
                    &io,
                    &cancel,
                )
            })
            .collect()
    }
}
pub(super) fn fixture_probe() -> Result<serde_json::Value> {
    let mut cases = Vec::new();
    for (nested, fault) in [
        (false, None),
        (false, Some(super::CopyFault::AfterPublish)),
        (true, None),
    ] {
        let fixture = Fixture::with_nested(nested)?;
        let journal = fixture.journal()?;
        let outcome = fixture.execute(&journal, fault)?;
        drop(journal);
        let reopened = fixture.journal()?;
        let states = fixture.inspect(&reopened)?;
        let preserved = fixture.preserved()?;
        let rejected = fixture
            .execute(&reopened, None)
            .is_err_and(|error| error.code == ErrorCode::Conflict);
        let verified = states
            .iter()
            .filter(|state| **state == super::RecoveredItem::PublishedVerified)
            .count();
        if !preserved
            || !rejected
            || verified
                != if fault.is_none() {
                    fixture.plan.files.len()
                } else {
                    1
                }
        {
            return Err(failed(
                "Owned catalog-journal fixture failed its actual-state checks.",
            ));
        }
        cases.push(serde_json::json!({"outcome":outcome,"publishedVerified":verified,"sourceAndNeighborPreserved":preserved,"replayBlocked":rejected,"journalEvents":reopened.history_count()?}));
    }
    Ok(
        serde_json::json!({"scope":"generated-temporary-only","catalogSchema":crate::catalog::SCHEMA_VERSION,"completed":cases[0],"interruptedAfterPublish":cases[1],"nested":cases[2]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn crash_boundaries() -> [super::super::CopyFault; 9] {
        use super::super::CopyFault::*;
        [
            AfterIntent,
            AfterStageCreation,
            AfterCopyChunk,
            AfterFlush,
            AfterVerification,
            BeforePublish,
            AfterPublish,
            AfterItemCommit,
            AfterPlanCommit,
        ]
    }
    #[test]
    fn catalog_crash_fixture_child() {
        use std::io::Read;
        let Some(root) = std::env::var_os("STILLHARBOUR_M2_CATALOG_CHILD_ROOT") else {
            return;
        };
        let root = std::path::PathBuf::from(root).canonicalize().unwrap();
        let token = std::env::var("STILLHARBOUR_M2_CATALOG_CHILD_TOKEN").unwrap();
        uuid::Uuid::parse_str(&token).unwrap();
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("stillharbour-catalog-copy-")
        );
        let run_scope = crate::filesystem::DirectoryChain::open(&root).unwrap();
        let marker = super::super::native::read(run_scope.leaf(), "fixture-owner").unwrap();
        assert_eq!(marker.metadata().unwrap().len(), token.len() as u64);
        assert_eq!(filesystem::identity(&marker).unwrap().links, 1);
        let mut owner = Vec::new();
        marker
            .take(token.len() as u64 + 1)
            .read_to_end(&mut owner)
            .unwrap();
        assert_eq!(owner, token.as_bytes());
        let source = crate::filesystem::DirectoryChain::open(&root.join("source")).unwrap();
        let output = super::super::output_chain(&root.join("output")).unwrap();
        let _data = crate::filesystem::DirectoryChain::open(&root.join("data")).unwrap();
        // Validate the fixed constructor-owned source bytes through held native
        // parents, with fixed-size buffers; never adopt supplied library paths.
        for (parent, name, expected) in [
            (source.leaf(), "Photo.JPG", generated_photo_bytes().unwrap()),
            (source.leaf(), "Photo.xmp", COMPANION_BYTES.to_vec()),
            (
                output.leaf(),
                "destination-only.bin",
                NEIGHBOR_BYTES.to_vec(),
            ),
        ] {
            let file = super::super::native::read(parent, name).unwrap();
            assert_eq!(file.metadata().unwrap().len(), expected.len() as u64);
            assert_eq!(filesystem::identity(&file).unwrap().links, 1);
            let mut bytes = Vec::new();
            file.take(expected.len() as u64 + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            assert_eq!(bytes, expected);
        }
        let database = root.join("data/Fixture.stillharbour");
        let proof = crate::filesystem::open_readonly(&database).unwrap();
        assert!(proof.metadata().unwrap().is_file());
        assert_eq!(filesystem::identity(&proof).unwrap().links, 1);
        assert!(proof.metadata().unwrap().len() <= 8 * 1024 * 1024);
        drop(proof);
        let catalog = Arc::new(Catalog::open(database).unwrap());
        assert_eq!(
            catalog.summary().unwrap().schema,
            crate::catalog::SCHEMA_VERSION
        );
        let selected = token.clone();
        let expected_source = crate::library::native::encode_path(&root.join("source"));
        let binding=catalog.call(move|db| {
            let text:Option<String>=db.query_row("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=65536 THEN evidence END FROM operation_journal WHERE phase=?1 AND item_id=?2",params![PLAN_PHASE,selected],|row|row.get(0))?;
            let binding:Binding=decode(text.as_deref().unwrap(),MAX_RECORD)?;
            let paths:Vec<Vec<u8>>=db.prepare("SELECT native_path FROM roots")?.query_map([],|row|row.get(0))?.collect::<rusqlite::Result<_>>()?;
            assert_eq!(paths,vec![expected_source]);
            Ok(binding)
        }).unwrap();
        assert_eq!(binding.plan.id, token);
        assert_eq!(binding.plan.files.len(), 2);
        assert_eq!(
            binding
                .plan
                .files
                .iter()
                .map(|file| file.source.as_str())
                .collect::<Vec<_>>(),
            ["Photo.JPG", "Photo.xmp"]
        );
        assert_eq!(
            binding
                .plan
                .files
                .iter()
                .map(|file| file.target.as_str())
                .collect::<Vec<_>>(),
            ["Photo.JPG", "Photo.xmp"]
        );
        assert!(binding.plan.source.matches(source.leaf()).unwrap());
        assert!(binding.plan.destination.matches(output.leaf()).unwrap());
        let journal = CatalogJournal::bind(
            catalog,
            binding.owner_id,
            binding.manifest_id,
            binding.plan.clone(),
        )
        .unwrap();
        assert_eq!(journal.history_count().unwrap(), 0);
        let point = std::env::var("STILLHARBOUR_M2_CATALOG_CHILD_FAULT").unwrap();
        let fault = crash_boundaries()
            .into_iter()
            .find(|fault| format!("{fault:?}") == point)
            .unwrap();
        let validate = || {
            let source = crate::filesystem::DirectoryChain::open(&root.join("source"))?;
            let output = super::super::output_chain(&root.join("output"))?;
            if !binding.plan.source.matches(source.leaf())?
                || !binding.plan.destination.matches(output.leaf())?
            {
                return Err(failed("Owned catalog crash-fixture roots changed."));
            }
            Ok(())
        };
        use crate::jobs::operation_locks::{Access, Registry, Scope};
        let lease = Registry::shared()
            .try_acquire(vec![
                Scope::new(source, Access::Read),
                Scope::new(output, Access::Write),
            ])
            .unwrap();
        let io = crate::jobs::paced_io::PacedIo::shared();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        super::super::CRASH_ON_FAULT.store(true, std::sync::atomic::Ordering::Release);
        let _ = super::super::CopyContext {
            plan: &binding.plan,
            journal: &journal,
            source_root: lease.directory(0).leaf(),
            destination: lease.directory(1).leaf(),
            io: &io,
            cancel: &cancel,
            validate_scope: &validate,
        }
        .execute(Some(fault));
        panic!("Owned catalog process-exit boundary was not reached");
    }
    #[test]
    fn catalog_actor_process_exit_matrix_reopens_real_state_without_replay_or_source_loss() {
        for fault in crash_boundaries() {
            let fixture = Fixture::new().unwrap();
            drop(fixture.journal().unwrap());
            let Fixture {
                library,
                _run,
                source,
                destination,
                owner,
                manifest,
                plan,
                baseline,
            } = fixture;
            let root = _run.path().canonicalize().unwrap();
            std::fs::write(root.join("fixture-owner"), plan.id.as_bytes()).unwrap();
            drop(library);
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "executor::catalog_journal::tests::catalog_crash_fixture_child",
                    "--nocapture",
                ])
                .env("STILLHARBOUR_M2_CATALOG_CHILD_ROOT", &root)
                .env("STILLHARBOUR_M2_CATALOG_CHILD_TOKEN", &plan.id)
                .env("STILLHARBOUR_M2_CATALOG_CHILD_FAULT", format!("{fault:?}"));
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW; owned noninteractive child.
            }
            let mut child = command.spawn().unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("Owned catalog crash child timed out");
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            };
            assert_eq!(
                status.code(),
                Some(77),
                "Catalog child did not reach {fault:?}"
            );
            let catalog = Arc::new(Catalog::open(root.join("data/Fixture.stillharbour")).unwrap());
            let library = crate::library::Library::new(
                catalog,
                crate::library::LibraryOptions {
                    data_directory: root.join("data"),
                    workers: crate::library::WorkerPaths {
                        executable: std::env::current_exe().unwrap(),
                        exiftool: None,
                        working_directory: root.join("data/worker"),
                    },
                },
            )
            .unwrap();
            let fixture = Fixture {
                library,
                _run,
                source,
                destination,
                owner,
                manifest,
                plan,
                baseline,
            };
            let journal = fixture.journal().unwrap();
            let before = journal.history_count().unwrap();
            let states = fixture.inspect(&journal).unwrap();
            assert_eq!(fixture.inspect(&journal).unwrap(), states);
            use super::super::{CopyFault, RecoveredItem};
            let expected = match fault {
                CopyFault::AfterIntent => RecoveredItem::NotStarted,
                CopyFault::AfterStageCreation => RecoveredItem::RetainedPartialStage,
                CopyFault::AfterCopyChunk
                | CopyFault::AfterFlush
                | CopyFault::AfterVerification
                | CopyFault::BeforePublish => RecoveredItem::StagedVerified,
                CopyFault::AfterPublish
                | CopyFault::AfterItemCommit
                | CopyFault::AfterPlanCommit => RecoveredItem::PublishedVerified,
                _ => unreachable!(),
            };
            assert_eq!(
                states[0], expected,
                "Unexpected recovery state at {fault:?}"
            );
            assert_eq!(
                states[1],
                if fault == CopyFault::AfterPlanCommit {
                    RecoveredItem::PublishedVerified
                } else {
                    RecoveredItem::NotStarted
                }
            );
            assert!(fixture.execute(&journal, None).is_err());
            assert_eq!(journal.history_count().unwrap(), before);
            assert!(fixture.preserved().unwrap());
        }
    }
    #[test]
    fn catalog_stream_uses_actual_retained_scope_and_reopens_completed_and_interrupted_evidence() {
        let receipt = fixture_probe().unwrap();
        assert_eq!(receipt["completed"]["publishedVerified"], 2);
        assert_eq!(receipt["interruptedAfterPublish"]["publishedVerified"], 1);
        assert_eq!(receipt["nested"]["publishedVerified"], 2);
    }
    #[test]
    fn nested_parent_substitution_never_adopts_new_source_or_destination_directories() {
        for source_swap in [true, false] {
            let fixture = Fixture::with_nested(true).unwrap();
            let journal = fixture.journal().unwrap();
            if source_swap {
                std::fs::rename(
                    fixture.source.join("source-folder"),
                    fixture.source.join("retained-source-folder"),
                )
                .unwrap();
                std::fs::create_dir(fixture.source.join("source-folder")).unwrap();
                std::fs::write(
                    fixture.source.join("source-folder/Photo.JPG"),
                    generated_photo_bytes().unwrap(),
                )
                .unwrap();
                std::fs::write(
                    fixture.source.join("source-folder/Photo.xmp"),
                    COMPANION_BYTES,
                )
                .unwrap();
            } else {
                std::fs::rename(
                    fixture.destination.join("organized"),
                    fixture.destination.join("retained-output-folder"),
                )
                .unwrap();
                std::fs::create_dir(fixture.destination.join("organized")).unwrap();
                std::fs::write(
                    fixture.destination.join("organized/untouched.bin"),
                    b"new destination occupant",
                )
                .unwrap();
            }
            assert!(fixture.execute(&journal, None).is_err());
            assert_eq!(journal.history_count().unwrap(), 0);
            assert!(fixture.preserved().unwrap());
            if source_swap {
                assert_eq!(
                    std::fs::read(fixture.source.join("retained-source-folder/Photo.JPG")).unwrap(),
                    generated_photo_bytes().unwrap()
                );
                assert_eq!(
                    std::fs::read(fixture.source.join("retained-source-folder/Photo.xmp")).unwrap(),
                    COMPANION_BYTES
                );
            } else {
                assert_eq!(
                    std::fs::read(fixture.destination.join("organized/untouched.bin")).unwrap(),
                    b"new destination occupant"
                );
            }
        }
    }
    #[test]
    fn bounded_unknown_retained_events_fail_before_execution_and_consistent_snapshots_reopen() {
        for payload in [
            serde_json::json!({"padding":"x".repeat(MAX_EVENT)}),
            serde_json::json!({"ordinal":1,"stream_digest":"foreign","item":0,"phase":"item_intent","evidence":{}}),
        ] {
            let fixture = Fixture::new().unwrap();
            let journal = fixture.journal().unwrap();
            drop(journal);
            let owner = fixture.owner.clone();
            let stream = fixture.plan.id.clone();
            fixture.library.catalog.call(move |db| {
                db.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,'owned-fixture')",params![owner,stream,EVENT_PHASE,payload.to_string()])?;Ok(())
            }).unwrap();
            assert!(fixture.journal().is_err());
            assert!(fixture.preserved().unwrap());
        }
        let fixture = Fixture::new().unwrap();
        let journal = fixture.journal().unwrap();
        fixture.execute(&journal, None).unwrap();
        drop(journal);
        let snapshot = fixture._run.path().join("Snapshot.stillharbour");
        fixture.library.catalog.snapshot(snapshot.clone()).unwrap();
        let restored = Arc::new(Catalog::open(snapshot).unwrap());
        let journal = CatalogJournal::bind(
            restored,
            fixture.owner.clone(),
            fixture.manifest.clone(),
            fixture.plan.clone(),
        )
        .unwrap();
        assert_eq!(
            fixture.inspect(&journal).unwrap(),
            [
                super::super::RecoveredItem::PublishedVerified,
                super::super::RecoveredItem::PublishedVerified
            ]
        );
        assert!(fixture.execute(&journal, None).is_err());
        assert!(fixture.preserved().unwrap());
    }
    #[test]
    fn catalog_write_denial_after_publication_never_invents_completion() {
        let fixture = Fixture::new().unwrap();
        let journal = fixture.journal().unwrap();
        fixture.library.catalog.call(|db| {db.execute_batch("CREATE TRIGGER fixture_deny_completion BEFORE INSERT ON operation_journal WHEN NEW.phase='copy_execution_event_v1' AND json_extract(NEW.evidence,'$.phase')='item_completed_verified' BEGIN SELECT RAISE(ABORT,'Owned fixture denied completion'); END;")?;Ok(())}).unwrap();
        assert_eq!(
            fixture.execute(&journal, None).unwrap_err().code,
            ErrorCode::Catalog
        );
        drop(journal);
        let reopened = fixture.journal().unwrap();
        assert_eq!(
            fixture.inspect(&reopened).unwrap(),
            [
                super::super::RecoveredItem::PublishedVerified,
                super::super::RecoveredItem::NotStarted
            ]
        );
        assert!(fixture.preserved().unwrap());
        fixture.library.catalog.call(|db| {let complete:u32=db.query_row("SELECT count(*) FROM operation_journal WHERE phase='copy_execution_event_v1' AND json_extract(evidence,'$.phase') IN ('item_completed_verified','plan_completed_verified')",[],|row|row.get(0))?;assert_eq!(complete,0);Ok(())}).unwrap();
    }
    #[test]
    fn unknown_foreign_out_of_order_and_concurrent_journal_writes_fail_closed() {
        let fixture = Fixture::new().unwrap();
        let first = fixture.journal().unwrap();
        let second = fixture.journal().unwrap();
        assert!(
            first
                .append(Some(0), "item_completed_verified", serde_json::json!({}))
                .is_err()
        );
        assert!(
            first
                .append(
                    None,
                    "plan_completed_verified",
                    serde_json::json!({"files":2})
                )
                .is_err()
        );
        assert!(
            first
                .append(Some(0), "invented", serde_json::json!({}))
                .is_err()
        );
        let file = &fixture.plan.files[0];
        let intent = serde_json::json!({"stage":file.stage,"target":file.target,"sourceRevision":file.revision});
        first
            .append(Some(0), "item_intent", intent.clone())
            .unwrap();
        assert!(second.append(Some(0), "item_intent", intent).is_err());
        let mut wrong = fixture.plan.clone();
        wrong.files[0].target = "different.JPG".into();
        wrong.digest = wrong.digest().unwrap();
        assert!(
            CatalogJournal::bind(
                fixture.library.catalog.clone(),
                fixture.owner.clone(),
                fixture.manifest.clone(),
                wrong
            )
            .is_err()
        );
        assert!(
            CatalogJournal::bind(
                fixture.library.catalog.clone(),
                fixture.owner.clone(),
                uuid::Uuid::new_v4().to_string(),
                fixture.plan.clone()
            )
            .is_err()
        );
        assert!(fixture.preserved().unwrap());
        assert_eq!(first.history_count().unwrap(), 1);
        assert_eq!(second.history_count().unwrap(), 0);
        fixture
            .library
            .catalog
            .call(|db| {
                db.execute("UPDATE roots SET mode='referenced'", [])?;
                Ok(())
            })
            .unwrap();
        assert!(fixture.journal().is_err());
        let duplicate = Fixture::new().unwrap();
        drop(duplicate.journal().unwrap());
        duplicate.library.catalog.call(|db| {
            db.execute_batch("DROP INDEX operation_copy_stream;INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) SELECT plan_id,item_id,phase,evidence,created_at FROM operation_journal WHERE phase='copy_execution_plan_v1';")?;Ok(())
        }).unwrap();
        let error = duplicate
            .journal()
            .err()
            .expect("Duplicate stream anchors were adopted");
        assert!(
            error
                .message
                .contains("Multiple immutable copy-stream anchors")
        );
        assert!(duplicate.preserved().unwrap());
    }
}
