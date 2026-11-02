#![cfg(any(windows, target_os = "macos"))]
use super::*;
use std::{
    io::Write,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};

struct Fixture {
    owned: tempfile::TempDir,
    path: PathBuf,
    original: File,
}
impl Fixture {
    fn new() -> Self {
        let owned = tempfile::Builder::new()
            .prefix("stillharbour-native-metadata-")
            .tempdir()
            .unwrap();
        let root = owned.path().canonicalize().unwrap();
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::write(root.join("owner"), uuid::Uuid::new_v4().to_string()).unwrap();
        let path = root.join("generated.photo");
        let mut original = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        original
            .write_all(b"generated ordinary media bytes")
            .unwrap();
        original.sync_all().unwrap();
        Self {
            owned,
            path,
            original,
        }
    }
    fn output(&self) -> File {
        let path = self
            .owned
            .path()
            .join(format!("restored-{}.photo", uuid::Uuid::new_v4()));
        let mut output = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        output.write_all(b"generated ordinary media bytes").unwrap();
        output.sync_all().unwrap();
        output
    }
    fn audit(&self) {
        assert_eq!(
            std::fs::read(&self.path).unwrap(),
            b"generated ordinary media bytes"
        );
    }
}

#[test]
fn native_metadata_inspection_is_stable_serializable_and_never_rewrites_original() {
    let fixture = Fixture::new();
    let before = native::revision(&fixture.original).unwrap();
    let first = MetadataManifest::inspect(&fixture.original).unwrap();
    let second = MetadataManifest::inspect(&fixture.original).unwrap();
    assert_eq!(first.digest().unwrap(), second.digest().unwrap());
    let encoded = serde_json::to_string(&first).unwrap();
    let decoded: MetadataManifest = serde_json::from_str(&encoded).unwrap();
    decoded.compare(&fixture.original).unwrap();
    assert_eq!(first.digest().unwrap(), decoded.digest().unwrap());
    assert!(
        decoded.restore_new(&fixture.original).is_err(),
        "Original metadata must never become an output target."
    );
    assert_eq!(native::revision(&fixture.original).unwrap(), before);
    fixture.audit();
}

#[test]
fn native_metadata_restores_exact_supported_policy_to_new_output_and_detects_changes() {
    let fixture = Fixture::new();
    let before = native::revision(&fixture.original).unwrap();
    let metadata = MetadataManifest::inspect(&fixture.original).unwrap();
    let target = fixture.output();
    target
        .set_modified(UNIX_EPOCH + Duration::from_secs(1_600_000_000))
        .unwrap();
    assert!(metadata.compare(&target).is_err());
    metadata.restore_new(&target).unwrap();
    metadata.compare(&target).unwrap();
    assert_eq!(
        MetadataManifest::inspect(&target)
            .unwrap()
            .digest()
            .unwrap(),
        metadata.digest().unwrap()
    );
    let mut malformed = metadata.clone();
    malformed.version = 999;
    assert!(malformed.restore_new(&target).is_err());
    assert_eq!(native::revision(&fixture.original).unwrap(), before);
    fixture.audit();
}

#[test]
#[cfg(windows)]
fn native_metadata_named_ads_is_retained_and_explicitly_unsupported() {
    let fixture = Fixture::new();
    let stream = PathBuf::from(format!("{}:preserve", fixture.path.display()));
    std::fs::write(&stream, b"unique ADS bytes").unwrap();
    let error = MetadataManifest::inspect(&fixture.original).unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert_eq!(std::fs::read(stream).unwrap(), b"unique ADS bytes");
    fixture.audit();
}

