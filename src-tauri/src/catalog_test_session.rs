//! Desktop-test-only persistent lifecycle; production never reads these env vars.
use super::*;
use std::path::Path;
use stillharbour_core::catalog_restore::activation::{CatalogFixtureRole, CatalogFixtureSession};
pub(super) struct NativeTestData {
    temporary: Option<tempfile::TempDir>,
    session: Option<Arc<CatalogFixtureSession>>,
    path: PathBuf,
}
impl NativeTestData {
    pub(super) fn open() -> Result<Self> {
        match (
            std::env::var_os("STILLHARBOUR_CATALOG_NATIVE_SESSION_ROOT"),
            std::env::var_os("STILLHARBOUR_CATALOG_NATIVE_SESSION_OWNER"),
        ) {
            (None, None) => {
                let temporary = tempfile::Builder::new()
                    .prefix("stillharbour-native-test-")
                    .tempdir()?;
                let path = temporary.path().to_owned();
                Ok(Self {
                    temporary: Some(temporary),
                    session: None,
                    path,
                })
            }
            (Some(path), Some(owner)) => {
                let owner = owner.into_string().map_err(|_| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "The generated session owner must be ASCII hex.",
                    )
                })?;
                let session = Arc::new(CatalogFixtureSession::open(Path::new(&path), &owner)?);
                let path = session.role_path(CatalogFixtureRole::Appdata);
                match std::fs::symlink_metadata(&path) {
                    Ok(_) => {
                        session.role_binding(CatalogFixtureRole::Appdata)?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        session.create_role(CatalogFixtureRole::Appdata)?;
                    }
                    Err(e) => return Err(e.into()),
                }
                Ok(Self {
                    temporary: None,
                    session: Some(session),
                    path,
                })
            }
            _ => Err(AppError::new(
                ErrorCode::InvalidInput,
                "Both native session environment values must describe one generated owned run.",
            )),
        }
    }
    pub(super) fn path(&self) -> &Path {
        self.temporary
            .as_ref()
            .map_or(self.path.as_path(), |dir| dir.path())
    }
    pub(super) fn session(&self) -> Option<&Arc<CatalogFixtureSession>> {
        self.session.as_ref()
    }
}
