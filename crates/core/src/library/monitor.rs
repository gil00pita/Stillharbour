//! Filesystem notifications invalidate observations; only safe scans establish state.
use super::{Library, model::RootMonitor};
use crate::domain::{AppError, ErrorCode, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const QUIET: Duration = Duration::from_millis(750);
const MAX_DELAY: Duration = Duration::from_secs(5);
const PERIODIC: Duration = Duration::from_secs(300);
const FALLBACK: Duration = Duration::from_secs(30);
const MAX_WATCHERS: usize = 64;

fn invalidates(event: &Event, managed: &Path) -> bool {
    event.need_rescan()
        || (!matches!(event.kind, EventKind::Access(_))
            && (event.paths.is_empty()
                || !event.paths.iter().all(|path| path.starts_with(managed))))
}

#[derive(Default)]
struct Signal {
    epoch: AtomicU64,
    failed: AtomicBool,
}
#[derive(Clone)]
pub(super) struct Monitor {
    pub stop: Arc<AtomicBool>,
    pub views: Arc<Mutex<Vec<RootMonitor>>>,
}
struct Pending {
    first: Option<Instant>,
    last: Instant,
    checked: Instant,
}
impl Pending {
    fn new(now: Instant) -> Self {
        Self {
            first: Some(now),
            last: now,
            checked: now,
        }
    }
    fn changed(&mut self, now: Instant) {
        self.first.get_or_insert(now);
        self.last = now;
    }
    fn due(&self, now: Instant, interval: Duration) -> bool {
        now.duration_since(self.checked) >= interval
            || self.first.is_some_and(|first| {
                now.duration_since(self.last) >= QUIET || now.duration_since(first) >= MAX_DELAY
            })
    }
    fn finished(&mut self, now: Instant) {
        self.checked = now;
    }
    fn started(&mut self, now: Instant) {
        self.first = None;
        self.checked = now;
    }
}
#[derive(PartialEq)]
struct WatchBinding {
    path: PathBuf,
    device: u64,
    file: u64,
    volume: String,
}
struct RootWatch {
    binding: Option<WatchBinding>,
    finished_job: Option<String>,
    watcher: Option<RecommendedWatcher>,
    signal: Arc<Signal>,
    seen: u64,
    pending: Pending,
    attached: Instant,
}
impl RootWatch {
    fn new(now: Instant) -> Self {
        Self {
            binding: None,
            finished_job: None,
            watcher: None,
            signal: Arc::new(Signal::default()),
            seen: 0,
            pending: Pending::new(now),
            attached: now - FALLBACK,
        }
    }
    fn attach(&mut self, path: &Path, managed: PathBuf, now: Instant) {
        self.attached = now;
        self.watcher = None;
        self.signal.failed.store(false, Ordering::Relaxed);
        let signal = self.signal.clone();
        let result = RecommendedWatcher::new(
            move |event: notify::Result<Event>| match event {
                Ok(event) if invalidates(&event, &managed) => {
                    signal.epoch.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    signal.failed.store(true, Ordering::Relaxed);
                    signal.epoch.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            },
            Config::default().with_follow_symlinks(false),
        )
        .and_then(|mut watcher| {
            watcher.watch(path, RecursiveMode::Recursive)?;
            Ok(watcher)
        });
        match result {
            Ok(watcher) => self.watcher = Some(watcher),
            Err(_) => self.signal.failed.store(true, Ordering::Relaxed),
        }
    }
}

pub(super) fn start(owner: Weak<Library>, managed: PathBuf) -> Result<Monitor> {
    let state = Monitor {
        stop: Arc::new(AtomicBool::new(false)),
        views: Arc::new(Mutex::new(Vec::new())),
    };
    let worker = state.clone();
    std::thread::Builder::new()
        .name("library-monitor".into())
        .spawn(move || run(owner, worker, managed))
        .map_err(|e| {
            AppError::new(
                ErrorCode::WorkerFailed,
                "Library monitoring could not start.",
            )
            .caused_by(e)
        })?;
    Ok(state)
}
fn run(owner: Weak<Library>, state: Monitor, managed: PathBuf) {
    let mut roots: BTreeMap<String, RootWatch> = BTreeMap::new();
    let mut cursor = 0usize;
    while !state.stop.load(Ordering::Relaxed) {
        let Some(library) = owner.upgrade() else {
            break;
        };
        let status = match library.status() {
            Ok(status) => status,
            Err(_) => {
                drop(library);
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };
        let records = match super::store::roots(&library.catalog) {
            Ok(records) => records
                .into_iter()
                .map(|record| (record.view.id.clone(), record))
                .collect::<BTreeMap<_, _>>(),
            Err(_) => {
                drop(library);
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };
        let now = Instant::now();
        roots.retain(|id, _| {
            status
                .roots
                .iter()
                .any(|r| r.id == *id && matches!(r.availability.as_str(), "online" | "scanning"))
        });
        let mut views = Vec::with_capacity(status.roots.len());
        let mut watcher_count = roots.values().filter(|r| r.watcher.is_some()).count();
        for root in &status.roots {
            if !matches!(root.availability.as_str(), "online" | "scanning") {
                views.push(RootMonitor {
                    root_id: root.id.clone(),
                    mode: root.availability.clone(),
                    message: "Retaining last known contents until this library is accessible."
                        .into(),
                });
                continue;
            }
            let entry = roots
                .entry(root.id.clone())
                .or_insert_with(|| RootWatch::new(now));
            let finished = status
                .jobs
                .iter()
                .find(|job| {
                    job.root_id == root.id
                        && !job.id.starts_with("thumbnail:")
                        && !matches!(
                            job.state.as_str(),
                            "queued" | "running" | "cancel_requested"
                        )
                })
                .map(|job| job.id.clone());
            if entry.finished_job != finished {
                entry.finished_job = finished;
                entry.pending.finished(now);
            }
            let Some(record) = records.get(&root.id) else {
                continue;
            };
            let binding = WatchBinding {
                path: record.path.clone(),
                device: record.identity.volume,
                file: record.identity.file,
                volume: record.volume.clone(),
            };
            if entry.binding.as_ref() != Some(&binding) {
                if entry.watcher.is_some() {
                    watcher_count -= 1;
                }
                entry.watcher = None;
                entry.binding = Some(binding);
                entry.attached = now - FALLBACK;
                entry.pending.changed(now);
            }
            let failed = entry.signal.failed.load(Ordering::Relaxed);
            if (entry.watcher.is_none() || failed)
                && now.duration_since(entry.attached) >= FALLBACK
                && (entry.watcher.is_some() || watcher_count < MAX_WATCHERS)
            {
                // Status has freshly validated this root's identity and volume;
                // event paths are never used as scan inputs or execution authority.
                if entry.watcher.is_some() {
                    watcher_count -= 1;
                }
                entry.attach(&record.path, managed.clone(), now);
                if entry.watcher.is_some() {
                    watcher_count += 1;
                }
            }
            let epoch = entry.signal.epoch.load(Ordering::Relaxed);
            if epoch != entry.seen {
                entry.pending.changed(now);
                entry.seen = epoch;
            }
            let native = entry.watcher.is_some() && !entry.signal.failed.load(Ordering::Relaxed);
            views.push(RootMonitor {
                root_id: root.id.clone(),
                mode: if native { "watching" } else { "periodic" }.into(),
                message: if root.availability == "scanning" {
                    "Updating library contents…"
                } else if native {
                    "Watching for changes; safety reconciliation every 5 minutes."
                } else {
                    "Native notifications unavailable; checking every 30 seconds."
                }
                .into(),
            });
        }
        if let Ok(mut target) = state.views.lock() {
            *target = views;
        }
        let idle = library.scan_idle();
        if idle && !state.stop.load(Ordering::Relaxed) {
            let ids: Vec<_> = roots.keys().cloned().collect();
            for offset in 0..ids.len() {
                let index = (cursor + offset) % ids.len();
                let id = &ids[index];
                let entry = &roots[id];
                let native =
                    entry.watcher.is_some() && !entry.signal.failed.load(Ordering::Relaxed);
                if !entry
                    .pending
                    .due(now, if native { PERIODIC } else { FALLBACK })
                {
                    continue;
                }
                let Some(root) = status.roots.iter().find(|r| &r.id == id) else {
                    continue;
                };
                match library.scan(id.clone(), root.filter.clone()) {
                    Ok(_) => {
                        roots.get_mut(id).unwrap().pending.started(now);
                        cursor = (index + 1) % ids.len();
                    }
                    Err(error) if error.code == ErrorCode::Busy => {}
                    Err(_) => {
                        roots.get_mut(id).unwrap().pending.started(now);
                    }
                }
                break;
            }
        }
        drop(library);
        for _ in 0..4 {
            if state.stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_and_owned_cache_writes_do_not_cause_scan_feedback_but_rescan_flags_do() {
        let managed = Path::new("/fixture/app-data");
        assert!(!invalidates(
            &Event::new(EventKind::Access(notify::event::AccessKind::Read))
                .add_path("/fixture/photo.jpg".into()),
            managed
        ));
        assert!(!invalidates(
            &Event::new(EventKind::Any).add_path("/fixture/app-data/cache.png".into()),
            managed
        ));
        assert!(invalidates(
            &Event::new(EventKind::Any).add_path("/fixture/photo.jpg".into()),
            managed
        ));
        assert!(invalidates(
            &Event::new(EventKind::Any)
                .add_path("/fixture/app-data/cache.png".into())
                .set_flag(notify::event::Flag::Rescan),
            managed
        ));
    }
    #[test]
    fn bursts_have_a_quiet_delay_a_maximum_delay_and_periodic_recovery() {
        let now = Instant::now();
        let mut p = Pending::new(now);
        assert!(!p.due(now + Duration::from_millis(500), PERIODIC));
        p.changed(now + Duration::from_millis(500));
        assert!(!p.due(now + Duration::from_secs(1), PERIODIC));
        assert!(p.due(now + Duration::from_millis(1250), PERIODIC));
        p.changed(now + Duration::from_millis(4900));
        assert!(p.due(now + MAX_DELAY, PERIODIC));
        p.started(now + MAX_DELAY);
        assert!(!p.due(now + MAX_DELAY + FALLBACK, PERIODIC));
        assert!(p.due(now + MAX_DELAY + PERIODIC, PERIODIC));
        assert!(p.due(now + MAX_DELAY + FALLBACK, FALLBACK));
        p.finished(now + MAX_DELAY + PERIODIC);
        assert!(!p.due(now + MAX_DELAY + PERIODIC + FALLBACK, PERIODIC));
        p.changed(now + MAX_DELAY + PERIODIC);
        assert!(
            p.due(now + MAX_DELAY + PERIODIC + FALLBACK, PERIODIC),
            "Changes arriving during a scan remain pending after completion"
        );
    }
}
