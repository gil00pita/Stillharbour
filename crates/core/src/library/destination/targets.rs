//! Bounded metadata-only target observations. Absence is neither a reservation
//! nor proof that two future names are distinct under native filesystem rules.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem,
    library::native,
    organization::{OrganizationRow, OrganizationTargetReview, OrganizationTargetState},
};
use std::{collections::BTreeMap, ffi::OsString, fs::File};

const MAX_DIRECTORIES: usize = 1024;

struct Entry {
    name: OsString,
    file: u64,
    volume: u64,
    directory: bool,
    unsafe_entry: bool,
}

#[cfg(windows)]
fn lookup(parent: &File, name: &str) -> Result<Option<Entry>> {
    use windows_sys::Win32::Storage::FileSystem::*;
    filesystem::windows::destination_entry(parent, std::ffi::OsStr::new(name))?
        .map(|entry| {
            Ok(Entry {
                name: entry.name,
                file: entry.file_id,
                volume: filesystem::identity(parent)?.volume,
                directory: entry.attributes & FILE_ATTRIBUTE_DIRECTORY != 0,
                unsafe_entry: entry.attributes
                    & (FILE_ATTRIBUTE_REPARSE_POINT
                        | FILE_ATTRIBUTE_OFFLINE
                        | FILE_ATTRIBUTE_RECALL_ON_OPEN
                        | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                    != 0,
            })
        })
        .transpose()
}

#[cfg(unix)]
fn lookup(parent: &File, name: &str) -> Result<Option<Entry>> {
    use rustix::fs::{AtFlags, FileType, statat};
    let stat = match statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => stat,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(error) => return Err(std::io::Error::from(error).into()),
    };
    let kind = FileType::from_raw_mode(stat.st_mode);
    #[cfg(target_os = "macos")]
    let placeholder = stat.st_flags & 0x40000000 != 0;
    #[cfg(not(target_os = "macos"))]
    let placeholder = false;
    #[cfg(target_os = "macos")]
    let volume = stat.st_dev as u64;
    #[cfg(not(target_os = "macos"))]
    let volume = stat.st_dev;
    Ok(Some(Entry {
        name: name.into(),
        file: stat.st_ino,
        volume,
        directory: kind == FileType::Directory,
        unsafe_entry: placeholder || !matches!(kind, FileType::Directory | FileType::RegularFile),
    }))
}

fn open_child(parent: &File, entry: &Entry) -> Result<File> {
    #[cfg(windows)]
    let handle = filesystem::windows::child(parent, &entry.name, true)?;
    #[cfg(unix)]
    let handle = {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(entry.name.as_bytes()).map_err(|_| {
            AppError::new(ErrorCode::InvalidInput, "Invalid native target component.")
        })?;
        native::child_file(parent, &name, true)?
    };
    let identity = filesystem::identity(&handle)?;
    if identity.file != entry.file || identity.volume != entry.volume {
        return Err(changed());
    }
    Ok(handle)
}

fn changed() -> AppError {
    AppError::new(
        ErrorCode::Conflict,
        "The destination changed during target review. Check it again.",
    )
}

