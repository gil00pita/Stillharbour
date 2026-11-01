//! All algorithms are filters. Only the complete direct comparator proves bytes.
use super::{changed, invalid, model::ContentAlgorithm};
use crate::{
    domain::Result,
    jobs::paced_io::{CHUNK_BYTES, IoStep, PacedIo},
};
use sha2::Digest;
use std::{
    io::{Read, Seek, SeekFrom},
    sync::atomic::AtomicBool,
};

pub(crate) const POLICY: &str = "inspection-content-v1";
pub(crate) const SAMPLE_POLICY: &str = "inspection-three-window-blake3-v1";
#[derive(Default)]
pub(crate) struct Stats {
    pub bytes_read: u128,
}

fn read_window(
    reader: &mut impl Read,
    buffer: &mut [u8],
    io: &PacedIo,
    cancel: &AtomicBool,
    stats: &mut Stats,
) -> Result<IoStep<usize>> {
    let mut used = 0;
    while used < buffer.len() {
        match io.read(reader, &mut buffer[used..], cancel)? {
            IoStep::Canceled => return Ok(IoStep::Canceled),
            IoStep::Completed(0) => break,
            IoStep::Completed(count) => {
                used += count;
                stats.bytes_read += count as u128;
            }
        }
    }
    Ok(IoStep::Completed(used))
}

pub(crate) fn sample(
    reader: &mut (impl Read + Seek),
    bytes: u64,
    io: &PacedIo,
    cancel: &AtomicBool,
    stats: &mut Stats,
) -> Result<IoStep<String>> {
    let window = bytes.min(CHUNK_BYTES as u64);
    let offsets = std::collections::BTreeSet::from([
        0,
        bytes.saturating_sub(window) / 2,
        bytes.saturating_sub(window),
    ]);
    let mut hash = blake3::Hasher::new();
    hash.update(SAMPLE_POLICY.as_bytes());
    hash.update(&bytes.to_le_bytes());
    let mut buffer = [0; CHUNK_BYTES];
    for offset in offsets {
        reader.seek(SeekFrom::Start(offset))?;
        let IoStep::Completed(count) =
            read_window(reader, &mut buffer[..window as usize], io, cancel, stats)?
        else {
            return Ok(IoStep::Canceled);
        };
        if count as u64 != window {
            return Err(changed("File length changed during sampling."));
        }
        hash.update(&offset.to_le_bytes());
        hash.update(&buffer[..count]);
    }
    Ok(IoStep::Completed(hash.finalize().to_hex().to_string()))
}

enum Hasher {
    Blake3(Box<blake3::Hasher>),
    Sha256(sha2::Sha256),
    Sha1(sha1::Sha1),
    Md5(md5::Md5),
}
impl Hasher {
    fn new(algorithm: ContentAlgorithm) -> Result<Self> {
        Ok(match algorithm {
            ContentAlgorithm::Blake3 => Self::Blake3(Box::default()),
            ContentAlgorithm::DirectBytes => {
                return Err(invalid(
                    "Direct-byte mode does not produce a full-file hash.",
                ));
            }
            ContentAlgorithm::Sha256 => Self::Sha256(sha2::Sha256::new()),
            ContentAlgorithm::Sha1 => Self::Sha1(sha1::Sha1::new()),
            ContentAlgorithm::Md5 => Self::Md5(md5::Md5::new()),
        })
    }
    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Blake3(hash) => {
                hash.update(bytes);
            }
            Self::Sha256(hash) => hash.update(bytes),
            Self::Sha1(hash) => hash.update(bytes),
            Self::Md5(hash) => hash.update(bytes),
        }
    }
    fn finish(self) -> String {
        match self {
            Self::Blake3(hash) => hash.finalize().to_hex().to_string(),
            Self::Sha256(hash) => hex(&hash.finalize()),
            Self::Sha1(hash) => hex(&hash.finalize()),
            Self::Md5(hash) => hex(&hash.finalize()),
        }
    }
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[(byte >> 4) as usize]),
                char::from(DIGITS[(byte & 15) as usize]),
            ]
        })
        .collect()
}

pub(crate) fn full(
    reader: &mut impl Read,
    bytes: u64,
    algorithm: ContentAlgorithm,
    io: &PacedIo,
    cancel: &AtomicBool,
    stats: &mut Stats,
) -> Result<IoStep<String>> {
    let mut hash = Hasher::new(algorithm)?;
    let mut total = 0u64;
    let mut buffer = [0; CHUNK_BYTES];
    loop {
        let count = bytes.saturating_sub(total).min(CHUNK_BYTES as u64).max(1) as usize;
        let IoStep::Completed(read) = read_window(reader, &mut buffer[..count], io, cancel, stats)?
        else {
            return Ok(IoStep::Canceled);
        };
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| changed("Content length overflow."))?;
        if total > bytes {
            return Err(changed("File grew during full hashing."));
        }
        hash.update(&buffer[..read]);
    }
    if total != bytes {
        return Err(changed("File shortened during full hashing."));
    }
    Ok(IoStep::Completed(hash.finish()))
}

