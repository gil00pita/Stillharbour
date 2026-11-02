//! Instrumented-only proof of one harness-created persistent native session.
use super::*;
use serde::Deserialize;
#[cfg(test)]
mod tests;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Owner {
    version: u32,
    owner_nonce: String,
}
#[derive(Clone, Copy)]
pub enum CatalogFixtureRole {
    Appdata,
    Source,
    Repository,
    Restore,
}
pub struct CatalogFixtureSession {
    path: PathBuf,
    owner: String,
    chain: DirectoryChain,
    marker: File,
    marker_identity: FileIdentity,
    marker_digest: String,
}
impl CatalogFixtureSession {
    pub fn open(path: &Path, owner: &str) -> Result<Self> {
        if owner.len() != 32
            || !owner
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(blocked());
        }
        let supplied = DirectoryChain::open(path)?;
        let path = path.canonicalize()?;
        if path.parent() != Some(std::env::temp_dir().canonicalize()?.as_path())
            || !path
                .file_name()
                .ok_or_else(blocked)?
                .to_string_lossy()
                .starts_with(&format!("stillharbour-m4-catalog-{owner}-"))
        {
            return Err(blocked());
        }
        let chain = DirectoryChain::open(&path)?;
        if !same(chain.identity(), supplied.identity()) {
            return Err(blocked());
        }
        let (bytes, marker, _) = read_bytes(&path.join("session-owner.json"), 256)?;
        let record: Owner = serde_json::from_slice(&bytes).map_err(|_| blocked())?;
        if record.version != 1 || record.owner_nonce != owner {
            return Err(blocked());
        }
        let value = Self {
            path,
            owner: owner.into(),
            chain,
            marker_identity: filesystem::identity(&marker)?,
            marker,
            marker_digest: hash(&bytes),
        };
        value.revalidate()?;
        Ok(value)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn revalidate(&self) -> Result<()> {
        let chain = DirectoryChain::open(&self.path)?;
        let (bytes, marker, _) = read_bytes(&self.path.join("session-owner.json"), 256)?;
        if chain.identities().len() != self.chain.identities().len()
            || chain
                .identities()
                .iter()
                .zip(self.chain.identities())
                .any(|(a, b)| !same(a, b))
            || filesystem::identity(&marker)? != self.marker_identity
            || filesystem::identity(&self.marker)? != self.marker_identity
            || hash(&bytes) != self.marker_digest
        {
            return Err(blocked());
        }
        Ok(())
    }
    pub fn role_path(&self, role: CatalogFixtureRole) -> PathBuf {
        self.path.join(match role {
            CatalogFixtureRole::Appdata => "appdata",
            CatalogFixtureRole::Source => "source",
            CatalogFixtureRole::Repository => "repository",
            CatalogFixtureRole::Restore => "restore",
        })
    }
    pub fn create_role(&self, role: CatalogFixtureRole) -> Result<PathBuf> {
        self.revalidate()?;
        let path = self.role_path(role);
        let leaf = path
            .file_name()
            .ok_or_else(blocked)?
            .to_str()
            .ok_or_else(blocked)?;
        let parent = bridge::Root::existing(
            self.path.clone(),
            self.chain.identity().clone(),
            native::volume_token(self.chain.leaf())?,
        );
        bridge::make_directory(&parent, leaf)?;
        self.revalidate()?;
        Ok(path)
    }
    pub fn role_binding(&self, role: CatalogFixtureRole) -> Result<(FileIdentity, String)> {
        self.revalidate()?;
        let chain = DirectoryChain::open(&self.role_path(role))?;
        let result = (
            chain.identity().clone(),
            native::volume_token(chain.leaf())?,
        );
        self.revalidate()?;
        Ok(result)
    }
    pub fn cleanup_authority(&self, store: &ActivationStore) -> Result<()> {
        self.revalidate()?;
        let _guard = store.transaction.lock().map_err(|_| blocked())?;
        store.revalidate()?;
        let (identity, volume) = self.role_binding(CatalogFixtureRole::Appdata)?;
        if store.path != self.role_path(CatalogFixtureRole::Appdata)
            || !same(&identity, store.chain.identity())
            || native::volume_token(store.chain.leaf())? != volume
        {
            return Err(blocked());
        }
        let value = authority::read(&store.scope)?.ok_or_else(blocked)?;
        if value.initializing || value.pending.is_some() {
            return Err(blocked());
        }
        store.active(&value, false)?.revalidate()?;
        authority::delete_generated_session(&store.scope)?;
        if authority::read(&store.scope)?.is_some() {
            return Err(blocked());
        }
        self.revalidate()
    }
}
