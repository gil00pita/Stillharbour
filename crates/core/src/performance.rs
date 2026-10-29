//! Opt-in diagnostic numbers only. No identifiers, paths, metadata or image content.
//! Production builds compile these calls to no-ops; fixture builds remain off until enabled.
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    CatalogOpen,
    CatalogCall,
    CatalogQueueWait,
    CatalogWork,
    PageQuery,
    DateBuckets,
    DateSeek,
    SelectionFreeze,
    ThumbnailBatch,
    ThumbnailQueueWait,
    ThumbnailBuild,
    ThumbnailWorker,
    ResourceWaitInteractive,
    ResourceWaitBackground,
    WorkerRss,
    Decode,
    Resize,
    Encode,
    Read,
    Write,
    SqlitePageMiss,
    SqlitePageWrite,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    Catalog,
    Source,
    Cache,
    Worker,
}
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub metric: Metric,
    pub endpoint: Endpoint,
    pub elapsed_us: u64,
    pub bytes: u64,
    pub volume_alias: u16,
}
#[derive(Default, Serialize)]
pub struct Snapshot {
    pub events: Vec<Event>,
    pub dropped: u64,
    pub capacity: usize,
    pub volume_count: usize,
    #[cfg(feature = "fixture-probes")]
    #[serde(skip)]
    volumes: std::collections::BTreeMap<u64, u16>,
}

#[cfg(feature = "fixture-probes")]
mod recorder {
    use super::*;
    use std::sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    };
    static ENABLED: AtomicBool = AtomicBool::new(false);
    static LOST: AtomicU64 = AtomicU64::new(0);
    static TRACE: OnceLock<Mutex<Snapshot>> = OnceLock::new();
    pub fn enabled() -> bool {
        ENABLED.load(Ordering::Relaxed)
    }
    pub fn enable() {
        let trace = TRACE.get_or_init(|| Mutex::new(Snapshot::default()));
        if let Ok(mut trace) = trace.lock() {
            *trace = Snapshot {
                events: Vec::with_capacity(4096),
                dropped: 0,
                capacity: 4096,
                ..Snapshot::default()
            };
            LOST.store(0, Ordering::Relaxed);
            ENABLED.store(true, Ordering::Relaxed);
        }
    }
    pub fn record(event: Event) {
        if !enabled() {
            return;
        }
        if let Some(trace) = TRACE.get()
            && let Ok(mut trace) = trace.try_lock()
        {
            if trace.events.len() < trace.capacity {
                trace.events.push(event);
            } else {
                trace.dropped = trace.dropped.saturating_add(1);
            }
        } else {
            LOST.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub fn snapshot() -> Snapshot {
        if let Some(trace) = TRACE.get()
            && let Ok(trace) = trace.lock()
        {
            Snapshot {
                events: trace.events.clone(),
                dropped: trace.dropped.saturating_add(LOST.load(Ordering::Relaxed)),
                capacity: trace.capacity,
                volume_count: trace.volume_count,
                ..Snapshot::default()
            }
        } else {
            Snapshot::default()
        }
    }
    pub fn volume_alias(volume: u64) -> u16 {
        if !enabled() {
            return 0;
        }
        if let Some(trace) = TRACE.get()
            && let Ok(mut trace) = trace.try_lock()
        {
            if let Some(alias) = trace.volumes.get(&volume) {
                return *alias;
            }
            if trace.volumes.len() >= 16 {
                return 0;
            }
            let alias = trace.volumes.len() as u16 + 1;
            trace.volumes.insert(volume, alias);
            trace.volume_count = trace.volumes.len();
            alias
        } else {
            0
        }
    }
    pub fn disable() {
        ENABLED.store(false, Ordering::Relaxed);
    }
}
#[inline]
pub fn enabled() -> bool {
    #[cfg(feature = "fixture-probes")]
    {
        recorder::enabled()
    }
    #[cfg(not(feature = "fixture-probes"))]
    {
        false
    }
}
pub fn enable() {
    #[cfg(feature = "fixture-probes")]
    recorder::enable();
}
pub fn disable() {
    #[cfg(feature = "fixture-probes")]
    recorder::disable();
}
pub fn snapshot() -> Snapshot {
    #[cfg(feature = "fixture-probes")]
    {
        recorder::snapshot()
    }
    #[cfg(not(feature = "fixture-probes"))]
    {
        Snapshot::default()
    }
}
#[inline]
pub fn clock() -> Option<Instant> {
    enabled().then(Instant::now)
}
#[inline]
pub fn record(metric: Metric, endpoint: Endpoint, elapsed_us: u64, bytes: u64) {
    record_at(metric, endpoint, 0, elapsed_us, bytes);
}
#[inline]
pub fn record_at(
    metric: Metric,
    endpoint: Endpoint,
    volume_alias: u16,
    elapsed_us: u64,
    bytes: u64,
) {
    #[cfg(feature = "fixture-probes")]
    recorder::record(Event {
        metric,
        endpoint,
        elapsed_us,
        bytes,
        volume_alias: if volume_alias <= 16 { volume_alias } else { 0 },
    });
    #[cfg(not(feature = "fixture-probes"))]
    let _ = (metric, endpoint, volume_alias, elapsed_us, bytes);
}
pub fn file_volume(file: &std::fs::File) -> u16 {
    if !enabled() {
        return 0;
    }
    #[cfg(all(feature = "fixture-probes", unix))]
    {
        use std::os::unix::fs::MetadataExt;
        file.metadata()
            .ok()
            .map(|metadata| recorder::volume_alias(metadata.dev()))
            .unwrap_or(0)
    }
    #[cfg(not(all(feature = "fixture-probes", unix)))]
    {
        let _ = file;
        0
    }
}
#[cfg(feature = "fixture-probes")]
pub(crate) fn path_volume(path: &std::path::Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path.parent()?)
            .ok()
            .map(|metadata| metadata.dev())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}
