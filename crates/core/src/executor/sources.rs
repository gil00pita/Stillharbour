//! Native source planning is distinct from an executable copy plan. Future
//! target folders need not exist and no output object is created here.
use super::{Object, failed, locations};
use crate::library::model::RootMode;
use crate::{
    catalog::Catalog,
    domain::Result,
    filesystem::DirectoryChain,
    jobs::operation_locks::{Access, Registry, Scope},
    library::{copy_manifest, native, operation_store},
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportSelection {
    pub id: String,
    pub digest: String,
    pub source_mode: RootMode,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct File {
    pub id: String,
    pub display_name: String,
    pub location: locations::Source,
    pub object: Object,
    pub revision: String,
    pub bytes: u64,
    pub target: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub policy: String,
    pub catalog_id: String,
    pub owner_id: String,
    pub owner_digest: String,
    pub manifest_id: String,
    pub manifest_digest: String,
    pub root_id: String,
    pub source: Object,
    pub source_volume: String,
    pub destination: Object,
    pub destination_volume: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_selection: Option<ImportSelection>,
    pub files: Vec<File>,
}
pub(super) fn root(db: &Connection, id: &str, managed: bool) -> Result<(Vec<u8>, Object, String)> {
    let (path,device,inode,volume,mode):(Option<Vec<u8>>,String,String,String,String)=db.query_row(
        "SELECT CASE WHEN length(native_path)<=65536 THEN native_path END,device,inode,volume,mode FROM roots WHERE id=?",
        [id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)))?;
    if !matches!(mode.as_str(), "managed" | "referenced") || (managed && mode != "managed") {
        return Err(failed("The source root needs its unchanged managed grant."));
    }
    Ok((
        path.ok_or_else(|| failed("Source root path exceeds its bound."))?,
        Object {
            volume: device
                .parse()
                .map_err(|_| failed("Invalid source volume identity."))?,
            file: inode
                .parse()
                .map_err(|_| failed("Invalid source file identity."))?,
        },
        volume,
    ))
}
impl Snapshot {
    pub(super) fn required_mode(&self) -> RootMode {
        self.import_selection
            .as_ref()
            .map_or(RootMode::Managed, |selection| selection.source_mode)
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn validate_roots(&self, catalog: &Arc<Catalog>) -> Result<()> {
        self.validate_roots_for(catalog, true)
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn validate_read_roots(&self, catalog: &Arc<Catalog>) -> Result<()> {
        self.validate_roots_for(catalog, false)
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    fn validate_roots_for(&self, catalog: &Arc<Catalog>, managed: bool) -> Result<()> {
        let id = self.root_id.clone();
        let importing = self.import_selection.is_some();
        let required_mode = self.required_mode();
        let owner = self.owner_id.clone();
        let (path, object, volume) = catalog.call(move |db| {
            let draft = operation_store::load(db, &owner)?;
            operation_store::validate_destination_root(db, &draft)?;
            if managed && importing {
                let mode: String =
                    db.query_row("SELECT mode FROM roots WHERE id=?", [&id], |row| row.get(0))?;
                let expected = match required_mode {
                    RootMode::Managed => "managed",
                    RootMode::Referenced => "referenced",
                };
                if mode != expected {
                    return Err(failed("The import source grant changed."));
                }
            }
            root(db, &id, managed && !importing)
        })?;
        if object != self.source || volume != self.source_volume {
            return Err(failed("Source root identity/volume grant changed."));
        }
        let file = native::open_root(&native::decode_path(&path)?, None)?;
        if !self.source.matches(&file)? || native::volume_token(&file)? != self.source_volume {
            return Err(failed(
                "The reviewed source root is unavailable or changed identity.",
            ));
        }
        Ok(())
    }
    pub(super) fn build(catalog: &Arc<Catalog>, owner: String, manifest: String) -> Result<Self> {
        Self::build_for(catalog, owner, manifest, true)
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn build_read(
        catalog: &Arc<Catalog>,
        owner: String,
        manifest: String,
    ) -> Result<Self> {
        Self::build_for(catalog, owner, manifest, false)
    }
    fn build_for(
        catalog: &Arc<Catalog>,
        owner: String,
        manifest: String,
        managed: bool,
    ) -> Result<Self> {
        crate::operations::validate_id(&owner)?;
        crate::operations::validate_id(&manifest)?;
        let (draft, manifest, root_id, path, source, source_volume) = catalog.call(move |db| {
            let draft = operation_store::load(db, &owner)?;
            operation_store::validate_destination_root(db, &draft)?;
            let manifest = copy_manifest::load(db, &owner, &manifest)?;
            let root_id = manifest
                .files
                .first()
                .ok_or_else(|| failed("No source files were selected."))?
                .view
                .root_id
                .clone();
            if !manifest.view.issues.is_empty()
                || manifest
                    .files
                    .iter()
                    .any(|file| file.view.root_id != root_id)
            {
                return Err(failed(
                    "Source planning currently requires one conflict-free source root.",
                ));
            }
            let (path, source, volume) = root(db, &root_id, managed)?;
            Ok((draft, manifest, root_id, path, source, volume))
        })?;
        let source_path = native::decode_path(&path)?;
        let destination_path = native::decode_path(&draft.destination_native)?;
        let destination = Object {
            volume: draft.destination_device,
            file: draft.destination_file,
        };
        // Bind selected spellings first, then independently open canonical
        // native components, including their ancestor identities.
        let selected_source = native::open_root(&source_path, None)?;
        let selected_destination = native::open_root(&destination_path, None)?;
        if !source.matches(&selected_source)?
            || !destination.matches(&selected_destination)?
            || native::volume_token(&selected_source)? != source_volume
            || native::volume_token(&selected_destination)? != draft.destination_volume
        {
            return Err(failed(
                "Source/destination identity changed before source planning.",
            ));
        }
        drop((selected_source, selected_destination));
        let source_chain = DirectoryChain::open(&source_path.canonicalize()?)?;
        let destination_chain = DirectoryChain::open(&destination_path.canonicalize()?)?;
        if !source.matches(source_chain.leaf())?
            || !destination.matches(destination_chain.leaf())?
            || native::volume_token(source_chain.leaf())? != source_volume
            || native::volume_token(destination_chain.leaf())? != draft.destination_volume
            || source_chain
                .identities()
                .iter()
                .any(|id| id.volume == destination.volume && id.file == destination.file)
            || destination_chain
                .identities()
                .iter()
                .any(|id| id.volume == source.volume && id.file == source.file)
        {
            return Err(failed(
                "Source planning requires separate unchanged native roots.",
            ));
        }
        let lease = Registry::shared().try_acquire(vec![
            Scope::new(source_chain, Access::Read),
            Scope::new(destination_chain, Access::Read),
        ])?;
        let mut files = Vec::new();
        for member in &manifest.files {
            let location = locations::Source::capture(
                lease.directory(0).leaf(),
                &native::decode_path(&member.relative_native)?,
            )?;
            let file = location.open(lease.directory(0).leaf())?;
            let object = Object::of(&file)?;
            let revision = native::revision(&file)?;
            let bytes = file.metadata()?.len();
            if (object.volume, object.file) != member.identity
                || revision != member.view.source_revision
                || bytes.to_string() != member.view.bytes
            {
                return Err(failed(
                    "A confirmed source changed during native source planning.",
                ));
            }
            files.push(File {
                id: member.view.id.clone(),
                display_name: member.view.source_name.clone(),
                location,
                object,
                revision,
                bytes,
                target: member.view.target_relative_path.clone(),
            });
        }
        let snapshot = Self {
            policy: "native-copy-source-snapshot-v1".into(),
            catalog_id: draft.catalog_id,
            owner_id: draft.view.id,
            owner_digest: draft.view.digest,
            manifest_id: manifest.view.id,
            manifest_digest: manifest.view.digest,
            root_id,
            source,
            source_volume,
            destination,
            destination_volume: draft.destination_volume,
            import_selection: None,
            files,
        };
        let retained = snapshot.clone();
        catalog.call(move |db| retained.validate_for(db, managed))?;
        // Recheck every opened member through its native parent proof. This is
        // not an atomic directory snapshot or permission to execute later.
        for member in &snapshot.files {
            let file = member.location.open(lease.directory(0).leaf())?;
            if !member.object.matches(&file)?
                || native::revision(&file)? != member.revision
                || file.metadata()?.len() != member.bytes
            {
                return Err(failed(
                    "Source membership changed before source planning completed.",
                ));
            }
        }
        Ok(snapshot)
    }
    pub(super) fn open_scope(
        &self,
        catalog: &Arc<Catalog>,
        destination_access: Access,
    ) -> Result<crate::jobs::operation_locks::Lease> {
        self.open_scope_with(catalog, destination_access, Vec::new())
    }
    pub(super) fn open_scope_with(
        &self,
        catalog: &Arc<Catalog>,
        destination_access: Access,
        additional: Vec<Scope>,
    ) -> Result<crate::jobs::operation_locks::Lease> {
        let snapshot = self.clone();
        let (source_path, destination_path) = catalog.call(move |db| {
            snapshot.validate_for(db, destination_access == Access::Write)?;
            let (source, _, _) = root(
                db,
                &snapshot.root_id,
                destination_access == Access::Write && snapshot.import_selection.is_none(),
            )?;
            Ok((
                source,
                operation_store::load(db, &snapshot.owner_id)?.destination_native,
            ))
        })?;
        let source_path = native::decode_path(&source_path)?;
        let destination_path = native::decode_path(&destination_path)?;
        let source = native::open_root(&source_path, None)?;
        let destination = native::open_root(&destination_path, None)?;
        if !self.source.matches(&source)?
            || !self.destination.matches(&destination)?
            || native::volume_token(&source)? != self.source_volume
            || native::volume_token(&destination)? != self.destination_volume
        {
            return Err(failed(
                "The reviewed source/destination identity changed before job admission.",
            ));
        }
        drop((source, destination));
        let source = DirectoryChain::open(&source_path.canonicalize()?)?;
        let destination = if destination_access == Access::Write {
            super::output_chain(&destination_path.canonicalize()?)?
        } else {
            DirectoryChain::open(&destination_path.canonicalize()?)?
        };
        if !self.source.matches(source.leaf())?
            || !self.destination.matches(destination.leaf())?
            || native::volume_token(source.leaf())? != self.source_volume
            || native::volume_token(destination.leaf())? != self.destination_volume
            || source
                .identities()
                .iter()
                .any(|id| id.volume == self.destination.volume && id.file == self.destination.file)
            || destination
                .identities()
                .iter()
                .any(|id| id.volume == self.source.volume && id.file == self.source.file)
        {
            return Err(failed(
                "The reviewed roots overlap or changed native scope.",
            ));
        }
        let mut scopes = vec![
            Scope::new(source, Access::Read),
            Scope::new(destination, destination_access),
        ];
        scopes.extend(additional);
        Registry::shared().try_acquire(scopes)
    }
    pub(super) fn validate(&self, db: &Connection) -> Result<()> {
        self.validate_for(db, true)
    }
    pub(super) fn validate_for(&self, db: &Connection, managed: bool) -> Result<()> {
        operation_store::validate_destination_root(
            db,
            &operation_store::load(db, &self.owner_id)?,
        )?;
        self.validate_mapping(db)?;
        let (_, source, volume) = root(
            db,
            &self.root_id,
            managed && self.import_selection.is_none(),
        )?;
        if managed && self.import_selection.is_some() {
            let mode: String = db.query_row(
                "SELECT mode FROM roots WHERE id=?",
                [&self.root_id],
                |row| row.get(0),
            )?;
            let expected = match self.required_mode() {
                RootMode::Managed => "managed",
                RootMode::Referenced => "referenced",
            };
            if mode != expected {
                return Err(failed(
                    "The import source grant changed. Retain current outputs and review again.",
                ));
            }
        }
        if self.source != source || self.source_volume != volume {
            return Err(failed(
                "Source snapshot no longer matches its current root grant.",
            ));
        }
        Ok(())
    }
    pub(super) fn validate_mapping(&self, db: &Connection) -> Result<()> {
        if self.import_selection.is_some() {
            #[cfg(any(test, feature = "copy-workflow"))]
            {
                return super::workflow::import_planning::validate_projection(db, self);
            }
            #[cfg(not(any(test, feature = "copy-workflow")))]
            {
                return Err(failed(
                    "Import copy projections are unavailable in this build.",
                ));
            }
        }
        let draft = operation_store::load(db, &self.owner_id)?;
        let manifest = copy_manifest::load(db, &self.owner_id, &self.manifest_id)?;
        if self.policy != "native-copy-source-snapshot-v1"
            || self.catalog_id != draft.catalog_id
            || self.owner_digest != draft.view.digest
            || self.manifest_digest != manifest.view.digest
            || self.destination
                != (Object {
                    volume: draft.destination_device,
                    file: draft.destination_file,
                })
            || self.destination_volume != draft.destination_volume
            || self.files.is_empty()
            || self.files.len() != manifest.files.len()
            || !manifest.view.issues.is_empty()
        {
            return Err(failed(
                "Source snapshot no longer matches its retained native scope.",
            ));
        }
        for (file, member) in self.files.iter().zip(&manifest.files) {
            file.location.validate()?;
            if member.view.root_id != self.root_id
                || file.id != member.view.id
                || file.display_name != member.view.source_name
                || file.location.relative_native != member.relative_native
                || (file.object.volume, file.object.file) != member.identity
                || file.revision != member.view.source_revision
                || file.bytes.to_string() != member.view.bytes
                || file.target != member.view.target_relative_path
            {
                return Err(failed(
                    "Source snapshot differs from the exact confirmed file mapping.",
                ));
            }
        }
        Ok(())
    }
}
