use super::super::{inspection_inventory, model::DEFAULT_PHOTO_EXTENSIONS, store::RootRecord};
use super::*;
use unicode_normalization::UnicodeNormalization;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inventory {
    pub root: String,
    pub relative: Vec<u8>,
    pub source_name: String,
    pub signature: String,
}
pub(super) struct Companion {
    pub name: String,
    pub relative: PathBuf,
    pub extension: String,
    pub filename_specific: bool,
}
fn key(value: &str) -> String {
    value.nfc().flat_map(char::to_lowercase).collect()
}
pub(super) fn inspect(
    root: &RootRecord,
    relative: &Path,
    name: &str,
) -> Result<(Vec<Companion>, usize, String)> {
    let mut parent = native::open_root(&root.path, Some(&root.identity))?;
    if native::volume_token(&parent)? != root.volume {
        return Err(conflict());
    }
    let folder = relative.parent().unwrap_or(Path::new(""));
    for component in folder.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(invalid("Invalid source folder component."));
        }
        parent = inspection_inventory::child(&parent, component.as_os_str(), true)?;
    }
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| invalid("The source filename needs a lossless companion association."))?;
    let mut companions = Vec::new();
    let mut peers = 0;
    let mut entries = Vec::new();
    let mut visited = 0;
    inspection_inventory::each(&parent, |entry| {
        visited += 1;
        if visited > 32768 {
            return Err(invalid(
                "The source folder exceeds the bounded companion inventory. Export a smaller organized folder.",
            ));
        }
        let filename=entry.name.to_str().ok_or_else(||invalid("A source-folder name cannot be safely associated across this export's UTF-8 XMP boundary."))?;
        let path = Path::new(filename);
        let candidate = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let filename_specific = key(candidate) == key(name);
        if key(candidate) != key(stem) && !filename_specific {
            return Ok(());
        }
        entries.push((
            filename.to_owned(),
            entry.directory,
            entry.blocked,
            entry.bytes,
        ));
        if entry.blocked || entry.directory {
            return Err(invalid(
                "A possible companion is a link, placeholder, special entry or directory. Resolve it before XMP export.",
            ));
        }
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if DEFAULT_PHOTO_EXTENSIONS.contains(&extension.as_str()) {
            peers += 1;
        } else {
            companions.push(Companion {
                name: filename.into(),
                relative: folder.join(&entry.name),
                extension,
                filename_specific,
            });
        }
        if companions.len() > 32 {
            return Err(invalid("The source has more than 32 possible companions."));
        }
        Ok(())
    })?;
    entries.sort();
    companions.sort_by(|a, b| a.name.cmp(&b.name));
    let encoded =
        serde_json::to_vec(&entries).map_err(|_| invalid("Invalid companion inventory."))?;
    Ok((
        companions,
        peers,
        blake3::hash(&encoded).to_hex().to_string(),
    ))
}