#[cfg(feature = "fixture-probes")]
pub(crate) fn volume_alias(volume: Option<u64>) -> u16 {
    volume.map(recorder::volume_alias).unwrap_or(0)
}

pub fn since(metric: Metric, endpoint: Endpoint, start: Option<Instant>, bytes: u64) {
    if let Some(start) = start {
        record(
            metric,
            endpoint,
            start.elapsed().as_micros().min(u64::MAX as u128) as u64,
            bytes,
        );
    }
}
pub struct Span {
    metric: Metric,
    endpoint: Endpoint,
    start: Option<Instant>,
}
impl Span {
    #[inline]
    pub fn new(metric: Metric, endpoint: Endpoint) -> Self {
        Self {
            metric,
            endpoint,
            start: clock(),
        }
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        since(self.metric, self.endpoint, self.start, 0);
    }
}

#[cfg(all(test, feature = "fixture-probes"))]
mod tests {
    use super::*;
    #[test]
    fn opt_in_trace_is_bounded_numeric_and_stops() {
        enable();
        let private_volume = 9182736450918273645u64;
        let alias = volume_alias(Some(private_volume));
        assert!((1..=16).contains(&alias));
        for device in 0..100 {
            volume_alias(Some(device));
        }
        assert_eq!(snapshot().volume_count, 16);
        assert!(
            !serde_json::to_string(&snapshot())
                .unwrap()
                .contains(&private_volume.to_string())
        );
        for _ in 0..5000 {
            record(Metric::Read, Endpoint::Source, 5, 7);
        }
        let trace = snapshot();
        assert_eq!(trace.capacity, 4096);
        assert_eq!(trace.events.len(), 4096);
        assert!(trace.dropped >= 904);
        let json = serde_json::to_string(&trace).unwrap();
        assert!(!json.contains("path"));
        disable();
        record(Metric::Write, Endpoint::Cache, 4, 9);
        assert_eq!(snapshot().events.len(), 4096);
    }
}

/// SQLite logical pager traffic, distinct from filesystem/physical-device I/O.
#[cfg(feature = "fixture-probes")]
pub(crate) fn catalog_counters(db: &rusqlite::Connection) -> Option<(i32, i32, u64)> {
    if !enabled() {
        return None;
    }
    pager_counters(db)
}

#[cfg(feature = "fixture-probes")]
pub(crate) fn pager_counters(db: &rusqlite::Connection) -> Option<(i32, i32, u64)> {
    let page_size: i64 = db
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .ok()?;
    let get = |kind| {
        let mut current = 0;
        let mut high = 0;
        // SAFETY: the live connection belongs to this catalog thread; SQLite only
        // writes these two correctly sized stack integers. Counters are not reset.
        let result = unsafe {
            rusqlite::ffi::sqlite3_db_status(db.handle(), kind, &mut current, &mut high, 0)
        };
        (result == rusqlite::ffi::SQLITE_OK && current >= 0).then_some(current)
    };
    Some((
        get(rusqlite::ffi::SQLITE_DBSTATUS_CACHE_MISS)?,
        get(rusqlite::ffi::SQLITE_DBSTATUS_CACHE_WRITE)?,
        u64::try_from(page_size).ok()?,
    ))
}
#[cfg(feature = "fixture-probes")]
pub(crate) fn catalog_io(db: &rusqlite::Connection, before: Option<(i32, i32, u64)>, volume: u16) {
    if let (Some(before), Some(after)) = (before, catalog_counters(db)) {
        for (metric, first, last) in [
            (Metric::SqlitePageMiss, before.0, after.0),
            (Metric::SqlitePageWrite, before.1, after.1),
        ] {
            if last >= first {
                record_at(
                    metric,
                    Endpoint::Catalog,
                    volume,
                    0,
                    (last - first) as u64 * before.2,
                );
            }
        }
    }
}
