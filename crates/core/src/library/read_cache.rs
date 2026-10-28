//! Connection-owned derived reads. Never caches selection membership or safety evidence.
use super::model::DateBucket;
use std::collections::VecDeque;

const CAPACITY: usize = 16;
const BYTE_BUDGET: usize = 512 * 1024;

#[derive(Clone)]
pub(crate) enum Summary {
    Count(u32),
    Buckets(Vec<DateBucket>),
}

#[derive(Default)]
pub(crate) struct ReadCache {
    epoch: (u64, i64),
    bytes: usize,
    entries: VecDeque<(String, Summary, usize)>,
}

impl ReadCache {
    #[cfg(feature = "fixture-probes")]
    pub(crate) fn reset_for_probe(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    pub(crate) fn get(
        &mut self,
        db: &rusqlite::Connection,
        key: &str,
    ) -> rusqlite::Result<Option<Summary>> {
        // total_changes observes our actor's writes, including writes with no
        // library revision bump. data_version also rejects external observations
        // (supported writers still must honor the application catalog lock).
        let epoch = (
            db.total_changes(),
            db.pragma_query_value(None, "data_version", |r| r.get(0))?,
        );
        Ok(self.get_at(epoch, key))
    }

    fn get_at(&mut self, epoch: (u64, i64), key: &str) -> Option<Summary> {
        if epoch != self.epoch {
            self.entries.clear();
            self.bytes = 0;
            self.epoch = epoch;
        }
        let index = self.entries.iter().position(|entry| entry.0 == key)?;
        let entry = self.entries.remove(index)?;
        let value = entry.1.clone();
        self.entries.push_back(entry);
        Some(value)
    }

    pub(crate) fn insert(&mut self, key: String, value: Summary) {
        // Budget encoded data as well as entry count. A pathological date set is
        // returned normally but never retained in this cache.
        let bytes = key.len()
            + match &value {
                Summary::Count(_) => 4,
                Summary::Buckets(buckets) => match serde_json::to_vec(buckets) {
                    Ok(encoded) => encoded.len(),
                    Err(_) => return,
                },
            };
        if bytes > BYTE_BUDGET {
            return;
        }
        while self.entries.len() >= CAPACITY || self.bytes + bytes > BYTE_BUDGET {
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.2;
            }
        }
        self.bytes += bytes;
        self.entries.push_back((key, value, bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_lru_invalidates_on_any_connection_write() {
        let mut cache = ReadCache::default();
        for n in 0..20 {
            cache.insert(n.to_string(), Summary::Count(n));
        }
        assert_eq!(cache.entries.len(), CAPACITY);
        assert!(cache.get_at((0, 0), "0").is_none());
        assert!(matches!(cache.get_at((0, 0), "4"), Some(Summary::Count(4))));
        cache.insert("new".into(), Summary::Count(21));
        assert!(cache.get_at((0, 0), "5").is_none());
        assert!(cache.get_at((1, 0), "4").is_none());
        assert!(cache.entries.is_empty());
        assert_eq!(cache.bytes, 0);
        cache.insert(
            "huge".into(),
            Summary::Buckets(vec![DateBucket {
                year: "x".repeat(BYTE_BUDGET),
                count: 1,
                months: vec![],
            }]),
        );
        assert!(cache.entries.is_empty());
        for key in ["first", "second"] {
            cache.insert(
                key.into(),
                Summary::Buckets(vec![DateBucket {
                    year: "x".repeat(BYTE_BUDGET / 2),
                    count: 1,
                    months: vec![],
                }]),
            );
        }
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.bytes <= BYTE_BUDGET);
        assert!(cache.get_at((1, 0), "first").is_none());
    }
}