pub(crate) fn equal(
    left: &mut impl Read,
    right: &mut impl Read,
    bytes: u64,
    io: &PacedIo,
    cancel: &AtomicBool,
    stats: &mut Stats,
) -> Result<IoStep<bool>> {
    let (mut a, mut b) = ([0; CHUNK_BYTES], [0; CHUNK_BYTES]);
    let mut total = 0u64;
    loop {
        let count = bytes.saturating_sub(total).min(CHUNK_BYTES as u64).max(1) as usize;
        let IoStep::Completed(x) = read_window(left, &mut a[..count], io, cancel, stats)? else {
            return Ok(IoStep::Canceled);
        };
        let IoStep::Completed(y) = read_window(right, &mut b[..count], io, cancel, stats)? else {
            return Ok(IoStep::Canceled);
        };
        if x != y || a[..x] != b[..y] {
            return Ok(IoStep::Completed(false));
        }
        if x == 0 {
            return if total == bytes {
                Ok(IoStep::Completed(true))
            } else {
                Err(changed("Files shortened during byte comparison."))
            };
        }
        total = total
            .checked_add(x as u64)
            .ok_or_else(|| changed("Comparison length overflow."))?;
        if total > bytes {
            return Err(changed("Files grew during byte comparison."));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn completed<T>(step: IoStep<T>) -> T {
        match step {
            IoStep::Completed(value) => value,
            IoStep::Canceled => panic!("Unexpected cancellation"),
        }
    }
    #[test]
    fn compatibility_hashes_match_standard_vectors_and_do_not_replace_byte_equality() {
        let io = PacedIo::new(1024 * 1024 * 1024).unwrap();
        let cancel = AtomicBool::new(false);
        for (algorithm, expected) in [
            (
                ContentAlgorithm::Sha256,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                ContentAlgorithm::Sha1,
                "a9993e364706816aba3e25717850c26c9cd0d89d",
            ),
            (ContentAlgorithm::Md5, "900150983cd24fb0d6963f7d28e17f72"),
        ] {
            assert_eq!(
                completed(
                    full(
                        &mut Cursor::new(b"abc"),
                        3,
                        algorithm,
                        &io,
                        &cancel,
                        &mut Stats::default()
                    )
                    .unwrap()
                ),
                expected
            );
        }
        assert!(!completed(
            equal(
                &mut Cursor::new(b"abc"),
                &mut Cursor::new(b"abd"),
                3,
                &io,
                &cancel,
                &mut Stats::default()
            )
            .unwrap()
        ));
    }
    #[test]
    fn identical_size_and_samples_do_not_prove_full_content_equality() {
        let io = PacedIo::new(1024 * 1024 * 1024).unwrap();
        let cancel = AtomicBool::new(false);
        let a = vec![7; 5 * CHUNK_BYTES];
        let mut b = a.clone();
        b[CHUNK_BYTES + 7] ^= 1;
        let size = a.len() as u64;
        assert_eq!(
            completed(
                sample(
                    &mut Cursor::new(&a),
                    size,
                    &io,
                    &cancel,
                    &mut Stats::default()
                )
                .unwrap()
            ),
            completed(
                sample(
                    &mut Cursor::new(&b),
                    size,
                    &io,
                    &cancel,
                    &mut Stats::default()
                )
                .unwrap()
            )
        );
        for algorithm in [
            ContentAlgorithm::Blake3,
            ContentAlgorithm::Sha256,
            ContentAlgorithm::Sha1,
            ContentAlgorithm::Md5,
        ] {
            assert_ne!(
                completed(
                    full(
                        &mut Cursor::new(&a),
                        size,
                        algorithm,
                        &io,
                        &cancel,
                        &mut Stats::default()
                    )
                    .unwrap()
                ),
                completed(
                    full(
                        &mut Cursor::new(&b),
                        size,
                        algorithm,
                        &io,
                        &cancel,
                        &mut Stats::default()
                    )
                    .unwrap()
                )
            );
        }
        assert!(!completed(
            equal(
                &mut Cursor::new(a),
                &mut Cursor::new(b),
                size,
                &io,
                &cancel,
                &mut Stats::default()
            )
            .unwrap()
        ));
    }
    #[test]
    fn content_reads_reject_length_changes_and_obey_cancellation() {
        let io = PacedIo::new(1024 * 1024 * 1024).unwrap();
        let cancel = AtomicBool::new(false);
        for bytes in [2, 4] {
            assert!(
                full(
                    &mut Cursor::new(b"abc"),
                    bytes,
                    ContentAlgorithm::Blake3,
                    &io,
                    &cancel,
                    &mut Stats::default()
                )
                .is_err()
            );
        }
        let cancel = AtomicBool::new(true);
        assert!(matches!(
            full(
                &mut Cursor::new(b"abc"),
                3,
                ContentAlgorithm::Blake3,
                &io,
                &cancel,
                &mut Stats::default()
            )
            .unwrap(),
            IoStep::Canceled
        ));
    }
}
