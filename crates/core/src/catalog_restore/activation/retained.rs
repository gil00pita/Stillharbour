//! The sole retained source is the current OS selector's sealed pre-restore file.
//! This proof cannot be deserialized, selected by path, or manufactured by IPC.
use super::*;
use std::sync::Arc;

pub struct RetainedCatalogSource {
    store: Arc<ActivationStore>,
    selector: authority::Selector,
    generation: u64,
    source: CatalogFileBinding,
    current: CurrentCatalogBinding,
    catalog_id: String,
    revision: String,
    roots: Vec<CatalogRootMapping>,
    file: File,
    chain: DirectoryChain,
}
struct Witness {
    selector: authority::Selector,
    generation: u64,
    source: CatalogFileBinding,
    current: CurrentCatalogBinding,
    catalog_id: String,
    revision: String,
    roots: Vec<CatalogRootMapping>,
    file: File,
    chain: DirectoryChain,
}
impl RetainedCatalogSource {
    pub fn transaction_id(&self) -> &str {
        self.selector
            .transaction_id
            .as_deref()
            .expect("proof requires active transaction")
    }
    pub fn generation(&self) -> String {
        self.generation.to_string()
    }
    pub fn source(&self) -> &CatalogFileBinding {
        &self.source
    }
    pub fn current(&self) -> &CurrentCatalogBinding {
        &self.current
    }
    pub fn catalog_id(&self) -> &str {
        &self.catalog_id
    }
    pub fn revision(&self) -> &str {
        &self.revision
    }
    pub fn roots(&self) -> &[CatalogRootMapping] {
        &self.roots
    }
    pub fn revalidate(&self) -> Result<()> {
        let _guard = self.store.transaction.lock().map_err(|_| blocked())?;
        let value = authority::read(&self.store.scope)?.ok_or_else(blocked)?;
        if value.generation != self.generation || value.active != self.selector {
            return Err(blocked());
        }
        let witness = self
            .store
            .retained_locked(self.transaction_id(), &value, false)?;
        if encode(&witness.source)? != encode(&self.source)?
            || encode(&witness.current)? != encode(&self.current)?
            || witness.catalog_id != self.catalog_id
            || witness.revision != self.revision
            || encode(&witness.roots)? != encode(&self.roots)?
            || filesystem::identity(&self.file)? != self.source.identity
            || witness.chain.identities().len() != self.chain.identities().len()
            || witness
                .chain
                .identities()
                .iter()
                .zip(self.chain.identities())
                .any(|(a, b)| !same(a, b))
        {
            return Err(blocked());
        }
        Ok(())
    }
}
impl ActivationStore {
    /// A UI hint only; the source factory independently earns its full proof.
    pub fn latest_retained_transaction_id(&self) -> Result<Option<String>> {
        let _guard = self.transaction.lock().map_err(|_| blocked())?;
        self.revalidate()?;
        let value = authority::read(&self.scope)?.ok_or_else(blocked)?;
        if value.initializing || value.pending.is_some() {
            return Ok(None);
        }
        Ok(value.active.transaction_id)
    }
    pub fn latest_retained_source(
        self: &Arc<Self>,
        transaction_id: &str,
    ) -> Result<Arc<RetainedCatalogSource>> {
        let _guard = self.transaction.lock().map_err(|_| blocked())?;
        let value = authority::read(&self.scope)?.ok_or_else(blocked)?;
        let witness = self.retained_locked(transaction_id, &value, false)?;
        Ok(Arc::new(RetainedCatalogSource {
            store: self.clone(),
            selector: witness.selector,
            generation: witness.generation,
            source: witness.source,
            current: witness.current,
            catalog_id: witness.catalog_id,
            revision: witness.revision,
            roots: witness.roots,
            file: witness.file,
            chain: witness.chain,
        }))
    }
    // Caller already holds transaction. Never call public proof.revalidate while
    // stage_pending owns this mutex: native source checks share this primitive.
    fn retained_locked(
        &self,
        transaction_id: &str,
        value: &authority::Authority,
        allow_pending: bool,
    ) -> Result<Witness> {
        self.revalidate()?;
        if value.initializing
            || (!allow_pending && value.pending.is_some())
            || value.active.transaction_id.as_deref() != Some(transaction_id)
        {
            return Err(blocked());
        }
        let (prepared, seal) = self.descriptor(transaction_id)?;
        if seal != value.active.digest {
            return Err(blocked());
        }
        let selected = self.active(value, false)?;
        let current = CurrentCatalogBinding {
            path: native::encode_path(&selected.path),
            identity: selected.identity.clone(),
            volume: selected.volume.clone(),
        };
        self.frozen(&prepared.pre_restore)?;
        let path = prepared.pre_restore.native_path()?;
        let chain = DirectoryChain::open(path.parent().ok_or_else(blocked)?)?;
        let mut file = filesystem::open_readonly(&path)?;
        let revision = native::revision(&file)?;
        if filesystem::identity(&file)? != prepared.pre_restore.identity
            || prepared.pre_restore.identity.links != 1
            || native::volume_token(&file)? != prepared.pre_restore.volume
            || file.metadata()?.len() != prepared.pre_restore.bytes
        {
            return Err(blocked());
        }
        file.seek(SeekFrom::Start(0))?;
        if bridge::digest(&mut file, &AtomicBool::new(false))? != prepared.pre_restore.digest {
            return Err(blocked());
        }
        let expected = crate::catalog::current_schema_fingerprint()?;
        let catalog = super::super::validate_catalog(&path, &prepared.catalog_id, &expected)?;
        if catalog.revision != prepared.current_catalog_revision
            || native::revision(&file)? != revision
        {
            return Err(blocked());
        }
        self.frozen(&prepared.pre_restore)?;
        selected.revalidate()?;
        self.revalidate()?;
        Ok(Witness {
            selector: value.active.clone(),
            generation: value.generation,
            source: prepared.pre_restore,
            current,
            catalog_id: prepared.catalog_id,
            revision: catalog.revision,
            roots: catalog.roots,
            file,
            chain,
        })
    }
    pub(super) fn verify_retained_provenance_locked(
        &self,
        prepared: &PreparedCatalogRestore,
        value: &authority::Authority,
    ) -> Result<()> {
        if prepared.source_kind
            != crate::backup::snapshots::CatalogRestoreSourceKind::RetainedPreRestore
        {
            return Ok(());
        }
        if let Some(pending) = &value.pending
            && (pending.selector.transaction_id.as_deref()
                != Some(prepared.transaction_id.as_str())
                || pending.expected_active != value.active
                || pending.expected_generation != value.generation)
        {
            return Err(blocked());
        }
        let witness = self.retained_locked(
            prepared
                .retained_transaction_id
                .as_deref()
                .ok_or_else(blocked)?,
            value,
            true,
        )?;
        if prepared.retained_generation.as_deref() != Some(witness.generation.to_string().as_str())
            || prepared.catalog_id != witness.catalog_id
            || prepared.source_catalog_revision != witness.revision
        {
            return Err(blocked());
        }
        Ok(())
    }
}
