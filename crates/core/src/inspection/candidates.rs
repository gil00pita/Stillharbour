//! Bounded candidate generation over a checked-reader port. No mutation port.
use super::{
    hashing::{self, Stats},
    invalid,
    model::*,
    rules::{self, Properties},
};
use crate::{
    domain::{AppError, ErrorCode, Result},
    jobs::paced_io::{IoStep, PacedIo},
};
use std::{
    collections::BTreeMap,
    io::{Read, Seek},
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) const MAX_ITEMS: usize = 100_000;
const MAX_PAIRS: u64 = 1_000_000;
pub(crate) const MAX_GROUPS: usize = 4096;
pub(crate) struct Item {
    pub id: String,
    pub object: Option<String>,
    pub links: u64,
    pub available: bool,
    pub properties: Properties,
}
pub(crate) trait Sources {
    type Reader: Read + Seek;
    fn open(&self, index: usize) -> Result<Self::Reader>;
    /// Recheck identity/revision/length and native membership, including roots.
    fn validate(&self, index: usize, reader: &Self::Reader) -> Result<()>;
}
pub(crate) struct Fingerprint {
    pub index: usize,
    pub algorithm: ContentAlgorithm,
    pub policy: String,
    pub digest: String,
}
#[derive(Default)]
pub(crate) struct Report {
    pub fingerprints: Vec<Fingerprint>,
    pub groups: Vec<Vec<String>>,
    pub aliases: Vec<Vec<String>>,
    pub issues: BTreeMap<String, AppError>,
    pub unknown_pairs: u64,
    pub compared_pairs: u64,
    pub limit_reached: bool,
    pub stats: Stats,
}
fn checked<S: Sources>(sources: &S, index: usize) -> Result<S::Reader> {
    let reader = sources.open(index)?;
    sources.validate(index, &reader)?;
    Ok(reader)
}
fn bytes_equal<S: Sources>(
    sources: &S,
    a: usize,
    b: usize,
    bytes: u64,
    io: &PacedIo,
    cancel: &AtomicBool,
    stats: &mut Stats,
) -> Result<IoStep<bool>> {
    let (mut left, mut right) = (checked(sources, a)?, checked(sources, b)?);
    let result = hashing::equal(&mut left, &mut right, bytes, io, cancel, stats)?;
    sources.validate(a, &left)?;
    sources.validate(b, &right)?;
    Ok(result)
}

