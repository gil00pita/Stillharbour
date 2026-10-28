//! Small app-owned preferences; never an original-file destination or catalog migration.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem,
    library::{
        filters::{CompiledFilter, invalid},
        model::{DEFAULT_PHOTO_EXTENSIONS, RootMode, ScanFilter},
    },
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GalleryView {
    Gallery,
    Grid,
    List,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThumbnailSize {
    Compact,
    Comfortable,
    Large,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GridFit {
    Contain,
    Cover,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Capture,
    Name,
    Size,
    Added,
    Modified,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppPreferences {
    pub version: u32,
    pub default_root_id: Option<String>,
    pub root_mode: RootMode,
    pub scan_filter: ScanFilter,
    pub gallery_view: GalleryView,
    pub thumbnail_size: ThumbnailSize,
    pub grid_fit: GridFit,
    pub sort: SortOrder,
    pub navigation_collapsed: bool,
}
impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            default_root_id: None,
            root_mode: RootMode::Managed,
            scan_filter: ScanFilter {
                extensions: DEFAULT_PHOTO_EXTENSIONS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                ..Default::default()
            },
            gallery_view: GalleryView::Gallery,
            thumbnail_size: ThumbnailSize::Comfortable,
            grid_fit: GridFit::Contain,
            sort: SortOrder::Capture,
            navigation_collapsed: false,
        }
    }
}
impl AppPreferences {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || !self.scan_filter.include_globs.is_empty()
            || self.scan_filter.from_date.is_some()
            || self.scan_filter.through_date.is_some()
        {
            return Err(invalid(
                "Unsupported default settings version or scan restrictions.",
            ));
        }
        if self
            .default_root_id
            .as_ref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        {
            return Err(invalid("Default library identity is invalid."));
        }
        CompiledFilter::new(self.scan_filter.clone())?;
        Ok(())
    }
}

pub struct PreferencesStore {
    directory: PathBuf,
    gate: Mutex<()>,
}
impl PreferencesStore {
    /// Directory comes only from the native app's private data directory.
    pub fn new(directory: &Path) -> Result<Self> {
        Ok(Self {
            directory: directory.canonicalize()?,
            gate: Mutex::new(()),
        })
    }
    fn read(&self) -> Result<AppPreferences> {
        let path = self.directory.join("preferences.json");
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AppPreferences::default());
            }
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_file() {
            return Err(invalid("Settings must be a regular app-owned file."));
        }
        let file = filesystem::open_readonly(&path)?;
        let mut bytes = Vec::new();
        file.take(65_537).read_to_end(&mut bytes)?;
        if bytes.len() > 65_536 {
            return Err(invalid("Settings file exceeds its supported size."));
        }
        let settings: AppPreferences = serde_json::from_slice(&bytes).map_err(|e| {
            invalid("Saved settings could not be read; the file has been preserved.").caused_by(e)
        })?;
        settings.validate()?;
        Ok(settings)
    }
    pub fn get(&self) -> Result<AppPreferences> {
        let _gate = self
            .gate
            .lock()
            .map_err(|_| invalid("Settings are busy."))?;
        self.read()
    }
    pub fn save(&self, settings: AppPreferences) -> Result<AppPreferences> {
        settings.validate()?;
        let _gate = self
            .gate
            .lock()
            .map_err(|_| invalid("Settings are busy."))?;
        self.read()?; // Never replace unreadable or future-version settings with a default.
        let bytes = serde_json::to_vec_pretty(&settings)
            .map_err(|e| invalid("Settings could not be encoded.").caused_by(e))?;
        if bytes.len() > 65_536 {
            return Err(invalid("Settings exceed their supported size."));
        }
        #[cfg(not(unix))]
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Saving settings needs native durability validation on this platform.",
        ));
        #[cfg(unix)]
        {
            let mut staged = tempfile::NamedTempFile::new_in(&self.directory)?;
            staged.write_all(&bytes)?;
            staged.as_file().sync_all()?;
            staged
                .persist(self.directory.join("preferences.json"))
                .map_err(|e| {
                    AppError::new(ErrorCode::Io, "Settings could not be saved.").caused_by(e.error)
                })?;
            filesystem::sync_directory(&self.directory)?;
            Ok(settings)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_roundtrip_and_invalid_updates_preserve_saved_settings() {
        let data = tempfile::tempdir().unwrap();
        let store = PreferencesStore::new(data.path()).unwrap();
        let mut settings = store.get().unwrap();
        assert_eq!(settings.scan_filter.extensions.len(), 33);
        settings.root_mode = RootMode::Referenced;
        settings.gallery_view = GalleryView::List;
        settings.scan_filter.include_hidden_files = true;
        store.save(settings.clone()).unwrap();
        assert_eq!(
            PreferencesStore::new(data.path()).unwrap().get().unwrap(),
            settings
        );
        let before = std::fs::read(data.path().join("preferences.json")).unwrap();
        settings.scan_filter.min_bytes = Some("50".into());
        settings.scan_filter.max_bytes = Some("1".into());
        assert!(store.save(settings).is_err());
        assert_eq!(
            std::fs::read(data.path().join("preferences.json")).unwrap(),
            before
        );
    }
    #[test]
    fn corrupt_and_future_preferences_are_not_overwritten() {
        let data = tempfile::tempdir().unwrap();
        let store = PreferencesStore::new(data.path()).unwrap();
        for bytes in [
            b"broken".to_vec(),
            serde_json::to_vec(&AppPreferences {
                version: 99,
                ..Default::default()
            })
            .unwrap(),
            vec![b' '; 65_537],
        ] {
            std::fs::write(data.path().join("preferences.json"), &bytes).unwrap();
            assert!(store.get().is_err());
            assert!(store.save(AppPreferences::default()).is_err());
            assert_eq!(
                std::fs::read(data.path().join("preferences.json")).unwrap(),
                bytes
            );
        }
    }
    #[test]
    #[cfg(unix)]
    fn linked_preferences_never_read_or_replace_the_target() {
        let data = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"preserve").unwrap();
        std::os::unix::fs::symlink(outside.path(), data.path().join("preferences.json")).unwrap();
        let store = PreferencesStore::new(data.path()).unwrap();
        assert!(store.get().is_err());
        assert!(store.save(AppPreferences::default()).is_err());
        assert_eq!(std::fs::read(outside.path()).unwrap(), b"preserve");
    }
}
