//! Native appdata lifetime ownership and OS-sealed restart activation.
use super::*;
use crate::{
    backup::credentials::catalog_restore as authority, executor::snapshot_bridge as bridge,
    filesystem::DirectoryChain,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    sync::{Mutex, atomic::AtomicBool},
};

const DEFAULT_CATALOG: &str = "Foundation.stillharbour";
const MAX_DESCRIPTOR: u64 = 8 * 1024 * 1024;
mod retained;
#[cfg(test)]
mod tests;
pub use retained::RetainedCatalogSource;
#[cfg(feature = "fixture-probes")]
mod fixture_session;
#[cfg(feature = "fixture-probes")]
pub use fixture_session::{CatalogFixtureRole, CatalogFixtureSession};

fn blocked() -> AppError {
    AppError::new(
        ErrorCode::Conflict,
        "Catalog activation could not prove the exact selected native catalog and OS generation. Preserve the original, pre-restore copy and candidate; no other or empty catalog was selected.",
    )
}
fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| blocked())
}

fn same(a: &FileIdentity, b: &FileIdentity) -> bool {
    a.volume == b.volume && a.file == b.file
}
fn hash(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}
fn present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn root(path: &Path) -> Result<DirectoryChain> {
    #[cfg(windows)]
    {
        DirectoryChain::open_for_owned_publication(path)
    }
    #[cfg(not(windows))]
    {
        DirectoryChain::open(path)
    }
}
fn create_new(path: &Path, bytes: &[u8]) -> Result<File> {
    let parent = DirectoryChain::open(path.parent().ok_or_else(blocked)?)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options
            .share_mode(
                windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                    | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE,
            )
            .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    #[cfg(unix)]
    parent.leaf().sync_all()?;
    let current = DirectoryChain::open(path.parent().ok_or_else(blocked)?)?;
    if !same(parent.identity(), current.identity()) || filesystem::identity(&file)?.links != 1 {
        return Err(blocked());
    }
    Ok(file)
}
fn read_bytes(path: &Path, limit: u64) -> Result<(Vec<u8>, File, DirectoryChain)> {
    let chain = DirectoryChain::open(path.parent().ok_or_else(blocked)?)?;
    let mut file = filesystem::open_readonly(path)?;
    let id = filesystem::identity(&file)?;
    let revision = native::revision(&file)?;
    if !file.metadata()?.is_file() || id.links != 1 || file.metadata()?.len() > limit {
        return Err(blocked());
    }
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit
        || filesystem::identity(&file)? != id
        || native::revision(&file)? != revision
    {
        return Err(blocked());
    }
    Ok((bytes, file, chain))
}
fn current_seal(value: &CurrentCatalogBinding) -> Result<String> {
    Ok(hash(&encode(value)?))
}
fn compatible(path: &Path) -> Result<File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        use windows_sys::Win32::Storage::FileSystem::*;
        let _parent = DirectoryChain::open(path.parent().ok_or_else(blocked)?)?;
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL)
            .open(path)?;
        if !file.metadata()?.is_file()
            || file.metadata()?.file_attributes()
                & (FILE_ATTRIBUTE_REPARSE_POINT
                    | FILE_ATTRIBUTE_OFFLINE
                    | FILE_ATTRIBUTE_RECALL_ON_OPEN
                    | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                != 0
        {
            return Err(blocked());
        }
        Ok(file)
    }
    #[cfg(not(windows))]
    {
        filesystem::open_readonly(path)
    }
}
fn binding(path: &Path) -> Result<CurrentCatalogBinding> {
    let value = super::capture_current(path)?;
    if value.identity.links != 1 {
        return Err(blocked());
    }
    Ok(value)
}