pub(super) fn review(
    root: &File,
    rows: &[OrganizationRow],
) -> Result<(Vec<OrganizationTargetReview>, Vec<u8>)> {
    let root_volume = filesystem::identity(root)?.volume;
    let mut directories = vec![root.try_clone()?];
    let mut revisions = vec![native::revision(root)?];
    let mut cache = BTreeMap::from([(String::new(), 0usize)]);
    let mut results = Vec::with_capacity(rows.len());
    let mut evidence = Vec::new();
    for row in rows {
        let mut result = OrganizationTargetReview {
            asset_id: row.asset_id.clone(),
            target_relative_path: row.target_relative_path.clone(),
            state: OrganizationTargetState::PreviewBlocked,
            blocking_component: None,
        };
        if row.issues.is_empty()
            && let Some(path) = &row.target_relative_path
        {
            let parts = path.split('/').collect::<Vec<_>>();
            if parts
                .iter()
                .any(|part| part.to_ascii_lowercase().ends_with(".photoslibrary"))
            {
                result.state = OrganizationTargetState::UnsupportedAncestor;
                result.blocking_component = Some(path.clone());
                results.push(result);
                continue;
            }
            let mut parent = 0;
            let mut prefix = String::new();
            for (index, part) in parts.iter().enumerate() {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(part);
                result.blocking_component = Some(prefix.clone());
                let entry = lookup(&directories[parent], part)?;
                evidence.push((
                    row.asset_id.clone(),
                    prefix.clone(),
                    entry.as_ref().map(|e| {
                        (
                            native::encode_path(std::path::Path::new(&e.name)),
                            e.volume,
                            e.file,
                            e.directory,
                            e.unsafe_entry,
                        )
                    }),
                ));
                let Some(entry) = entry else {
                    result.state = OrganizationTargetState::UnoccupiedObserved;
                    result.blocking_component = None;
                    break;
                };
                if index + 1 == parts.len() {
                    result.state = OrganizationTargetState::Occupied;
                    break;
                }
                if entry.unsafe_entry
                    || entry.volume != root_volume
                    || entry
                        .name
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .ends_with(".photoslibrary")
                {
                    result.state = OrganizationTargetState::UnsupportedAncestor;
                    break;
                }
                if !entry.directory {
                    result.state = OrganizationTargetState::AncestorNotDirectory;
                    break;
                }
                if let Some(&known) = cache.get(&prefix) {
                    let identity = filesystem::identity(&directories[known])?;
                    if identity.file != entry.file || identity.volume != entry.volume {
                        return Err(changed());
                    }
                    parent = known;
                } else {
                    if directories.len() >= MAX_DIRECTORIES {
                        return Err(AppError::new(
                            ErrorCode::Unsupported,
                            "The target review exceeds 1,024 existing directories. Use a smaller selection.",
                        ));
                    }
                    let handle = open_child(&directories[parent], &entry)?;
                    revisions.push(native::revision(&handle)?);
                    parent = directories.len();
                    directories.push(handle);
                    cache.insert(prefix.clone(), parent);
                }
            }
        }
        results.push(result);
    }
    for (directory, revision) in directories.iter().zip(&revisions) {
        if native::revision(directory)? != *revision {
            return Err(changed());
        }
    }
    let fingerprint = serde_json::to_vec(&(
        "organization-target-observation-v1",
        &revisions,
        evidence,
        &results,
    ))
    .map_err(|error| {
        AppError::new(ErrorCode::Io, "Could not fingerprint target observations.").caused_by(error)
    })?;
    Ok((results, fingerprint))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(path: &str) -> OrganizationRow {
        OrganizationRow {
            asset_id: path.into(),
            root_id: "fixture".into(),
            source_path: "source.jpg".into(),
            source_revision: "1".into(),
            source_components: None,
            target_relative_path: Some(path.into()),
            sequence: 1,
            bytes: "5".into(),
            issues: Vec::new(),
            used_undated: false,
            companion_candidates: 0,
        }
    }
    fn root(path: &std::path::Path) -> File {
        #[cfg(windows)]
        {
            filesystem::windows::open_root(path).unwrap()
        }
        #[cfg(unix)]
        {
            filesystem::open_readonly(path).unwrap()
        }
    }

    #[test]
    fn target_observations_distinguish_occupied_ancestors_and_absent_names_without_writes() {
        use OrganizationTargetState::*;
        let fixture = tempfile::tempdir().unwrap();
        std::fs::create_dir(fixture.path().join("nested")).unwrap();
        std::fs::write(fixture.path().join("existing.JPG"), b"destination-only").unwrap();
        std::fs::write(fixture.path().join("nested/photo.JPG"), b"nested original").unwrap();
        std::fs::write(fixture.path().join("file"), b"parent is a file").unwrap();
        let names = [
            "existing.JPG",
            "nested/photo.JPG",
            "free.JPG",
            "new/child.JPG",
            "file/child.JPG",
            "new/Private.photoslibrary/child.JPG",
        ];
        let handle = root(fixture.path());
        let rows = names.map(row);
        let (observed, fingerprint) = review(&handle, &rows).unwrap();
        assert_eq!(
            observed.iter().map(|r| r.state.clone()).collect::<Vec<_>>(),
            [
                Occupied,
                Occupied,
                UnoccupiedObserved,
                UnoccupiedObserved,
                AncestorNotDirectory,
                UnsupportedAncestor
            ]
        );
        assert_eq!(observed[4].blocking_component.as_deref(), Some("file"));
        assert_eq!(review(&handle, &rows).unwrap().1, fingerprint);
        drop(handle);
        assert_eq!(
            std::fs::read(fixture.path().join("existing.JPG")).unwrap(),
            b"destination-only"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("nested/photo.JPG")).unwrap(),
            b"nested original"
        );
        assert_eq!(std::fs::read_dir(fixture.path()).unwrap().count(), 3);
        std::fs::write(fixture.path().join("free.JPG"), b"new conflict").unwrap();
        let (changed, new_fingerprint) = review(&root(fixture.path()), &rows).unwrap();
        assert_eq!(changed[2].state, Occupied);
        assert_ne!(fingerprint, new_fingerprint);
    }

    #[test]
    fn target_lookup_uses_native_existing_case_and_unicode_resolution() {
        let fixture = tempfile::tempdir().unwrap();
        for name in ["Photo.JPG", "caf\u{e9}.JPG", "\u{c4}.JPG"] {
            std::fs::write(fixture.path().join(name), b"native name").unwrap();
        }
        let handle = root(fixture.path());
        for name in ["PHOTO.JPG", "cafe\u{301}.JPG", "\u{e4}.JPG"] {
            let expected = std::fs::symlink_metadata(fixture.path().join(name)).is_ok();
            let (result, _) = review(&handle, &[row(name)]).unwrap();
            assert_eq!(
                result[0].state == OrganizationTargetState::Occupied,
                expected,
                "{name}"
            );
        }
        assert_eq!(std::fs::read_dir(fixture.path()).unwrap().count(), 3);
    }

    #[test]
    fn target_observations_never_follow_a_link_or_junction_ancestor() {
        let fixture = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("photo.JPG"), b"outside content").unwrap();
        let link = fixture.path().join("alias");
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        #[cfg(windows)]
        assert!(std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:STILLHARBOUR_TEST_JUNCTION -Target $env:STILLHARBOUR_TEST_TARGET | Out-Null"])
            .env("STILLHARBOUR_TEST_JUNCTION", &link).env("STILLHARBOUR_TEST_TARGET", outside.path()).status().unwrap().success());
        let (observed, _) = review(
            &root(fixture.path()),
            &[row("alias/photo.JPG"), row("alias")],
        )
        .unwrap();
        assert_eq!(
            observed[0].state,
            OrganizationTargetState::UnsupportedAncestor
        );
        assert_eq!(observed[1].state, OrganizationTargetState::Occupied);
        assert_eq!(
            std::fs::read(outside.path().join("photo.JPG")).unwrap(),
            b"outside content"
        );
    }

    #[test]
    fn preview_issues_cannot_become_an_unoccupied_target_approval() {
        let fixture = tempfile::tempdir().unwrap();
        let mut blocked = row("free.JPG");
        blocked
            .issues
            .push(crate::organization::OrganizationIssue::Collision);
        let (observed, _) = review(&root(fixture.path()), &[blocked]).unwrap();
        assert_eq!(observed[0].state, OrganizationTargetState::PreviewBlocked);
    }
}
