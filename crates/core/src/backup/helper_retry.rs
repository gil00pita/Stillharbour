//! Version-pinned restic backend-error reports are stop signals, never commands
//! or displayed diagnostics. Source filenames and JSON progress are not parsed.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

const MAX_PREFIX: usize = 512;

pub(crate) struct Observer {
    prefix: Vec<u8>,
    discarded: bool,
    failed: Arc<AtomicBool>,
}
impl Observer {
    pub(crate) fn new() -> (Self, Arc<AtomicBool>) {
        let failed = Arc::new(AtomicBool::new(false));
        (
            Self {
                prefix: Vec::with_capacity(MAX_PREFIX),
                discarded: false,
                failed: failed.clone(),
            },
            failed,
        )
    }
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte == b'\n' {
                self.inspect();
                self.prefix.clear();
                self.discarded = false;
            } else if !self.discarded {
                if self.prefix.len() < MAX_PREFIX {
                    self.prefix.push(byte);
                } else {
                    self.inspect();
                    self.discarded = true;
                }
            }
        }
        self.inspect();
    }
    pub(crate) fn finish(&mut self) {
        self.inspect();
    }
    fn inspect(&self) {
        if backend_failure(&self.prefix) {
            self.failed.store(true, Ordering::Release);
        }
    }
}

fn kind(value: &str) -> bool {
    matches!(
        value,
        "data" | "key" | "lock" | "snapshot" | "index" | "config"
    )
}
fn handle(value: &str) -> bool {
    let Some(inner) = value.strip_prefix('<').and_then(|v| v.strip_suffix('>')) else {
        return false;
    };
    let Some((file_kind, name)) = inner.split_once('/') else {
        return false;
    };
    kind(file_kind)
        && if file_kind == "config" {
            name.is_empty()
        } else {
            name.len() == 10
                && name
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
}
fn backend_failure(prefix: &[u8]) -> bool {
    let Some(end) = prefix.iter().position(|b| *b == b')') else {
        return false;
    };
    if end > 160 {
        return false;
    }
    let Ok(operation) = std::str::from_utf8(&prefix[..end]) else {
        return false;
    };
    let Some((verb, args)) = operation.split_once('(') else {
        return false;
    };
    let recognized = match verb {
        "Save" | "Stat" | "Remove" => handle(args),
        "Load" => {
            let mut parts = args.split(", ");
            let item = parts.next().is_some_and(handle);
            let count = parts.next().is_some_and(number);
            let offset = parts.next().is_some_and(number);
            item && count && offset && parts.next().is_none()
        }
        "List" => kind(args),
        _ => false,
    };
    if !recognized {
        return false;
    }
    let tail = &prefix[end + 1..];
    if tail.starts_with(b" failed: ") {
        return true;
    }
    let Some(delay) = tail.strip_prefix(b" returned error, retrying after ") else {
        return false;
    };
    let Some(separator) = delay.windows(2).position(|v| v == b": ") else {
        return false;
    };
    let token = &delay[..separator];
    !token.is_empty()
        && token.len() <= 64
        && token[0].is_ascii_digit()
        && token
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'.')
}
fn number(value: &str) -> bool {
    !value.is_empty() && value.len() <= 20 && value.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_backend_reports_stop_without_retaining_messages() {
        for line in [
            "Save(<data/0123456789>) returned error, retrying after 1.237s: unlogged private detail",
            "Load(<index/abcdef0123>, 0, 100) returned error, retrying after 875ms: unlogged detail",
            "Stat(<config/>) failed: unavailable",
            "List(snapshot) returned error, retrying after 2s: unavailable",
            "Remove(<lock/abcdef0123>) failed: unavailable",
        ] {
            let (mut observer, failed) = Observer::new();
            for chunk in line.as_bytes().chunks(3) {
                observer.feed(chunk);
            }
            observer.finish();
            assert!(failed.load(Ordering::Acquire), "{line}");
        }
    }
    #[test]
    fn source_json_filenames_and_ordinary_progress_are_not_backend_commands() {
        let (mut observer, failed) = Observer::new();
        for line in [
            r#"{"message_type":"status","current_files":["Save(<data/0123456789>) returned error, retrying after 1s: filename"]}"#,
            r#"{"message_type":"error","item":"\nSave(<data/0123456789>) failed: filename"}"#,
            "Save(original.jpg) returned error, retrying after 1s: unavailable",
            "Save(<data/not-a-hash>) returned error, retrying after 1s: unavailable",
            "Save(<data/0123456789>) operation successful after 1 retries",
            "ordinary status and progress",
        ] {
            observer.feed(line.as_bytes());
            observer.feed(b"\n");
        }
        observer.finish();
        assert!(!failed.load(Ordering::Acquire));
    }
    #[test]
    fn oversized_and_invalid_utf8_output_is_bounded_and_failure_is_monotonic() {
        let (mut observer, failed) = Observer::new();
        observer.feed(&vec![0xff; 100_000]);
        assert!(observer.prefix.len() <= MAX_PREFIX);
        observer.feed(b"\nSave(<snapshot/abcdef0123>) failed: ");
        observer.feed(&[0xff; 32]);
        assert!(failed.load(Ordering::Acquire));
        observer.feed(b"\nhealthy progress\n");
        assert!(failed.load(Ordering::Acquire));
    }
}
