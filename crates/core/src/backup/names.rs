//! Conservative review warnings, never native name-equivalence authorization.
use super::comparison::ComparisonEntryKind as Kind;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use unicode_normalization::UnicodeNormalization;

fn native_component(name: &str) -> bool {
    #[cfg(windows)]
    {
        crate::organization::portable_filename(name)
    }
    #[cfg(not(windows))]
    {
        !name.is_empty()
            && !matches!(name, "." | "..")
            && name.len() <= 255
            && !name.contains(['\0', '/'])
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum NameBlock {
    Alias,
    TypeConflict,
    Unsupported,
    NativeName,
}
impl NameBlock {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Alias => {
                "Potential case/Unicode name alias. Keep every entry; native destination name review is required before any copy. This comparison key is not filesystem equivalence evidence."
            }
            Self::TypeConflict => {
                "A file or unsupported object conflicts with a directory or descendant path. Retain both trees; no copy or replacement is authorized."
            }
            Self::Unsupported => {
                "This link, special object or placeholder was not read or followed. Its contents and fidelity are unverified; retain it."
            }
            Self::NativeName => {
                "This native name is outside the supported native destination naming policy. Native bytes are retained; display text cannot authorize a copy."
            }
        }
    }
}
pub(crate) fn blocks(
    source: &BTreeMap<PathBuf, Kind>,
    destination: &BTreeMap<PathBuf, Kind>,
) -> BTreeMap<PathBuf, NameBlock> {
    let paths: BTreeSet<_> = source.keys().chain(destination.keys()).collect();
    let mut prefixes = BTreeMap::new();
    let mut aliases = BTreeMap::<Vec<String>, BTreeSet<&PathBuf>>::new();
    for path in &paths {
        if path
            .components()
            .any(|c| c.as_os_str().to_str().is_none_or(|s| !native_component(s)))
        {
            prefixes.insert((*path).clone(), NameBlock::NativeName);
        }
        let key = path
            .components()
            .map(|part| {
                part.as_os_str().to_str().map(|text| {
                    text.nfc()
                        .collect::<String>()
                        .to_lowercase()
                        .nfc()
                        .collect::<String>()
                })
            })
            .collect::<Option<Vec<_>>>();
        if let Some(key) = key {
            aliases.entry(key).or_default().insert(*path);
        } else {
            prefixes.insert((*path).clone(), NameBlock::NativeName);
        }
        if source.get(*path) == Some(&Kind::Unsupported)
            || destination.get(*path) == Some(&Kind::Unsupported)
        {
            prefixes.insert((*path).clone(), NameBlock::Unsupported);
        }
        if source
            .get(*path)
            .zip(destination.get(*path))
            .is_some_and(|(a, b)| a != b)
        {
            prefixes.insert((*path).clone(), NameBlock::TypeConflict);
        }
        for ancestor in path
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
        {
            if source.get(ancestor).is_some_and(|k| *k != Kind::Directory)
                || destination
                    .get(ancestor)
                    .is_some_and(|k| *k != Kind::Directory)
            {
                prefixes.insert(ancestor.to_owned(), NameBlock::TypeConflict);
            }
        }
    }
    for group in aliases.values().filter(|group| group.len() > 1) {
        for path in group {
            prefixes.entry((*path).clone()).or_insert(NameBlock::Alias);
        }
    }
    paths
        .into_iter()
        .filter_map(|path| {
            path.ancestors()
                .find_map(|ancestor| prefixes.get(ancestor).map(|reason| (path.clone(), *reason)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entries(items: &[(&str, Kind)]) -> BTreeMap<PathBuf, Kind> {
        items.iter().map(|(p, k)| (PathBuf::from(p), *k)).collect()
    }
    #[test]
    fn aliases_and_type_conflicts_block_every_native_path_without_collapsing_them() {
        let a = entries(&[
            ("Album", Kind::Directory),
            ("Album/new.mp4", Kind::File),
            ("é.bin", Kind::File),
            ("parent", Kind::File),
            ("neighbor", Kind::File),
        ]);
        let b = entries(&[
            ("album", Kind::Directory),
            ("album/old.xmp", Kind::File),
            ("e\u{301}.bin", Kind::File),
            ("parent", Kind::Directory),
            ("parent/child", Kind::File),
        ]);
        let blocked = blocks(&a, &b);
        for path in [
            "Album",
            "Album/new.mp4",
            "album",
            "album/old.xmp",
            "é.bin",
            "e\u{301}.bin",
            "parent",
            "parent/child",
        ] {
            assert!(blocked.contains_key(&PathBuf::from(path)), "{path}");
        }
        assert!(!blocked.contains_key(&PathBuf::from("neighbor")));
        assert_eq!(a.len(), 5);
        assert_eq!(b.len(), 5);
    }
    #[test]
    fn exact_names_remain_pairs_and_unsupported_ancestors_block_descendants() {
        let a = entries(&[("same", Kind::File), ("link", Kind::Unsupported)]);
        let b = entries(&[
            ("same", Kind::File),
            ("link", Kind::Directory),
            ("link/child", Kind::File),
        ]);
        let blocked = blocks(&a, &b);
        assert!(!blocked.contains_key(&PathBuf::from("same")));
        assert!(blocked.contains_key(&PathBuf::from("link/child")));
    }
}
