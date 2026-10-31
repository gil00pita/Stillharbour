//! Backend-owned relative paths with held-parent identity proofs. Display names
//! are never decoded to produce native execution paths when a locator exists.
use super::{Object, failed};
use crate::{domain::Result, library::native, organization::portable_relative_target};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    path::{Component, Path},
};
const POLICY: &str = "native-parent-location-v1";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub policy: String,
    pub relative_native: Vec<u8>,
    pub parents: Vec<Object>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Destination {
    pub policy: String,
    pub relative_parent: String,
    pub parents: Vec<Object>,
}
fn parts(bytes: &[u8]) -> Result<Vec<OsString>> {
    if bytes.is_empty() || bytes.len() > 8192 {
        return Err(failed("The native source locator exceeds its bound."));
    }
    let path = native::decode_path(bytes)?;
    let mut result = Vec::new();
    for part in path.components() {
        let Component::Normal(name) = part else {
            return Err(failed(
                "Only root-relative native source components are accepted.",
            ));
        };
        if name
            .to_string_lossy()
            .to_ascii_lowercase()
            .ends_with(".photoslibrary")
        {
            return Err(failed(
                "Protected photo-library bundles cannot enter a copy locator.",
            ));
        }
        result.push(name.to_owned());
        if result.len() > 96 {
            return Err(failed("The native source locator exceeds 96 components."));
        }
    }
    if result.is_empty() {
        return Err(failed("The native source locator has no filename."));
    }
    Ok(result)
}
fn child(parent: &File, name: &OsStr, directory: bool) -> Result<File> {
    #[cfg(windows)]
    let file = crate::filesystem::windows::child(parent, name, directory)?;
    #[cfg(unix)]
    let file = {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(name.as_bytes())
            .map_err(|_| failed("Invalid native source name."))?;
        native::child_file(parent, &name, directory)?
    };
    if file.metadata()?.is_dir() != directory || (!directory && !file.metadata()?.is_file()) {
        return Err(failed("A native locator changed entry type."));
    }
    #[cfg(target_os = "macos")]
    {
        use crate::domain::{AppError, ErrorCode};
        use std::os::macos::fs::MetadataExt;
        if file.metadata()?.st_flags() & 0x40000000 != 0 {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Cloud placeholders are not copied through native locators.",
            ));
        }
    }
    Ok(file)
}
fn target_child(parent: &File, name: &OsStr) -> Result<File> {
    #[cfg(windows)]
    {
        crate::filesystem::windows::owned_directory_child(parent, name)
    }
    #[cfg(not(windows))]
    {
        child(parent, name, true)
    }
}
fn parent(
    root: &File,
    names: &[OsString],
    expected: &[Object],
    owned_output: bool,
) -> Result<File> {
    if names.len() != expected.len() {
        return Err(failed(
            "Native parent evidence does not match the locator depth.",
        ));
    }
    let mut parent = root.try_clone()?;
    for (name, object) in names.iter().zip(expected) {
        let next = if owned_output {
            target_child(&parent, name)?
        } else {
            child(&parent, name, true)?
        };
        if !object.matches(&next)? {
            return Err(failed(
                "A recorded native parent directory changed identity.",
            ));
        }
        parent = next;
    }
    Ok(parent)
}
impl Source {
    pub(super) fn capture(root: &File, path: &Path) -> Result<Self> {
        let relative_native = native::encode_path(path);
        let names = parts(&relative_native)?;
        let mut directory = root.try_clone()?;
        let mut parents = Vec::new();
        for name in &names[..names.len() - 1] {
            directory = child(&directory, name, true)?;
            parents.push(Object::of(&directory)?);
        }
        Ok(Self {
            policy: POLICY.into(),
            relative_native,
            parents,
        })
    }
    pub(super) fn validate(&self) -> Result<()> {
        let names = parts(&self.relative_native)?;
        if self.policy != POLICY || self.parents.len() + 1 != names.len() {
            return Err(failed("Unsupported or incomplete native source location."));
        }
        Ok(())
    }
    pub(super) fn open(&self, root: &File) -> Result<File> {
        self.validate()?;
        let mut names = parts(&self.relative_native)?;
        let leaf = names
            .pop()
            .ok_or_else(|| failed("Missing native source filename."))?;
        child(&parent(root, &names, &self.parents, false)?, &leaf, false)
    }
}
impl Destination {
    pub(super) fn capture(root: &File, relative_parent: &str) -> Result<Self> {
        let mut value = Self {
            policy: POLICY.into(),
            relative_parent: relative_parent.into(),
            parents: Vec::new(),
        };
        let names = value.names()?;
        let mut directory = root.try_clone()?;
        for name in &names {
            directory = target_child(&directory, name)?;
            value.parents.push(Object::of(&directory)?);
        }
        Ok(value)
    }
    fn names(&self) -> Result<Vec<OsString>> {
        if self.policy != POLICY
            || !portable_relative_target(&self.relative_parent)
            || self
                .relative_parent
                .split('/')
                .any(|name| name.to_ascii_lowercase().ends_with(".photoslibrary"))
        {
            return Err(failed("Unsupported or unsafe destination parent location."));
        }
        Ok(self
            .relative_parent
            .split('/')
            .map(OsString::from)
            .collect())
    }
    pub(super) fn validate(&self) -> Result<()> {
        if self.names()?.len() != self.parents.len() {
            return Err(failed("Incomplete destination parent identity evidence."));
        }
        Ok(())
    }
    pub(super) fn open(&self, root: &File) -> Result<File> {
        self.validate()?;
        parent(root, &self.names()?, &self.parents, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_source_bytes_drive_nested_copy_even_when_display_text_is_not_the_filename() {
        use super::super::{CopyContext, CopyFile, CopyOutcome, CopyPlan, Journal, RecoveredItem};
        let run = tempfile::tempdir().unwrap();
        let base = run.path().canonicalize().unwrap();
        let source = base.join("source");
        let output = base.join("output");
        std::fs::create_dir_all(source.join("source-folder")).unwrap();
        std::fs::create_dir_all(output.join("result-folder/deep")).unwrap();
        #[cfg(windows)]
        let name = {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[114, 97, 119, 45, 0xd800, 46, 98, 105, 110])
        };
        #[cfg(unix)]
        let name = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(vec![b'r', b'a', b'w', 0xff, b'.', b'b', b'i', b'n'])
        };
        let relative = std::path::PathBuf::from("source-folder").join(name);
        let original = b"constructor-owned native filename bytes";
        std::fs::write(source.join(&relative), original).unwrap();
        let source_scope = crate::filesystem::DirectoryChain::open(&source).unwrap();
        let output_scope = super::super::output_chain(&output).unwrap();
        let locator = Source::capture(source_scope.leaf(), &relative).unwrap();
        assert_eq!(locator.relative_native, native::encode_path(&relative));
        let locator: Source =
            serde_json::from_str(&serde_json::to_string(&locator).unwrap()).unwrap();
        let file = locator.open(source_scope.leaf()).unwrap();
        let token = uuid::Uuid::new_v4().to_string();
        let file = CopyFile {
            source: "display-only-not-a-native-path.bin".into(),
            target: "copied.bin".into(),
            stage: format!(".stillharbour-{token}-0.partial"),
            object: Object::of(&file).unwrap(),
            revision: native::revision(&file).unwrap(),
            bytes: original.len() as u64,
            source_location: Some(locator),
            destination_location: Some(
                Destination::capture(output_scope.leaf(), "result-folder/deep").unwrap(),
            ),
        };
        let mut plan = CopyPlan {
            id: token,
            digest: String::new(),
            source: Object::of(source_scope.leaf()).unwrap(),
            destination: Object::of(output_scope.leaf()).unwrap(),
            source_volume: native::volume_token(source_scope.leaf()).unwrap(),
            destination_volume: native::volume_token(output_scope.leaf()).unwrap(),
            files: vec![file],
        };
        plan.digest = plan.digest().unwrap();
        let journal = Journal::create(&base.join("operation.sqlite"), &plan).unwrap();
        let io = crate::jobs::paced_io::PacedIo::shared();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let validate = || Ok(());
        let context = CopyContext {
            plan: &plan,
            journal: &journal,
            source_root: source_scope.leaf(),
            destination: output_scope.leaf(),
            io: &io,
            cancel: &cancel,
            validate_scope: &validate,
        };
        assert_eq!(
            context.execute(None).unwrap(),
            CopyOutcome::CompletedVerified
        );
        assert_eq!(
            super::super::inspect_item(
                &plan.files[0],
                0,
                &journal,
                source_scope.leaf(),
                output_scope.leaf(),
                &io,
                &cancel
            )
            .unwrap(),
            RecoveredItem::PublishedVerified
        );
        assert_eq!(
            std::fs::read(output.join("result-folder/deep/copied.bin")).unwrap(),
            original
        );
        assert_eq!(std::fs::read(source.join(relative)).unwrap(), original);
        assert!(!source.join("display-only-not-a-native-path.bin").exists());
    }
    #[test]
    fn location_validation_refuses_escape_protected_bundles_unknown_policy_and_incomplete_parents()
    {
        let run = tempfile::tempdir().unwrap();
        let scope =
            crate::filesystem::DirectoryChain::open(&run.path().canonicalize().unwrap()).unwrap();
        for path in ["../outside", "./outside", "Package.photoslibrary/file.jpg"] {
            assert!(Source::capture(scope.leaf(), Path::new(path)).is_err());
        }
        for path in ["../outside", "Package.photoslibrary", "/absolute"] {
            assert!(Destination::capture(scope.leaf(), path).is_err());
        }
        let mut source = Source::capture(scope.leaf(), Path::new("file.bin")).unwrap();
        source.policy = "unknown".into();
        assert!(source.open(scope.leaf()).is_err());
        source.policy = POLICY.into();
        source.parents.push(Object::of(scope.leaf()).unwrap());
        assert!(source.open(scope.leaf()).is_err());
        source.relative_native = vec![0; 8193];
        assert!(source.validate().is_err());
    }
}
