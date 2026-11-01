//! Read-only fingerprint filters and complete byte evidence for import review.
//! Hash equality never substitutes for direct comparison or grants mutation.
use super::{Object, failed, locations, source_native};
use crate::{
    domain::Result,
    jobs::paced_io::{CHUNK_BYTES, IoStep, PacedIo},
};
use std::{
    fs::File,
    io::{Seek, SeekFrom},
    sync::atomic::AtomicBool,
};

pub(super) const SAMPLE_POLICY: &str = "import-three-window-blake3-v1";
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub id: String,
    pub root_id: String,
    pub location: locations::Source,
    pub object: Object,
    pub revision: String,
    pub bytes: u64,
    pub display: String,
}
#[derive(Default)]
pub(super) struct Stats {
    pub bytes_read: u128,
}
pub(super) struct Witness {
    pub digest: String,
    pub same_object: bool,
}
impl Entry {
    pub(super) fn open(&self, root: &File) -> Result<File> {
        let file = self.location.open(root)?;
        self.validate(root, &file)?;
        Ok(file)
    }
    pub(super) fn validate(&self, root: &File, file: &File) -> Result<()> {
        if self.object.file == 0 {
            return Err(failed(
                "Stable native file identity is unavailable for comparison.",
            ));
        }
        if !self.object.matches(file)?
            || source_native::revision(file)? != self.revision
            || file.metadata()?.len() != self.bytes
        {
            return Err(failed(
                "The compared file changed identity, revision or length.",
            ));
        }
        let member = self.location.open(root)?;
        if !self.object.matches(&member)?
            || source_native::revision(&member)? != self.revision
            || member.metadata()?.len() != self.bytes
        {
            return Err(failed(
                "The compared file changed native directory membership.",
            ));
        }
        Ok(())
    }
}
pub(super) fn key(hash: blake3::Hash) -> String {
    hash.to_hex().to_string()
}
fn read(
    io: &PacedIo,
    file: &mut File,
    buffer: &mut [u8],
    cancel: &AtomicBool,
    stats: &mut Stats,
) -> Result<IoStep<usize>> {
    let mut used = 0;
    while used < buffer.len() {
        match io.read(file, &mut buffer[used..], cancel)? {
            IoStep::Canceled => return Ok(IoStep::Canceled),
            IoStep::Completed(0) => break,
            IoStep::Completed(bytes) => {
                stats.bytes_read += bytes as u128;
                used += bytes;
            }
        }
    }
    Ok(IoStep::Completed(used))
}
/// The mapper is backend-owned; tests force collisions without weakening the
/// separate byte comparator. Production always supplies the actual BLAKE3 key.
pub(super) fn sample(
    entry: &Entry,
    root: &File,
    io: &PacedIo,
    cancel: &AtomicBool,
    stats: &mut Stats,
    hash_key: fn(blake3::Hash) -> String,
) -> Result<IoStep<String>> {
    let mut file = entry.open(root)?;
    let window = entry.bytes.min(CHUNK_BYTES as u64);
    let offsets = std::collections::BTreeSet::from([
        0,
        entry.bytes.saturating_sub(window) / 2,
        entry.bytes.saturating_sub(window),
    ]);
    let mut hash = blake3::Hasher::new();
    hash.update(SAMPLE_POLICY.as_bytes());
    hash.update(&entry.bytes.to_le_bytes());
    let mut buffer = [0u8; CHUNK_BYTES];
    for offset in offsets {
        file.seek(SeekFrom::Start(offset))?;
        let count = if window == 0 { 1 } else { window as usize };
        let IoStep::Completed(read) = read(io, &mut file, &mut buffer[..count], cancel, stats)?
        else {
            return Ok(IoStep::Canceled);
        };
        if read as u64 != window {
            return Err(failed("File length changed during sampling."));
        }
        hash.update(&offset.to_le_bytes());
        hash.update(&window.to_le_bytes());
        hash.update(&buffer[..read]);
    }
    entry.validate(root, &file)?;
    Ok(IoStep::Completed(hash_key(hash.finalize())))
}
pub(super) fn full(
    entry: &Entry,
    root: &File,
    io: &PacedIo,
    cancel: &AtomicBool,
    stats: &mut Stats,
    hash_key: fn(blake3::Hash) -> String,
) -> Result<IoStep<String>> {
    let mut file = entry.open(root)?;
    let mut hash = blake3::Hasher::new();
    let mut total = 0u64;
    let mut buffer = [0u8; CHUNK_BYTES];
    loop {
        let count = entry
            .bytes
            .saturating_sub(total)
            .min(CHUNK_BYTES as u64)
            .max(1) as usize;
        let IoStep::Completed(read) = read(io, &mut file, &mut buffer[..count], cancel, stats)?
        else {
            return Ok(IoStep::Canceled);
        };
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| failed("Compared byte count overflow."))?;
        if total > entry.bytes {
            return Err(failed("A file grew while hashing."));
        }
        hash.update(&buffer[..read]);
    }
    entry.validate(root, &file)?;
    if total != entry.bytes {
        return Err(failed("A file shortened while hashing."));
    }
    Ok(IoStep::Completed(hash_key(hash.finalize())))
}
pub(super) fn equal(
    left: (&Entry, &File),
    right: (&Entry, &File),
    io: &PacedIo,
    cancel: &AtomicBool,
    stats: &mut Stats,
) -> Result<IoStep<Option<Witness>>> {
    let (a, a_root) = left;
    let (b, b_root) = right;
    let mut left = a.open(a_root)?;
    let mut right = b.open(b_root)?;
    if a.bytes != b.bytes {
        return Ok(IoStep::Completed(None));
    }
    let mut left_buffer = [0u8; CHUNK_BYTES];
    let mut right_buffer = [0u8; CHUNK_BYTES];
    let mut total = 0u64;
    let mut hash = blake3::Hasher::new();
    let matched = loop {
        let count = a.bytes.saturating_sub(total).min(CHUNK_BYTES as u64).max(1) as usize;
        let IoStep::Completed(x) = read(io, &mut left, &mut left_buffer[..count], cancel, stats)?
        else {
            return Ok(IoStep::Canceled);
        };
        let IoStep::Completed(y) = read(io, &mut right, &mut right_buffer[..count], cancel, stats)?
        else {
            return Ok(IoStep::Canceled);
        };
        if x != y || left_buffer[..x] != right_buffer[..y] {
            break false;
        }
        if x == 0 {
            break true;
        }
        total = total
            .checked_add(x as u64)
            .ok_or_else(|| failed("Compared byte count overflow."))?;
        if total > a.bytes {
            return Err(failed("A file grew during direct comparison."));
        }
        hash.update(&left_buffer[..x]);
    };
    a.validate(a_root, &left)?;
    b.validate(b_root, &right)?;
    if matched && total != a.bytes {
        return Err(failed("File length changed during direct comparison."));
    }
    Ok(IoStep::Completed(matched.then(|| Witness {
        digest: hash.finalize().to_hex().to_string(),
        same_object: a.object == b.object,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(root: &File, name: &str) -> Entry {
        let location = locations::Source::capture(root, std::path::Path::new(name)).unwrap();
        let file = location.open(root).unwrap();
        Entry {
            id: name.into(),
            root_id: "owned".into(),
            location,
            object: Object::of(&file).unwrap(),
            revision: source_native::revision(&file).unwrap(),
            bytes: file.metadata().unwrap().len(),
            display: name.into(),
        }
    }
    #[test]
    fn sampled_and_forced_full_hash_collisions_never_produce_a_byte_witness() {
        let run = tempfile::tempdir().unwrap();
        let bytes = (0..CHUNK_BYTES * 5)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let mut different = bytes.clone();
        different[CHUNK_BYTES + 11] ^= 1;
        std::fs::write(run.path().join("source.bin"), &bytes).unwrap();
        std::fs::write(run.path().join("renamed.bin"), &different).unwrap();
        let root = source_native::open_root(run.path(), None).unwrap();
        let source = entry(&root, "source.bin");
        let candidate = entry(&root, "renamed.bin");
        let io = PacedIo::new(1024 * 1024 * 1024).unwrap();
        let cancel = AtomicBool::new(false);
        let mut stats = Stats::default();
        assert_eq!(
            sample(&source, &root, &io, &cancel, &mut stats, key).unwrap(),
            sample(&candidate, &root, &io, &cancel, &mut stats, key).unwrap()
        );
        fn collision(_: blake3::Hash) -> String {
            "0".repeat(64)
        }
        assert_eq!(
            full(&source, &root, &io, &cancel, &mut stats, collision).unwrap(),
            full(&candidate, &root, &io, &cancel, &mut stats, collision).unwrap()
        );
        let IoStep::Completed(witness) = equal(
            (&source, &root),
            (&candidate, &root),
            &io,
            &cancel,
            &mut stats,
        )
        .unwrap() else {
            panic!("unexpected cancellation")
        };
        assert!(witness.is_none());
        assert_eq!(std::fs::read(run.path().join("source.bin")).unwrap(), bytes);
        assert_eq!(
            std::fs::read(run.path().join("renamed.bin")).unwrap(),
            different
        );
    }
    #[test]
    fn renamed_bytes_empty_files_and_hard_links_have_exact_distinct_evidence() {
        let run = tempfile::tempdir().unwrap();
        let bytes = vec![71; CHUNK_BYTES + 19];
        std::fs::write(run.path().join("source.bin"), &bytes).unwrap();
        std::fs::write(run.path().join("other-name.bin"), &bytes).unwrap();
        std::fs::hard_link(run.path().join("source.bin"), run.path().join("link.bin")).unwrap();
        for name in ["empty-a", "empty-b"] {
            std::fs::write(run.path().join(name), []).unwrap();
        }
        let root = source_native::open_root(run.path(), None).unwrap();
        let io = PacedIo::new(1024 * 1024 * 1024).unwrap();
        let cancel = AtomicBool::new(false);
        let mut stats = Stats::default();
        let source = entry(&root, "source.bin");
        for (name, same) in [("other-name.bin", false), ("link.bin", true)] {
            let candidate = entry(&root, name);
            let IoStep::Completed(Some(proof)) = equal(
                (&source, &root),
                (&candidate, &root),
                &io,
                &cancel,
                &mut stats,
            )
            .unwrap() else {
                panic!("missing complete comparison")
            };
            assert_eq!(proof.same_object, same);
            assert_eq!(proof.digest, key(blake3::hash(&bytes)));
        }
        let a = entry(&root, "empty-a");
        let b = entry(&root, "empty-b");
        assert!(matches!(
            equal((&a, &root), (&b, &root), &io, &cancel, &mut stats).unwrap(),
            IoStep::Completed(Some(_))
        ));
        assert!(stats.bytes_read >= bytes.len() as u128 * 4);
        assert_eq!(std::fs::read(run.path().join("source.bin")).unwrap(), bytes);
    }
    #[test]
    fn cancellation_and_changed_versions_never_return_a_match() {
        let run = tempfile::tempdir().unwrap();
        std::fs::write(run.path().join("a"), b"same").unwrap();
        std::fs::write(run.path().join("b"), b"same").unwrap();
        let root = source_native::open_root(run.path(), None).unwrap();
        let a = entry(&root, "a");
        let b = entry(&root, "b");
        let io = PacedIo::default();
        let mut stats = Stats::default();
        assert!(matches!(
            equal(
                (&a, &root),
                (&b, &root),
                &io,
                &AtomicBool::new(true),
                &mut stats
            )
            .unwrap(),
            IoStep::Canceled
        ));
        assert_eq!(stats.bytes_read, 0);
        let modified = std::fs::metadata(run.path().join("b"))
            .unwrap()
            .modified()
            .unwrap();
        std::fs::write(run.path().join("b"), b"edit").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(run.path().join("b"))
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        assert!(
            equal(
                (&a, &root),
                (&b, &root),
                &io,
                &AtomicBool::new(false),
                &mut stats
            )
            .is_err()
        );
        assert_eq!(std::fs::read(run.path().join("a")).unwrap(), b"same");
        assert_eq!(std::fs::read(run.path().join("b")).unwrap(), b"edit");
    }
}