#[test]
#[cfg(windows)]
fn native_metadata_imported_descriptor_offsets_are_rejected_before_native_restore() {
    let fixture = Fixture::new();
    let mut metadata = MetadataManifest::inspect(&fixture.original).unwrap();
    let target = fixture.output();
    let before = native::revision(&target).unwrap();
    let NativeMetadata::Windows {
        security_descriptor,
        ..
    } = &mut metadata.metadata
    else {
        panic!("Wrong platform")
    };
    security_descriptor[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(metadata.restore_new(&target).is_err());
    assert_eq!(native::revision(&target).unwrap(), before);
    fixture.audit();
}

#[test]
#[cfg(windows)]
fn native_metadata_actual_mandatory_integrity_label_roundtrips_to_new_output() {
    let fixture = Fixture::new();
    super::windows::test_low_label(&fixture.original).unwrap();
    let before = native::revision(&fixture.original).unwrap();
    let metadata = MetadataManifest::inspect(&fixture.original).unwrap();
    let target = fixture.output();
    metadata.restore_new(&target).unwrap();
    metadata.compare(&target).unwrap();
    assert_eq!(native::revision(&fixture.original).unwrap(), before);
    fixture.audit();
}

#[test]
#[cfg(windows)]
fn native_metadata_unlabelled_source_restores_over_existing_new_output_label() {
    let fixture = Fixture::new();
    let metadata = MetadataManifest::inspect(&fixture.original).unwrap();
    let target = fixture.output();
    super::windows::test_low_label(&target).unwrap();
    assert!(metadata.compare(&target).is_err());
    let result = metadata.restore_new(&target);
    assert!(result.is_ok(), "{:?}", result.as_ref().err());
    metadata.compare(&target).unwrap();
    fixture.audit();
}

#[test]
#[cfg(windows)]
fn native_metadata_ordinary_hash_never_certifies_unknown_audit_security() {
    let fixture = Fixture::new();
    let mut metadata = MetadataManifest::inspect(&fixture.original).unwrap();
    let NativeMetadata::Windows {
        audit_descriptor, ..
    } = &mut metadata.metadata
    else {
        panic!("Wrong native platform")
    };
    *audit_descriptor = None;
    assert_eq!(metadata.digest().unwrap().len(), 64);
    assert!(!metadata.certified_replacement().unwrap());
    fixture.audit();
}

#[test]
#[cfg(windows)]
fn native_metadata_restores_source_permissions_across_different_owned_parent_acl() {
    let fixture = Fixture::new();
    let metadata = MetadataManifest::inspect(&fixture.original).unwrap();
    let parent = fixture.owned.path().join("private-output");
    std::fs::create_dir(&parent).unwrap();
    super::windows::test_private_directory(&parent).unwrap();
    let mut target = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(parent.join("restored.photo"))
        .unwrap();
    target.write_all(b"generated ordinary media bytes").unwrap();
    target.sync_all().unwrap();
    let result = metadata.restore_new(&target);
    assert!(result.is_ok(), "{:?}", result.as_ref().err());
    metadata.compare(&target).unwrap();
    fixture.audit();
}

#[test]
#[cfg(target_os = "macos")]
fn native_metadata_xattrs_and_resource_forks_are_retained_and_explicitly_unsupported() {
    use std::os::fd::AsRawFd;
    for name in [c"com.stillharbour.fixture", c"com.apple.ResourceFork"] {
        let fixture = Fixture::new();
        let bytes = b"unique native stream bytes";
        // SAFETY: generated owned fixture, bounded live C name/value buffers.
        assert_eq!(
            unsafe {
                libc::fsetxattr(
                    fixture.original.as_raw_fd(),
                    name.as_ptr(),
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    0,
                    0,
                )
            },
            0
        );
        assert_eq!(
            MetadataManifest::inspect(&fixture.original)
                .unwrap_err()
                .code,
            ErrorCode::Unsupported
        );
        let mut actual = [0u8; 64];
        // SAFETY: generated fixture fd, name and writable bounded value buffer.
        let read = unsafe {
            libc::fgetxattr(
                fixture.original.as_raw_fd(),
                name.as_ptr(),
                actual.as_mut_ptr().cast(),
                actual.len(),
                0,
                0,
            )
        };
        assert_eq!(read, bytes.len() as isize);
        assert_eq!(&actual[..bytes.len()], bytes);
        fixture.audit();
    }
}