pub(crate) fn scan<S: Sources>(
    sources: &S,
    items: &[Item],
    options: &DuplicateOptions,
    io: &PacedIo,
    cancel: &AtomicBool,
) -> Result<IoStep<Report>> {
    scan_keys(sources, items, options, io, cancel, |digest| digest)
}
// Only backend tests replace candidate keys. Direct comparison is unchanged.
fn scan_keys<S: Sources>(
    sources: &S,
    items: &[Item],
    options: &DuplicateOptions,
    io: &PacedIo,
    cancel: &AtomicBool,
    key: fn(String) -> String,
) -> Result<IoStep<Report>> {
    if items.len() > MAX_ITEMS {
        return Err(invalid(
            "Choose at most 100,000 files in one duplicate scope.",
        ));
    }
    if options.mode == MatchMode::NormalizedPixels {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Normalized pixels require a declared, fully supported decoder policy.",
        ));
    }
    if options.mode == MatchMode::Properties {
        options.rules.validate()?;
    }
    let mut report = Report::default();
    let mut objects: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Ok(IoStep::Canceled);
        }
        if !item.available {
            report.issues.entry(item.id.clone()).or_insert_with(|| {
                AppError::new(
                    ErrorCode::Io,
                    "The candidate root is offline or the file is unavailable.",
                )
            });
            continue;
        }
        if options.exclude_hardlinked_objects && item.links > 1 {
            continue;
        }
        match checked(sources, index) {
            Ok(_) => objects
                .entry(
                    item.object
                        .clone()
                        .unwrap_or_else(|| format!("unidentified:{}", item.id)),
                )
                .or_default()
                .push(index),
            Err(error) => {
                report.issues.entry(item.id.clone()).or_insert(error);
            }
        }
    }
    for indices in objects.values().filter(|indices| indices.len() > 1) {
        if report.aliases.len() >= MAX_GROUPS {
            report.limit_reached = true;
            break;
        }
        report.aliases.push(
            indices
                .iter()
                .map(|&index| items[index].id.clone())
                .collect(),
        );
    }
    let representatives = objects
        .values()
        .map(|indices| indices[0])
        .collect::<Vec<_>>();
    if options.mode == MatchMode::Properties {
        'pairs: for (position, &a) in representatives.iter().enumerate() {
            for &b in &representatives[position + 1..] {
                if cancel.load(Ordering::Acquire) {
                    return Ok(IoStep::Canceled);
                }
                if report.compared_pairs >= MAX_PAIRS || report.groups.len() >= MAX_GROUPS {
                    report.limit_reached = true;
                    break 'pairs;
                }
                report.compared_pairs += 1;
                let content = if options.rules.content
                    && items[a].properties.bytes == items[b].properties.bytes
                {
                    match bytes_equal(
                        sources,
                        a,
                        b,
                        items[a].properties.bytes,
                        io,
                        cancel,
                        &mut report.stats,
                    ) {
                        Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
                        Ok(IoStep::Completed(true)) => Truth::Match,
                        Ok(IoStep::Completed(false)) => Truth::Different,
                        Err(error) => {
                            report.issues.entry(items[b].id.clone()).or_insert(error);
                            Truth::Unknown
                        }
                    }
                } else if options.rules.content {
                    Truth::Different
                } else {
                    Truth::Unknown
                };
                let evidence = rules::compare(
                    &items[a].properties,
                    &items[b].properties,
                    &options.rules,
                    content,
                )?;
                if evidence.result == Truth::Unknown {
                    report.unknown_pairs += 1;
                }
                if evidence.result == Truth::Match {
                    report
                        .groups
                        .push(vec![items[a].id.clone(), items[b].id.clone()]);
                }
            }
        }
        return Ok(IoStep::Completed(report));
    }
    let mut sizes: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for index in representatives {
        sizes
            .entry(items[index].properties.bytes)
            .or_default()
            .push(index);
    }
    'sizes: for (bytes, indices) in sizes.into_iter().filter(|(_, indices)| indices.len() > 1) {
        let mut sampled: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for index in indices {
            let result = (|| {
                let mut reader = checked(sources, index)?;
                let result = hashing::sample(&mut reader, bytes, io, cancel, &mut report.stats)?;
                sources.validate(index, &reader)?;
                Ok(result)
            })();
            match result {
                Ok(IoStep::Completed(digest)) => {
                    report.fingerprints.push(Fingerprint {
                        index,
                        algorithm: ContentAlgorithm::Blake3,
                        policy: hashing::SAMPLE_POLICY.into(),
                        digest: digest.clone(),
                    });
                    sampled.entry(key(digest)).or_default().push(index)
                }
                Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
                Err(error) => {
                    report
                        .issues
                        .entry(items[index].id.clone())
                        .or_insert(error);
                }
            }
        }
        for indices in sampled.into_values().filter(|indices| indices.len() > 1) {
            let mut hashed: BTreeMap<String, Vec<usize>> = BTreeMap::new();
            for index in indices {
                let result = (|| {
                    if options.algorithm == ContentAlgorithm::DirectBytes {
                        return Ok(IoStep::Completed("direct-bytes".into()));
                    }
                    let mut reader = checked(sources, index)?;
                    let result = hashing::full(
                        &mut reader,
                        bytes,
                        options.algorithm,
                        io,
                        cancel,
                        &mut report.stats,
                    )?;
                    sources.validate(index, &reader)?;
                    Ok(result)
                })();
                match result {
                    Ok(IoStep::Completed(digest)) => {
                        if options.algorithm != ContentAlgorithm::DirectBytes {
                            report.fingerprints.push(Fingerprint {
                                index,
                                algorithm: options.algorithm,
                                policy: hashing::POLICY.into(),
                                digest: digest.clone(),
                            });
                        }
                        hashed.entry(key(digest)).or_default().push(index)
                    }
                    Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
                    Err(error) => {
                        report
                            .issues
                            .entry(items[index].id.clone())
                            .or_insert(error);
                    }
                }
            }
            for indices in hashed.into_values().filter(|indices| indices.len() > 1) {
                let mut verified: Vec<Vec<usize>> = Vec::new();
                for index in indices {
                    let mut matched = false;
                    for group in &mut verified {
                        if report.compared_pairs >= MAX_PAIRS {
                            report.limit_reached = true;
                            break 'sizes;
                        }
                        report.compared_pairs += 1;
                        match bytes_equal(
                            sources,
                            group[0],
                            index,
                            bytes,
                            io,
                            cancel,
                            &mut report.stats,
                        ) {
                            Ok(IoStep::Canceled) => return Ok(IoStep::Canceled),
                            Ok(IoStep::Completed(true)) => {
                                group.push(index);
                                matched = true;
                                break;
                            }
                            Ok(IoStep::Completed(false)) => {}
                            Err(error) => {
                                report
                                    .issues
                                    .entry(items[index].id.clone())
                                    .or_insert(error);
                                matched = true;
                                break;
                            }
                        }
                    }
                    if !matched {
                        verified.push(vec![index]);
                    }
                }
                for group in verified.into_iter().filter(|group| group.len() > 1) {
                    if report.groups.len() >= MAX_GROUPS {
                        report.limit_reached = true;
                        break 'sizes;
                    }
                    let mut ids = Vec::new();
                    for index in group {
                        let aliases = objects
                            .get(
                                &items[index]
                                    .object
                                    .clone()
                                    .unwrap_or_else(|| format!("unidentified:{}", items[index].id)),
                            )
                            .ok_or_else(|| {
                                invalid("Candidate identity projection is incomplete.")
                            })?;
                        ids.extend(aliases.iter().map(|&alias| items[alias].id.clone()));
                    }
                    report.groups.push(ids);
                }
            }
        }
    }
    Ok(IoStep::Completed(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    struct Memory(Vec<Vec<u8>>);
    impl Sources for Memory {
        type Reader = Cursor<Vec<u8>>;
        fn open(&self, index: usize) -> Result<Self::Reader> {
            Ok(Cursor::new(self.0[index].clone()))
        }
        fn validate(&self, _: usize, _: &Self::Reader) -> Result<()> {
            Ok(())
        }
    }
    fn options(algorithm: ContentAlgorithm) -> DuplicateOptions {
        DuplicateOptions {
            mode: MatchMode::ExactContent,
            algorithm,
            exclude_hardlinked_objects: false,
            rules: PropertyRules {
                logic: RuleLogic::All,
                filename: false,
                extension: false,
                size: false,
                modified: false,
                created: false,
                readonly: false,
                content: false,
                tolerance_millis: 0,
                name: NamePolicy::default(),
            },
        }
    }
    fn items() -> Vec<Item> {
        (0..3)
            .map(|index| Item {
                id: index.to_string(),
                object: Some(index.to_string()),
                links: 1,
                available: true,
                properties: Properties {
                    filename: Some(format!("renamed-{index}")),
                    extension: Some(if index == 0 { "jpg" } else { "raw" }.into()),
                    bytes: 3,
                    modified_millis: None,
                    created_millis: None,
                    readonly: None,
                },
            })
            .collect()
    }
    #[test]
    fn all_hash_modes_reject_mock_collisions_and_find_renamed_cross_extension_copies() {
        let sources = Memory(vec![b"abc".to_vec(), b"abc".to_vec(), b"abd".to_vec()]);
        let io = PacedIo::new(1024 * 1024 * 1024).unwrap();
        for algorithm in [
            ContentAlgorithm::Blake3,
            ContentAlgorithm::Sha256,
            ContentAlgorithm::Sha1,
            ContentAlgorithm::Md5,
            ContentAlgorithm::DirectBytes,
        ] {
            let IoStep::Completed(report) = scan_keys(
                &sources,
                &items(),
                &options(algorithm),
                &io,
                &AtomicBool::new(false),
                |_| "forced-collision".into(),
            )
            .unwrap() else {
                panic!("Canceled")
            };
            assert_eq!(report.groups, vec![vec!["0".to_string(), "1".to_string()]]);
            assert!(report.issues.is_empty());
            assert_eq!(report.compared_pairs, 2);
        }
    }
    #[test]
    fn aliases_are_separate_from_independent_duplicates_and_offline_is_not_empty() {
        let sources = Memory(vec![b"abc".to_vec(), b"abc".to_vec(), b"abc".to_vec()]);
        let mut records = items();
        records[1].object = records[0].object.clone();
        records[0].links = 2;
        records[1].links = 2;
        records[2].available = false;
        let io = PacedIo::new(1024 * 1024 * 1024).unwrap();
        let IoStep::Completed(report) = scan(
            &sources,
            &records,
            &options(ContentAlgorithm::Blake3),
            &io,
            &AtomicBool::new(false),
        )
        .unwrap() else {
            panic!("Canceled")
        };
        assert!(report.groups.is_empty());
        assert_eq!(report.aliases.len(), 1);
        assert_eq!(report.issues.len(), 1);
        records[2].available = true;
        let IoStep::Completed(report) = scan(
            &sources,
            &records,
            &options(ContentAlgorithm::Blake3),
            &io,
            &AtomicBool::new(false),
        )
        .unwrap() else {
            panic!("Canceled")
        };
        assert_eq!(report.groups[0].len(), 3);
        let mut excluded = options(ContentAlgorithm::Blake3);
        excluded.exclude_hardlinked_objects = true;
        let IoStep::Completed(report) =
            scan(&sources, &records, &excluded, &io, &AtomicBool::new(false)).unwrap()
        else {
            panic!("Canceled")
        };
        assert!(report.groups.is_empty() && report.aliases.is_empty());
    }
}
