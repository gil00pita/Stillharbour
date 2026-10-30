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
}

#[cfg(feature = "fixture-probes")]
mod recorder {
    use super::*;
    use std::{
        collections::BTreeMap,
        sync::{
            Mutex, OnceLock,
            atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        },
    };
    const CAPACITY: usize = 4096;
    const LANES: usize = 16;
    const LANE_CAPACITY: usize = CAPACITY / LANES;
    static NEXT_PRODUCER: AtomicUsize = AtomicUsize::new(0);
    thread_local! { static PRODUCER_LANE: usize = NEXT_PRODUCER.fetch_add(1, Ordering::Relaxed) % LANES; }
    pub(super) struct Recorder {
        enabled: AtomicBool,
        lost: AtomicU64,
        lanes: [Mutex<Vec<Event>>; LANES],
        volumes: Mutex<BTreeMap<u64, u16>>,
    }
    impl Default for Recorder {
        fn default() -> Self {
            Self {
                enabled: AtomicBool::new(false),
                lost: AtomicU64::new(0),
                lanes: std::array::from_fn(|_| Mutex::new(Vec::with_capacity(LANE_CAPACITY))),
                volumes: Mutex::new(BTreeMap::new()),
            }
        }
    }
    impl Recorder {
        pub(super) fn enabled(&self) -> bool {
            self.enabled.load(Ordering::Relaxed)
        }
        pub(super) fn enable(&self) {
            self.enabled.store(false, Ordering::Relaxed);
            let Ok(mut lanes) = self
                .lanes
                .iter()
                .map(Mutex::lock)
                .collect::<std::result::Result<Vec<_>, _>>()
            else {
                return;
            };
            let Ok(mut volumes) = self.volumes.lock() else {
                return;
            };
            for lane in &mut lanes {
                lane.clear();
            }
            volumes.clear();
            self.lost.store(0, Ordering::Relaxed);
            self.enabled.store(true, Ordering::Relaxed);
        }
        pub(super) fn record(&self, event: Event) {
            if !self.enabled() {
                return;
            }
            let start = PRODUCER_LANE.with(|lane| *lane);
            // Stable preferred lanes avoid producers chasing one global rotating
            // slot. Probe at most sixteen lanes, without waiting or allocating.
            // Sixteen x 256 events retains the original 4096-event total bound.
            for offset in 0..LANES {
                if let Ok(mut lane) = self.lanes[(start + offset) % LANES].try_lock()
                    && lane.len() < LANE_CAPACITY
                {
                    lane.push(event);
                    return;
                }
            }
            self.lost.fetch_add(1, Ordering::Relaxed);
        }
        #[cfg(test)]
        pub(super) fn with_producer_held(&self, work: impl FnOnce()) {
            let lane = PRODUCER_LANE.with(|lane| *lane);
            let _held = self.lanes[lane].lock().unwrap();
            work();
        }
        #[cfg(test)]
        pub(super) fn with_all_producers_held(&self, work: impl FnOnce()) {
            let _held = self
                .lanes
                .iter()
                .map(Mutex::lock)
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap();
            work();
        }
        pub(super) fn snapshot(&self) -> Snapshot {
            self.read(false)
        }
        pub(super) fn drain(&self) -> Snapshot {
            self.read(true)
        }
        fn read(&self, consume: bool) -> Snapshot {
            // Allocate before acquiring producer lanes. A consuming read swaps
            // Vec headers under the locks, then flattens outside the critical section.
            let mut taken: [Vec<Event>; LANES] = std::array::from_fn(|_| {
                if consume {
                    Vec::with_capacity(LANE_CAPACITY)
                } else {
                    Vec::new()
                }
            });
            let mut events = Vec::with_capacity(CAPACITY);
            let Ok(volumes) = self.volumes.lock() else {
                return Snapshot::default();
            };
            let Ok(mut lanes) = self
                .lanes
                .iter()
                .map(Mutex::lock)
                .collect::<std::result::Result<Vec<_>, _>>()
            else {
                return Snapshot::default();
            };
            for (index, lane) in lanes.iter_mut().enumerate() {
                if consume {
                    std::mem::swap(&mut **lane, &mut taken[index]);
                } else {
                    events.extend(lane.iter().cloned());
                }
            }
            let dropped = if consume {
                self.lost.swap(0, Ordering::Relaxed)
            } else {
                self.lost.load(Ordering::Relaxed)
            };
            let volume_count = volumes.len();
            drop(lanes);
            drop(volumes);
            if consume {
                events.extend(taken.into_iter().flatten());
            }
            Snapshot {
                events,
                dropped,
                capacity: CAPACITY,
                volume_count,
            }
        }
        pub(super) fn volume_alias(&self, volume: u64) -> u16 {
            if !self.enabled() {
                return 0;
            }
            if let Ok(mut volumes) = self.volumes.try_lock() {
                if let Some(alias) = volumes.get(&volume) {
                    return *alias;
                }
                if volumes.len() >= 16 {
                    return 0;
                }
                let alias = volumes.len() as u16 + 1;
                volumes.insert(volume, alias);
                alias
            } else {
                0
            }
        }
        pub(super) fn disable(&self) {
            self.enabled.store(false, Ordering::Relaxed);
        }
    }
    static TRACE: OnceLock<Recorder> = OnceLock::new();
    pub fn enabled() -> bool {
        TRACE.get().is_some_and(Recorder::enabled)
    }
    pub fn enable() {
        TRACE.get_or_init(Recorder::default).enable();
    }
    pub fn disable() {
        if let Some(trace) = TRACE.get() {
            trace.disable();
        }
    }
    pub fn record(event: Event) {
        if let Some(trace) = TRACE.get() {
            trace.record(event);
        }
    }
    pub fn snapshot() -> Snapshot {
        TRACE.get().map(Recorder::snapshot).unwrap_or_default()
    }
    pub fn drain() -> Snapshot {
        TRACE.get().map(Recorder::drain).unwrap_or_default()
    }
    pub fn volume_alias(volume: u64) -> u16 {
        TRACE
            .get()
            .map(|trace| trace.volume_alias(volume))
            .unwrap_or(0)
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
/// Consume a bounded diagnostic segment without disabling recording or remapping volumes.
pub fn drain() -> Snapshot {
    #[cfg(feature = "fixture-probes")]
    {
        recorder::drain()
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
        // The same recorder implementation as the singleton, isolated from
        // unrelated catalog tests recording on other threads in this process.
        let trace = recorder::Recorder::default();
        let event = |metric, endpoint, elapsed_us, bytes| Event {
            metric,
            endpoint,
            elapsed_us,
            bytes,
            volume_alias: 0,
        };
        trace.enable();
        let private_volume = 9182736450918273645u64;
        let alias = trace.volume_alias(private_volume);
        assert!((1..=16).contains(&alias));
        for device in 0..100 {
            trace.volume_alias(device);
        }
        assert_eq!(trace.snapshot().volume_count, 16);
        assert!(
            !serde_json::to_string(&trace.snapshot())
                .unwrap()
                .contains(&private_volume.to_string())
        );
        for _ in 0..5000 {
            trace.record(event(Metric::Read, Endpoint::Source, 5, 7));
        }
        let snapshot = trace.snapshot();
        assert_eq!(snapshot.capacity, 4096);
        assert_eq!(snapshot.events.len(), 4096);
        assert_eq!(snapshot.dropped, 904);
        assert!(!serde_json::to_string(&snapshot).unwrap().contains("path"));
        let segment = trace.drain();
        assert_eq!(segment.events.len(), 4096);
        assert_eq!(segment.dropped, 904);
        assert!(trace.snapshot().events.is_empty());
        assert_eq!(trace.snapshot().dropped, 0);
        assert_eq!(trace.volume_alias(private_volume), alias);
        trace.record(event(Metric::Write, Endpoint::Cache, 4, 9));
        let next = trace.drain();
        assert_eq!(next.events.len(), 1);
        assert_eq!(next.events[0].bytes, 9);
        assert_eq!(next.volume_count, 16);
        trace.with_producer_held(|| trace.record(event(Metric::Read, Endpoint::Source, 1, 2)));
        let held = trace.drain();
        assert_eq!(held.events.len(), 1);
        assert_eq!(
            held.dropped, 0,
            "One busy producer lost an under-capacity event"
        );
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            for producer in 0..8 {
                let (barrier, trace, event) = (&barrier, &trace, &event);
                scope.spawn(move || {
                    barrier.wait();
                    for index in 0..128 {
                        trace.record(event(Metric::Read, Endpoint::Source, producer, index));
                    }
                });
            }
        });
        let concurrent = trace.drain();
        assert_eq!(concurrent.events.len() + concurrent.dropped as usize, 1024);
        assert_eq!(
            concurrent.dropped, 0,
            "Under-capacity concurrent trace lost events"
        );
        assert_eq!(concurrent.events.len(), 1024);
        trace.with_all_producers_held(|| trace.record(event(Metric::Read, Endpoint::Source, 1, 2)));
        let contended = trace.drain();
        assert!(contended.events.is_empty());
        assert_eq!(
            contended.dropped, 1,
            "Unavoidable contention must remain visible"
        );
        trace.disable();
        trace.record(event(Metric::Write, Endpoint::Cache, 4, 9));
        assert!(trace.snapshot().events.is_empty());
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
