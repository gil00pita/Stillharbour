//! Conservative whole-bundle observations. These are not skip approvals.
use crate::{library::native, operations::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub(super) struct Member {
    pub id: String,
    pub bundles: Vec<String>,
    pub path: PathBuf,
    pub primary: bool,
}
pub(super) struct Keeper {
    pub source: String,
    pub asset: String,
    pub root: String,
    pub path: PathBuf,
    pub media: bool,
}

pub(super) fn observe(
    members: &[Member],
    keepers: &[Keeper],
    files: &[ImportComparisonFile],
    complete: bool,
) -> Vec<ImportBundleObservation> {
    let mut pending: BTreeSet<usize> = (0..members.len()).collect();
    let mut result = Vec::new();
    while let Some(first) = pending.pop_first() {
        let mut indices = BTreeSet::from([first]);
        let mut bundles: BTreeSet<String> = members[first].bundles.iter().cloned().collect();
        loop {
            let connected: Vec<_> = pending
                .iter()
                .copied()
                .filter(|index| {
                    members[*index]
                        .bundles
                        .iter()
                        .any(|id| bundles.contains(id))
                })
                .collect();
            if connected.is_empty() {
                break;
            }
            for index in connected {
                pending.remove(&index);
                indices.insert(index);
                bundles.extend(members[index].bundles.iter().cloned());
            }
        }
        let group: Vec<_> = indices.iter().map(|index| &members[*index]).collect();
        let mut observation = ImportBundleObservation {
            bundle_asset_ids: bundles.into_iter().collect(),
            file_ids: group.iter().map(|file| file.id.clone()).collect(),
            disposition: ImportBundleDisposition::NeedsAttention,
            existing_groups: Vec::new(),
            reason: "Incomplete comparison or ambiguous bundle layout; review the whole bundle."
                .into(),
        };
        if complete
            && !observation.bundle_asset_ids.is_empty()
            && group.iter().all(|member| {
                files.iter().any(|file| {
                    file.file_id == member.id && file.state != ImportFileComparison::NeedsAttention
                })
            })
        {
            resolve(&group, keepers, &mut observation);
        }
        result.push(observation);
    }
    result
}

fn resolve(group: &[&Member], keepers: &[Keeper], observation: &mut ImportBundleObservation) {
    let primaries: Vec<_> = group.iter().filter(|file| file.primary).collect();
    let Some(primary) = primaries.first() else {
        return;
    };
    let Some(stem) = primary.path.file_stem().and_then(|name| name.to_str()) else {
        return;
    };
    if stem.is_empty()
        || primaries.iter().any(|file| {
            file.path.parent() != primary.path.parent()
                || file.path.file_stem().and_then(|name| name.to_str()) != Some(stem)
        })
    {
        return;
    }
    // Exact Unicode suffixes only. Non-Unicode or alternative companion naming
    // remains reviewable; lossy display strings never form grouping keys.
    let mut options: Option<BTreeMap<String, ImportBundleMatch>> = None;
    for member in group {
        if member.path.parent() != primary.path.parent() {
            return;
        }
        let Some(suffix) = member
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix(stem))
            .filter(|suffix| suffix.starts_with('.') && suffix.len() > 1)
        else {
            return;
        };
        let mut choices: BTreeMap<String, Vec<&Keeper>> = BTreeMap::new();
        for keeper in keepers
            .iter()
            .filter(|keeper| keeper.source == member.id && (!member.primary || keeper.media))
        {
            let Some(candidate_stem) = keeper
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_suffix(suffix))
                .filter(|stem| !stem.is_empty())
            else {
                continue;
            };
            let key = blake3::hash(
                &serde_json::to_vec(&(
                    &keeper.root,
                    native::encode_path(
                        keeper
                            .path
                            .parent()
                            .unwrap_or_else(|| std::path::Path::new("")),
                    ),
                    candidate_stem,
                ))
                .expect("bounded native grouping tuple serializes"),
            )
            .to_hex()
            .to_string();
            choices.entry(key).or_default().push(keeper);
        }
        // Multiple candidates for one member in the same group are ambiguous.
        let choices: BTreeMap<_, _> = choices
            .into_iter()
            .filter_map(|(id, values)| {
                (values.len() == 1).then(|| {
                    let keeper = values[0];
                    (
                        id.clone(),
                        ImportBundleMatch {
                            id,
                            root_id: keeper.root.clone(),
                            members: vec![ImportBundleMember {
                                file_id: member.id.clone(),
                                asset_id: keeper.asset.clone(),
                            }],
                        },
                    )
                })
            })
            .collect();
        options = Some(match options {
            None => choices,
            Some(previous) => previous
                .into_iter()
                .filter_map(|(id, mut value)| {
                    let next = choices.get(&id)?;
                    if value
                        .members
                        .iter()
                        .any(|member| member.asset_id == next.members[0].asset_id)
                    {
                        return None;
                    }
                    value.members.extend(next.members.clone());
                    Some((id, value))
                })
                .collect(),
        });
    }
    observation.existing_groups = options.unwrap_or_default().into_values().collect();
    if observation.existing_groups.is_empty() {
        observation.disposition = ImportBundleDisposition::CopyWholeBundle;
        observation.reason="No complete existing bundle was established within the bounded indexed comparison. Keep all selected members together when copying.".into();
    } else {
        observation.disposition = ImportBundleDisposition::ReviewExistingBundle;
        observation.reason="Every selected member matched a distinct indexed file in one folder with a common renamed stem; primary matches are indexed media. Fresh complete-byte verification and a separate import approval are still required before skipping.".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn member(id: &str, path: &str, bundles: &[&str], primary: bool) -> Member {
        Member {
            id: id.into(),
            path: path.into(),
            bundles: bundles.iter().map(|id| (*id).into()).collect(),
            primary,
        }
    }
    fn keeper(source: &str, asset: &str, root: &str, path: &str, media: bool) -> Keeper {
        Keeper {
            source: source.into(),
            asset: asset.into(),
            root: root.into(),
            path: path.into(),
            media,
        }
    }
    fn observations(
        members: &[Member],
        keepers: &[Keeper],
        complete: bool,
    ) -> Vec<ImportBundleObservation> {
        let files = members
            .iter()
            .map(|member| ImportComparisonFile {
                file_id: member.id.clone(),
                source_path: String::new(),
                bytes: "1".into(),
                state: ImportFileComparison::ExactBytesCompared,
                matches: Vec::new(),
                further_candidates_uncompared: false,
            })
            .collect::<Vec<_>>();
        observe(members, keepers, &files, complete)
    }
    fn bundle() -> Vec<Member> {
        vec![
            member("raw", "source/Photo.RAW", &["raw", "jpg"], true),
            member("jpg", "source/Photo.JPG", &["jpg"], true),
            member("sidecar", "source/Photo.JPG.xmp", &["jpg"], false),
        ]
    }
    fn existing() -> Vec<Keeper> {
        vec![
            keeper("raw", "r", "library", "existing/Renamed.RAW", true),
            keeper("jpg", "j", "library", "existing/Renamed.JPG", true),
            keeper("sidecar", "x", "library", "existing/Renamed.JPG.xmp", false),
        ]
    }
    #[test]
    fn renamed_raw_jpeg_and_compound_sidecar_have_one_complete_connected_group() {
        let result = observations(&bundle(), &existing(), true);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].disposition,
            ImportBundleDisposition::ReviewExistingBundle
        );
        assert_eq!(result[0].bundle_asset_ids, vec!["jpg", "raw"]);
        assert_eq!(result[0].existing_groups.len(), 1);
        assert_eq!(result[0].existing_groups[0].members.len(), 3);
    }
    #[test]
    fn scattered_bytes_never_form_a_bundle() {
        for (index, root, path) in [
            (2, "library", "elsewhere/Renamed.JPG.xmp"),
            (2, "other-root", "existing/Renamed.JPG.xmp"),
            (2, "library", "existing/Unrelated.JPG.xmp"),
        ] {
            let mut keepers = existing();
            keepers[index].root = root.into();
            keepers[index].path = path.into();
            let result = observations(&bundle(), &keepers, true);
            assert_eq!(
                result[0].disposition,
                ImportBundleDisposition::CopyWholeBundle
            );
            assert!(result[0].existing_groups.is_empty());
        }
    }
    #[test]
    fn missing_companion_or_nonmedia_primary_keeps_all_source_members_for_copy() {
        let mut keepers = existing();
        keepers.pop();
        assert_eq!(
            observations(&bundle(), &keepers, true)[0].disposition,
            ImportBundleDisposition::CopyWholeBundle
        );
        let mut keepers = existing();
        keepers[0].media = false;
        assert_eq!(
            observations(&bundle(), &keepers, true)[0].disposition,
            ImportBundleDisposition::CopyWholeBundle
        );
    }
    #[test]
    fn ambiguous_candidate_or_reused_keeper_is_not_a_complete_group() {
        let mut keepers = existing();
        keepers.push(keeper(
            "jpg",
            "extra",
            "library",
            "existing/Renamed.JPG",
            true,
        ));
        assert_eq!(
            observations(&bundle(), &keepers, true)[0].disposition,
            ImportBundleDisposition::CopyWholeBundle
        );
        let mut keepers = existing();
        keepers[1].asset = keepers[0].asset.clone();
        assert_eq!(
            observations(&bundle(), &keepers, true)[0].disposition,
            ImportBundleDisposition::CopyWholeBundle
        );
    }
    #[test]
    fn incomplete_or_ambiguous_source_layout_requires_attention() {
        assert_eq!(
            observations(&bundle(), &existing(), false)[0].disposition,
            ImportBundleDisposition::NeedsAttention
        );
        let mut members = bundle();
        members[2].path = "source/arbitrary.xmp".into();
        assert_eq!(
            observations(&members, &existing(), true)[0].disposition,
            ImportBundleDisposition::NeedsAttention
        );
        let mut members = bundle();
        members[1].path = "source/Alternative.JPG".into();
        assert_eq!(
            observations(&members, &existing(), true)[0].disposition,
            ImportBundleDisposition::NeedsAttention
        );
    }
    #[test]
    fn transitive_shared_companions_cannot_split_connected_bundles() {
        let members = vec![
            member("a", "Photo.JPG", &["a"], true),
            member("b", "Photo.xmp", &["a", "b"], false),
            member("c", "Photo.RAW", &["b", "c"], true),
            member("d", "Photo.txt", &["c"], false),
            member("other", "Other.JPG", &["other"], true),
        ];
        let result = observations(&members, &[], true);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].file_ids, vec!["a", "b", "c", "d"]);
        assert_eq!(result[1].file_ids, vec!["other"]);
        assert!(
            result
                .iter()
                .all(|row| row.disposition == ImportBundleDisposition::CopyWholeBundle)
        );
    }
    #[cfg(windows)]
    #[test]
    fn invalid_utf16_is_never_grouped_through_lossy_names() {
        use std::os::windows::ffi::OsStringExt;
        let mut members = bundle();
        members[0].path = std::ffi::OsString::from_wide(&[0xd800, 46, 82, 65, 87]).into();
        assert_eq!(
            observations(&members, &existing(), true)[0].disposition,
            ImportBundleDisposition::NeedsAttention
        );
    }
    #[cfg(unix)]
    #[test]
    fn invalid_utf8_is_never_grouped_through_lossy_names() {
        use std::os::unix::ffi::OsStringExt;
        let mut members = bundle();
        members[0].path = std::ffi::OsString::from_vec(vec![255, 46, 82, 65, 87]).into();
        assert_eq!(
            observations(&members, &existing(), true)[0].disposition,
            ImportBundleDisposition::NeedsAttention
        );
    }
}
