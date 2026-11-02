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

#[test]
#[cfg(target_os = "macos")]
fn native_metadata_preserves_provenance_bytes_under_a_distinct_versioned_policy() {
    use std::os::fd::AsRawFd;
    let fixture = Fixture::new();
    let name = c"com.apple.provenance";
    let supplied = b"owned native provenance fixture";
    // SAFETY: only a generated fixture is changed to establish the test input.
    assert_eq!(
        unsafe {
            libc::fsetxattr(
                fixture.original.as_raw_fd(),
                name.as_ptr(),
                supplied.as_ptr().cast(),
                supplied.len(),
                0,
                0,
            )
        },
        0
    );
    fn bytes(file: &File) -> Vec<u8> {
        use std::os::fd::AsRawFd;
        let mut value = vec![0u8; 4096];
        // SAFETY: generated fixture fd, fixed C name and live bounded buffer.
        let read = unsafe {
            libc::fgetxattr(
                file.as_raw_fd(),
                c"com.apple.provenance".as_ptr(),
                value.as_mut_ptr().cast(),
                value.len(),
                0,
                0,
            )
        };
        assert!(read > 0 && read <= 4096);
        value.truncate(read as usize);
        value
    }
    // The OS may preserve its own opaque value rather than the supplied bytes.
    // The actual descriptor value, never syscall success, is the expectation.
    let expected = bytes(&fixture.original);
    let revision = native::revision(&fixture.original).unwrap();
    let manifest = MetadataManifest::inspect(&fixture.original).unwrap();
    let encoded = serde_json::to_value(&manifest).unwrap();
    assert_eq!(encoded["version"], 2);
    assert_eq!(
        encoded["metadata"]["provenance"],
        serde_json::json!(expected)
    );
    let output = fixture.output();
    manifest.restore_new(&output).unwrap();
    manifest.compare(&output).unwrap();
    assert_eq!(bytes(&output), expected);
    assert_eq!(bytes(&fixture.original), expected);
    assert_eq!(native::revision(&fixture.original).unwrap(), revision);
    let mut changed = encoded.clone();
    let mut altered = expected.clone();
    altered[0] ^= 0xff;
    changed["metadata"]["provenance"] = serde_json::json!(altered);
    let changed: MetadataManifest = serde_json::from_value(changed).unwrap();
    assert_ne!(changed.digest().unwrap(), manifest.digest().unwrap());
    // Exercise the actual setter branch with a DIFFERENT opaque value. Some
    // hosts protect it and others permit it; success must always mean equality.
    if changed.restore_new(&output).is_ok() {
        assert_eq!(bytes(&output), altered);
    }
    assert_eq!(bytes(&fixture.original), expected);
    assert!(
        changed.compare(&fixture.original).is_err(),
        "Opaque provenance bytes cannot be ignored during comparison."
    );
    let mut forged = encoded.clone();
    forged["version"] = serde_json::json!(1);
    forged["policy"] = serde_json::json!(POLICY_V1);
    let forged: MetadataManifest = serde_json::from_value(forged).unwrap();
    assert!(
        forged.digest().is_err(),
        "v2 bytes cannot silently acquire v1 semantics"
    );
    let mut missing = encoded.clone();
    missing["metadata"]
        .as_object_mut()
        .unwrap()
        .remove("provenance");
    let missing: MetadataManifest = serde_json::from_value(missing).unwrap();
    assert!(
        missing.digest().is_err(),
        "v2 requires its captured provenance bytes"
    );
    let mut oversized = encoded;
    oversized["metadata"]["provenance"] = serde_json::json!(vec![0u8; 4097]);
    let oversized: MetadataManifest = serde_json::from_value(oversized).unwrap();
    assert!(oversized.restore_new(&output).is_err());
    assert_eq!(bytes(&fixture.original), expected);
    fixture.audit();
}

#[test]
fn native_metadata_legacy_macos_manifest_keeps_its_original_digest_and_empty_attribute_policy() {
    let fixture = Fixture::new();
    let encoded = serde_json::json!({"version":1,"policy":POLICY_V1,"origin":filesystem::identity(&fixture.original).unwrap(),"metadata":{"platform":"macos","uid":42,"gid":7,"mode":416,"flags":0,"created_seconds":1,"created_nanoseconds":2,"modified_seconds":3,"modified_nanoseconds":4}});
    let manifest: MetadataManifest = serde_json::from_value(encoded).unwrap();
    let old_bytes = br#"[1,"essential-no-extra-streams-v1-owner-permissions-creation-modification",{"platform":"macos","uid":42,"gid":7,"mode":416,"flags":0,"created_seconds":1,"created_nanoseconds":2,"modified_seconds":3,"modified_nanoseconds":4}]"#;
    assert_eq!(
        manifest.digest().unwrap(),
        blake3::hash(old_bytes).to_hex().to_string()
    );
    let saved = serde_json::to_string(&manifest).unwrap();
    assert!(!saved.contains("\"provenance\":"));
    let roundtrip: MetadataManifest = serde_json::from_str(&saved).unwrap();
    assert_eq!(roundtrip.digest().unwrap(), manifest.digest().unwrap());
    fixture.audit();
}

#[test]
#[cfg(target_os = "macos")]
fn native_metadata_unknown_target_attributes_are_retained_before_any_restore_writes() {
    use std::os::fd::AsRawFd;
    let fixture = Fixture::new();
    let manifest = MetadataManifest::inspect(&fixture.original).unwrap();
    let output = fixture.output();
    let bytes = b"retain this unsupported inherited target value";
    // SAFETY: generated newly owned output only, fixed name and bounded bytes.
    assert_eq!(
        unsafe {
            libc::fsetxattr(
                output.as_raw_fd(),
                c"com.stillharbour.target".as_ptr(),
                bytes.as_ptr().cast(),
                bytes.len(),
                0,
                0,
            )
        },
        0
    );
    let original_revision = native::revision(&fixture.original).unwrap();
    let output_revision = native::revision(&output).unwrap();
    let error = manifest.restore_new(&output).unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert_eq!(native::revision(&output).unwrap(), output_revision);
    assert_eq!(
        native::revision(&fixture.original).unwrap(),
        original_revision
    );
    let mut readback = [0u8; 64];
    // SAFETY: live generated descriptor, fixed name and bounded output buffer.
    let n = unsafe {
        libc::fgetxattr(
            output.as_raw_fd(),
            c"com.stillharbour.target".as_ptr(),
            readback.as_mut_ptr().cast(),
            readback.len(),
            0,
            0,
        )
    };
    assert_eq!(&readback[..n as usize], bytes);
    fixture.audit();
}
