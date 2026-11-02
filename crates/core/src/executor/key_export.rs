//! Native-shell-only, new-file recovery key export. Secret bytes never enter IPC.
use super::*;
use crate::backup::credentials::Secret;
use std::path::PathBuf;
use zeroize::Zeroizing;

/// The native shell obtains this scope from the profile service after a native
/// picker and explicit plaintext-key confirmation. No deserializable authority.
pub struct KeyExport {
    pub directory: PathBuf,
    pub identity: filesystem::FileIdentity,
    pub volume: String,
    pub journal_directory: PathBuf,
    pub label: String,
    pub secret: Secret,
}

pub fn export(scope: KeyExport) -> Result<String> {
    export_with_intent_hook(scope, || {})
}

fn same_directory_identity(a: &filesystem::FileIdentity, b: &filesystem::FileIdentity) -> bool {
    a.volume == b.volume && a.file == b.file
}

fn export_with_intent_hook(scope: KeyExport, after_intent: impl FnOnce()) -> Result<String> {
    use std::io::{Read, Write};
    if scope.label.len() > 512 || scope.label.chars().any(char::is_control) {
        return Err(failed("The recovery-key label is invalid."));
    }
    let output = output_chain(&scope.directory)?;
    let validate = || -> Result<()> {
        let current = output_chain(&scope.directory)?;
        let actual = filesystem::identity(current.leaf())?;
        if actual.volume != scope.identity.volume
            || actual.file != scope.identity.file
            || source_native::volume_token(current.leaf())? != scope.volume
            || current.identities().len() != output.identities().len()
            || !current
                .identities()
                .iter()
                .zip(output.identities())
                .all(|(a, b)| same_directory_identity(a, b))
        {
            return Err(failed(
                "The selected recovery-key directory changed. Retain any partial output for review.",
            ));
        }
        Ok(())
    };
    validate()?;
    let journal = output_chain(&scope.journal_directory)?;
    if journal
        .identities()
        .last()
        .zip(output.identities().last())
        .is_some_and(|(a, b)| same_directory_identity(a, b))
    {
        return Err(failed(
            "Recovery keys require a separate selected directory.",
        ));
    }
    let operation = uuid::Uuid::new_v4().to_string();
    let leaf = format!("Stillharbour-recovery-key-{operation}.txt");
    let mut body=Zeroizing::new(b"Stillharbour encrypted snapshot recovery key\nKeep this unencrypted file separately from the repository.\nProfile: ".to_vec());
    body.extend_from_slice(scope.label.as_bytes());
    body.extend_from_slice(b"\nRepository password: ");
    body.extend_from_slice(scope.secret.as_str()?.as_bytes());
    body.extend_from_slice(b"\n");
    // Nonsecret durable intent precedes the first output creation. Existing
    // entries and uncertain outputs are always retained; never resume blindly.
    let mut log = native::create_file(journal.leaf(), &format!("key-export-{operation}.jsonl"))?;
    let intent=serde_json::to_vec(&serde_json::json!({"operation":operation,"phase":"intent","directoryIdentity":scope.identity,"volume":scope.volume,"leaf":leaf}))
        .map_err(|_|failed("Recovery-key export intent could not be encoded."))?;
    log.write_all(&intent)?;
    log.write_all(b"\n")?;
    log.sync_all()?;
    after_intent();
    validate()?;
    let mut file = private_output::create(output.leaf(), &leaf)?;
    let identity = filesystem::identity(&file)?;
    file.write_all(&body)?;
    file.sync_all()?;
    if !native::entry_matches(output.leaf(), &leaf, &identity)? {
        return Err(failed("Recovery-key output changed; retain it for review."));
    }
    drop(file);
    validate()?;
    let mut reopened = native::read(output.leaf(), &leaf)?;
    if filesystem::identity(&reopened)? != identity {
        return Err(failed("Recovery-key output identity changed."));
    }
    let mut verified = Zeroizing::new(Vec::new());
    (&mut reopened).take(8193).read_to_end(&mut verified)?;
    if verified.as_slice() != body.as_slice() {
        return Err(failed(
            "Recovery-key output could not be verified; retain it for review.",
        ));
    }
    validate()?;
    log.write_all(b"{\"phase\":\"verified\"}\n")?;
    log.sync_all()?;
    Ok(scope.directory.join(leaf).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_key_export_is_new_private_verified_and_journaled() {
        let owned = tempfile::tempdir().unwrap();
        let base = owned.path().canonicalize().unwrap();
        let output = base.join("separate-output");
        let journals = base.join("data");
        std::fs::create_dir(&output).unwrap();
        std::fs::create_dir(&journals).unwrap();
        std::fs::write(output.join("neighbor.txt"), b"keep existing neighbor").unwrap();
        let directory = source_native::open_root(&output, None).unwrap();
        let saved = export_with_intent_hook(
            KeyExport {
                directory: output.clone(),
                identity: filesystem::identity(&directory).unwrap(),
                volume: source_native::volume_token(&directory).unwrap(),
                journal_directory: journals.clone(),
                label: "Generated test profile".into(),
                secret: Secret::fixture("generated-only-secret"),
            },
            || {
                // Changing an ancestor's directory link count is ordinary concurrent
                // activity, not a replacement of the selected native directory.
                std::fs::create_dir(base.join("concurrent-owned-directory")).unwrap();
            },
        )
        .unwrap();
        assert!(
            std::fs::read_to_string(&saved)
                .unwrap()
                .contains("Repository password: generated-only-secret\n")
        );
        assert_eq!(
            std::fs::read(output.join("neighbor.txt")).unwrap(),
            b"keep existing neighbor"
        );
        let events = std::fs::read_dir(journals)
            .unwrap()
            .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("\"phase\":\"verified\""));
        assert!(!events[0].contains("generated-only-secret"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(saved).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Security::{
                ACL_SIZE_INFORMATION, AclSizeInformation,
                Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
                DACL_SECURITY_INFORMATION, GetAclInformation, GetSecurityDescriptorControl,
                GetSecurityDescriptorDacl, SE_DACL_PROTECTED,
            };
            let file = std::fs::File::open(saved).unwrap();
            let mut descriptor = std::ptr::null_mut();
            // SAFETY: live file handle and valid security-descriptor output.
            assert_eq!(
                unsafe {
                    GetSecurityInfo(
                        file.as_raw_handle(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut descriptor,
                    )
                },
                0
            );
            let mut control = 0;
            let mut revision = 0;
            let mut dacl = std::ptr::null_mut();
            let mut present = 0;
            let mut defaulted = 0;
            // SAFETY: GetSecurityInfo returned a valid descriptor. All outputs
            // remain live until LocalFree after these bounded queries.
            unsafe {
                assert_ne!(
                    GetSecurityDescriptorControl(descriptor, &mut control, &mut revision),
                    0
                );
                assert_ne!(control & SE_DACL_PROTECTED, 0);
                assert_ne!(
                    GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted),
                    0
                );
                assert_ne!(present, 0);
                assert!(!dacl.is_null());
                let mut info = ACL_SIZE_INFORMATION::default();
                assert_ne!(
                    GetAclInformation(
                        dacl,
                        (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
                        std::mem::size_of_val(&info) as u32,
                        AclSizeInformation
                    ),
                    0
                );
                assert_eq!(info.AceCount, 2);
                windows_sys::Win32::Foundation::LocalFree(descriptor);
            }
        }
    }
    #[test]
    #[cfg(target_os = "macos")]
    fn recovery_key_export_refuses_actual_inherited_acl_before_secret_creation() {
        use std::os::unix::fs::OpenOptionsExt;
        let owned = tempfile::Builder::new()
            .prefix("stillharbour-key-acl-")
            .tempdir_in(std::env::temp_dir().canonicalize().unwrap())
            .unwrap();
        let base = owned.path().canonicalize().unwrap();
        assert!(base.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::write(base.join("owner"), uuid::Uuid::new_v4().to_string()).unwrap();
        let output = base.join("private-output");
        let journals = base.join("journals");
        std::fs::create_dir(&output).unwrap();
        std::fs::create_dir(&journals).unwrap();
        let parent = source_native::open_root(&output, None).unwrap();
        crate::backup::native_metadata::private_directory(&parent).unwrap();
        // Fixed native chmod arguments apply ONLY to this asserted generated
        // directory. No shell interpolation or privilege changes are involved.
        assert!(std::process::Command::new("/bin/chmod").args(["+a","everyone allow read,readattr,readextattr,readsecurity,file_inherit,directory_inherit"]).arg(&output).status().unwrap().success());
        assert!(crate::backup::native_metadata::private_directory(&parent).is_err());
        let inherited = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(output.join("inherited-empty"))
            .unwrap();
        assert!(
            crate::backup::native_metadata::verify_private_file(&inherited).is_err(),
            "0600 alone must not certify a file with inherited ACL entries"
        );
        assert_eq!(inherited.metadata().unwrap().len(), 0);
        drop(inherited);
        let result = export(KeyExport {
            directory: output.clone(),
            identity: filesystem::identity(&parent).unwrap(),
            volume: source_native::volume_token(&parent).unwrap(),
            journal_directory: journals.clone(),
            label: "Generated inherited-ACL refusal".into(),
            secret: Secret::fixture("generated-secret-must-not-be-written"),
        });
        assert!(result.is_err());
        let names = std::fs::read_dir(&output)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(names, vec![std::ffi::OsString::from("inherited-empty")]);
        for entry in std::fs::read_dir(journals).unwrap() {
            let bytes = std::fs::read(entry.unwrap().path()).unwrap();
            assert!(
                !bytes
                    .windows(b"generated-secret-must-not-be-written".len())
                    .any(|window| window == b"generated-secret-must-not-be-written")
            );
        }
    }
    #[test]
    fn private_creation_never_replaces_and_export_rejects_wrong_directory_identity() {
        let owned = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let base = owned.path().canonicalize().unwrap();
        let parent = output_chain(&base).unwrap();
        std::fs::write(owned.path().join("existing.txt"), b"original").unwrap();
        assert!(private_output::create(parent.leaf(), "existing.txt").is_err());
        assert_eq!(
            std::fs::read(owned.path().join("existing.txt")).unwrap(),
            b"original"
        );
        let wrong = source_native::open_root(other.path(), None).unwrap();
        assert!(
            export(KeyExport {
                directory: owned.path().to_owned(),
                identity: filesystem::identity(&wrong).unwrap(),
                volume: source_native::volume_token(&wrong).unwrap(),
                journal_directory: other.path().to_owned(),
                label: "Generated".into(),
                secret: Secret::fixture("generated-only")
            })
            .is_err()
        );
        assert_eq!(std::fs::read_dir(owned.path()).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(other.path()).unwrap().count(), 0);
    }
}