/// Root owns this through final process shutdown, including all workers.
pub struct ActivationStore {
    path: PathBuf,
    chain: DirectoryChain,
    lock: File,
    lock_identity: FileIdentity,
    scope: String,
    restores: bridge::Root,
    transaction: Mutex<()>,
}
/// Native-only handoff; no renderer can manufacture or deserialize this type.
pub struct StartupSelection {
    pub path: PathBuf,
    pub identity: FileIdentity,
    pub volume: String,
    pub creation_allowed: bool,
    pub prepared: Option<PreparedCatalogRestore>,
    pub activated_now: bool,
    file: File,
    chain: DirectoryChain,
    scope: String,
    generation: u64,
}
impl StartupSelection {
    pub fn revalidate(&self) -> Result<()> {
        let chain = DirectoryChain::open(self.path.parent().ok_or_else(blocked)?)?;
        let file = compatible(&self.path)?;
        if chain.identities().len() != self.chain.identities().len()
            || chain
                .identities()
                .iter()
                .zip(self.chain.identities())
                .any(|(a, b)| !same(a, b))
            || filesystem::identity(&file)? != self.identity
            || filesystem::identity(&self.file)? != self.identity
            || native::volume_token(&file)? != self.volume
        {
            return Err(blocked());
        }
        Ok(())
    }
}
impl ActivationStore {
    #[cfg(test)]
    pub(crate) fn fixture_authority_scope(&self) -> &str {
        &self.scope
    }
    pub fn open(appdata: &Path) -> Result<Self> {
        let chain = root(appdata)?;
        let scope = hash(&encode(&(
            native::encode_path(appdata),
            chain
                .identities()
                .iter()
                .map(|i| (i.volume, i.file))
                .collect::<Vec<_>>(),
        ))?);
        let lock_path = appdata.join(".catalog-activation.lock");
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options
                .share_mode(
                    windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ
                        | windows_sys::Win32::Storage::FileSystem::FILE_SHARE_WRITE,
                )
                .custom_flags(
                    windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT,
                );
        }
        let lock = options.open(&lock_path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            use windows_sys::Win32::Storage::FileSystem::*;
            if lock.metadata()?.file_attributes()
                & (FILE_ATTRIBUTE_REPARSE_POINT
                    | FILE_ATTRIBUTE_OFFLINE
                    | FILE_ATTRIBUTE_RECALL_ON_OPEN
                    | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                != 0
            {
                return Err(blocked());
            }
        }
        if !lock.metadata()?.is_file() || filesystem::identity(&lock)?.links != 1 {
            return Err(blocked());
        }
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|e| {
            AppError::new(
                ErrorCode::CatalogInUse,
                "This application data directory is already active.",
            )
            .caused_by(e)
        })?;
        let parent = bridge::Root::existing(
            appdata.to_owned(),
            chain.identity().clone(),
            native::volume_token(chain.leaf())?,
        );
        let restores_path = appdata.join("catalog-restores");
        let restores = if present(&restores_path)? {
            let held = root(&restores_path)?;
            bridge::Root::existing(
                restores_path,
                held.identity().clone(),
                native::volume_token(held.leaf())?,
            )
        } else {
            let created = bridge::make_directory(&parent, "catalog-restores")?;
            let marker = encode(&(1u32, &scope, created.identity.volume, created.identity.file))?;
            bridge::write_new(&created, "owner.json", &marker)?;
            created
        };
        let expected = encode(&(
            1u32,
            &scope,
            restores.identity.volume,
            restores.identity.file,
        ))?;
        if read_bytes(&restores.path.join("owner.json"), 2048)?.0 != expected {
            return Err(blocked());
        }
        let lock_identity = filesystem::identity(&lock)?;
        let store = Self {
            path: appdata.to_owned(),
            chain,
            lock,
            lock_identity,
            scope,
            restores,
            transaction: Mutex::new(()),
        };
        store.revalidate()?;
        Ok(store)
    }
    fn revalidate(&self) -> Result<()> {
        let chain = root(&self.path)?;
        if chain.identities().len() != self.chain.identities().len()
            || chain
                .identities()
                .iter()
                .zip(self.chain.identities())
                .any(|(a, b)| !same(a, b))
            || filesystem::identity(&self.lock)? != self.lock_identity
            || filesystem::identity(&compatible(&self.path.join(".catalog-activation.lock"))?)?
                != self.lock_identity
        {
            return Err(blocked());
        }
        let restores = root(&self.restores.path)?;
        if !same(restores.identity(), &self.restores.identity) {
            return Err(blocked());
        }
        Ok(())
    }
    fn history(&self) -> Result<bool> {
        self.revalidate()?;
        let mut count = 0;
        for entry in fs::read_dir(&self.restores.path)? {
            let entry = entry?;
            count += 1;
            if count > 1024 {
                return Err(blocked());
            }
            if entry.file_name() != std::ffi::OsStr::new("owner.json") {
                return Ok(true);
            }
        }
        self.revalidate()?;
        Ok(false)
    }
    fn descriptor(&self, id: &str) -> Result<(PreparedCatalogRestore, String)> {
        if uuid::Uuid::parse_str(id)
            .map_err(|_| blocked())?
            .to_string()
            != id
        {
            return Err(blocked());
        }
        self.revalidate()?;
        let directory = self.restores.path.join(id);
        let (bytes, file, chain) = read_bytes(&directory.join("prepared.json"), MAX_DESCRIPTOR)?;
        let prepared: PreparedCatalogRestore =
            serde_json::from_slice(&bytes).map_err(|_| blocked())?;
        prepared.validate_provenance()?;
        if prepared.transaction_id != id
            || prepared.schema != crate::catalog::SCHEMA_VERSION
            || prepared.roots.is_empty()
            || prepared.roots.len() > 32
            || prepared.candidate.native_path()? != directory.join("candidate.stillharbour")
            || prepared.pre_restore.native_path()? != directory.join("pre-restore.stillharbour")
        {
            return Err(blocked());
        }
        let seal = hash(&encode(&(
            hash(&bytes),
            filesystem::identity(&file)?,
            native::volume_token(&file)?,
            chain
                .identities()
                .iter()
                .map(|i| (i.volume, i.file))
                .collect::<Vec<_>>(),
        ))?);
        Ok((prepared, seal))
    }
    fn witness(
        &self,
        current: &CurrentCatalogBinding,
        authority: &authority::Authority,
        prepared: Option<PreparedCatalogRestore>,
        activated_now: bool,
    ) -> Result<StartupSelection> {
        let path = native::decode_path(&current.path)?;
        let chain = DirectoryChain::open(path.parent().ok_or_else(blocked)?)?;
        let file = compatible(&path)?;
        if current.identity.links != 1
            || filesystem::identity(&file)? != current.identity
            || native::volume_token(&file)? != current.volume
            || (!authority.initializing && file.metadata()?.len() == 0)
        {
            return Err(blocked());
        }
        let selected = StartupSelection {
            path,
            identity: current.identity.clone(),
            volume: current.volume.clone(),
            creation_allowed: authority.initializing,
            prepared,
            activated_now,
            file,
            chain,
            scope: self.scope.clone(),
            generation: authority.generation,
        };
        selected.revalidate()?;
        Ok(selected)
    }
    fn active(
        &self,
        value: &authority::Authority,
        activated_now: bool,
    ) -> Result<StartupSelection> {
        if let Some(id) = &value.active.transaction_id {
            let (prepared, seal) = self.descriptor(id)?;
            if seal != value.active.digest {
                return Err(blocked());
            }
            let current = CurrentCatalogBinding {
                path: prepared.candidate.path.clone(),
                identity: prepared.candidate.identity.clone(),
                volume: prepared.candidate.volume.clone(),
            };
            self.witness(&current, value, Some(prepared), activated_now)
        } else {
            let current = binding(&self.path.join(DEFAULT_CATALOG))?;
            if current_seal(&current)? != value.active.digest {
                return Err(blocked());
            }
            self.witness(&current, value, None, false)
        }
    }
    fn frozen(&self, file: &CatalogFileBinding) -> Result<()> {
        let path = file.native_path()?;
        let mut held = filesystem::open_readonly(&path)?;
        let revision = native::revision(&held)?;
        if filesystem::identity(&held)? != file.identity
            || native::volume_token(&held)? != file.volume
            || held.metadata()?.len() != file.bytes
            || file.bytes == 0
        {
            return Err(blocked());
        }
        held.seek(SeekFrom::Start(0))?;
        if bridge::digest(&mut held, &AtomicBool::new(false))? != file.digest
            || native::revision(&held)? != revision
        {
            return Err(blocked());
        }
        Ok(())
    }
    fn validate_pending(
        &self,
        prepared: &PreparedCatalogRestore,
        value: &authority::Authority,
    ) -> Result<()> {
        let active = self.active(value, false)?;
        self.verify_retained_provenance_locked(prepared, value)?;
        if native::decode_path(&prepared.current.path)? != active.path
            || prepared.current.identity != active.identity
            || prepared.current.volume != active.volume
        {
            return Err(blocked());
        }
        let expected = crate::catalog::current_schema_fingerprint()?;
        super::validate_current_revision(
            &prepared.current,
            &prepared.catalog_id,
            &expected,
            &prepared.current_catalog_revision,
        )?;
        self.frozen(&prepared.candidate)?;
        self.frozen(&prepared.pre_restore)?;
        if prepared.candidate.identity == prepared.pre_restore.identity
            || prepared.candidate.identity == prepared.current.identity
            || prepared.pre_restore.identity == prepared.current.identity
        {
            return Err(blocked());
        }
        let candidate = super::validate_catalog(
            &prepared.candidate.native_path()?,
            &prepared.catalog_id,
            &expected,
        )?;
        if candidate.revision != prepared.candidate_catalog_revision
            || candidate.roots.len() != prepared.roots.len()
            || prepared
                .roots
                .iter()
                .map(|r| &r.root_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != prepared.roots.len()
        {
            return Err(blocked());
        }
        for requested in &prepared.roots {
            let actual = candidate
                .roots
                .iter()
                .find(|r| r.root_id == requested.root_id)
                .ok_or_else(blocked)?;
            if actual.path != requested.path
                || !same(&actual.identity, &requested.identity)
                || actual.volume != requested.volume
            {
                return Err(blocked());
            }
            let chain = DirectoryChain::open(&native::decode_path(&requested.path)?)?;
            if !same(chain.identity(), &requested.identity)
                || native::volume_token(chain.leaf())? != requested.volume
            {
                return Err(blocked());
            }
        }
        self.frozen(&prepared.candidate)?;
        self.frozen(&prepared.pre_restore)?;
        super::validate_current_revision(
            &prepared.current,
            &prepared.catalog_id,
            &expected,
            &prepared.current_catalog_revision,
        )?;
        active.revalidate()?;
        self.revalidate()
    }
    pub fn startup_selection(&self) -> Result<StartupSelection> {
        let _guard = self.transaction.lock().map_err(|_| blocked())?;
        self.revalidate()?;
        let mut value = match authority::read(&self.scope)? {
            Some(value) => value,
            None => {
                if self.history()? {
                    return Err(blocked());
                }
                let path = self.path.join(DEFAULT_CATALOG);
                let initializing = !present(&path)?;
                if initializing {
                    create_new(
                        &self.restores.path.join("initial-creation.json"),
                        self.scope.as_bytes(),
                    )?;
                    drop(create_new(&path, b"")?);
                } else if filesystem::open_readonly(&path)?.metadata()?.len() == 0 {
                    return Err(blocked());
                }
                let current = binding(&path)?;
                authority::initialize(
                    &self.scope,
                    authority::Selector {
                        transaction_id: None,
                        digest: current_seal(&current)?,
                    },
                    initializing,
                )?
            }
        };
        let mut activated = false;
        if let Some(pending) = &value.pending {
            let (prepared, seal) = self.descriptor(
                pending
                    .selector
                    .transaction_id
                    .as_deref()
                    .ok_or_else(blocked)?,
            )?;
            if seal != pending.selector.digest {
                return Err(blocked());
            }
            self.validate_pending(&prepared, &value)?;
            value = authority::promote(&self.scope, &value)?;
            activated = true;
        }
        self.active(&value, activated)
    }
    pub fn confirm_initialized(&self, selection: &StartupSelection) -> Result<()> {
        let _guard = self.transaction.lock().map_err(|_| blocked())?;
        self.revalidate()?;
        selection.revalidate()?;
        let value = authority::read(&self.scope)?.ok_or_else(blocked)?;
        if selection.scope != self.scope || selection.generation != value.generation {
            return Err(blocked());
        }
        if value.initializing {
            if selection.file.metadata()?.len() == 0 {
                return Err(blocked());
            }
            authority::confirm_initialized(&self.scope, &value)?;
        }
        Ok(())
    }
    pub fn stage_pending(&self, prepared: &PreparedCatalogRestore) -> Result<()> {
        let _guard = self.transaction.lock().map_err(|_| blocked())?;
        self.revalidate()?;
        let value = authority::read(&self.scope)?.ok_or_else(blocked)?;
        if value.initializing || value.pending.is_some() {
            return Err(blocked());
        }
        let (actual, seal) = self.descriptor(&prepared.transaction_id)?;
        if encode(&actual)? != encode(prepared)? {
            return Err(blocked());
        }
        self.validate_pending(&actual, &value)?;
        authority::stage(
            &self.scope,
            &value,
            authority::Selector {
                transaction_id: Some(prepared.transaction_id.clone()),
                digest: seal,
            },
        )?;
        Ok(())
    }
}
