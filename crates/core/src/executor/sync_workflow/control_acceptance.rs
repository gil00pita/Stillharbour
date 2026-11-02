//! Native generated-fixture controls; real worker, journal and paced I/O checkpoints.
use super::*;
use std::sync::{Condvar, Mutex, OnceLock, mpsc};

struct Gate {
    phase: &'static str,
    reached: AtomicBool,
    arrived: mpsc::SyncSender<()>,
    released: Mutex<bool>,
    changed: Condvar,
}
fn gates() -> &'static Mutex<BTreeMap<String, Arc<Gate>>> {
    static GATES: OnceLock<Mutex<BTreeMap<String, Arc<Gate>>>> = OnceLock::new();
    GATES.get_or_init(|| Mutex::new(BTreeMap::new()))
}
pub(in crate::executor::sync_workflow) fn journal_boundary(job: &str, phase: &str) {
    let gate = gates().lock().unwrap().get(job).cloned();
    let Some(gate) = gate else { return };
    if gate.phase != phase || gate.reached.swap(true, Ordering::AcqRel) {
        return;
    }
    gate.arrived.send(()).unwrap();
    let released = gate.released.lock().unwrap();
    let (released, deadline) = gate
        .changed
        .wait_timeout_while(released, Duration::from_secs(20), |released| !*released)
        .unwrap();
    assert!(
        *released && !deadline.timed_out(),
        "Native control test did not release its checkpoint"
    );
}
struct GateLease {
    job: String,
    gate: Arc<Gate>,
    arrived: mpsc::Receiver<()>,
}
impl GateLease {
    fn new(job: String, phase: &'static str) -> Self {
        let (sent, arrived) = mpsc::sync_channel(1);
        let gate = Arc::new(Gate {
            phase,
            reached: AtomicBool::new(false),
            arrived: sent,
            released: Mutex::new(false),
            changed: Condvar::new(),
        });
        assert!(
            gates()
                .lock()
                .unwrap()
                .insert(job.clone(), gate.clone())
                .is_none()
        );
        Self { job, gate, arrived }
    }
    fn wait(&self) {
        self.arrived
            .recv_timeout(Duration::from_secs(20))
            .expect("Real application checkpoint was not reached");
    }
    fn release(&self) {
        *self.gate.released.lock().unwrap() = true;
        self.gate.changed.notify_all();
    }
    fn release_best_effort(&self) {
        match self.gate.released.lock() {
            Ok(mut released) => *released = true,
            Err(poisoned) => *poisoned.into_inner() = true,
        }
        self.gate.changed.notify_all();
    }
    fn rearm(&self) {
        *self.gate.released.lock().unwrap() = false;
        self.gate.reached.store(false, Ordering::Release);
    }
}
impl Drop for GateLease {
    fn drop(&mut self) {
        match gates().lock() {
            Ok(mut gates) => {
                gates.remove(&self.job);
            }
            Err(poisoned) => {
                poisoned.into_inner().remove(&self.job);
            }
        }
        self.release_best_effort();
    }
}
#[test]
fn sync_application_control_gate_cleanup_does_not_panic_on_poisoned_release() {
    let id = uuid::Uuid::new_v4().to_string();
    let lease = GateLease::new(id.clone(), "copy_chunk");
    let gate = lease.gate.clone();
    let poison = gate.clone();
    assert!(
        std::thread::spawn(move || {
            let _held = poison.released.lock().unwrap();
            panic!("Generated test-only release-lock poison");
        })
        .join()
        .is_err()
    );
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(lease))).is_ok());
    assert!(!gates().lock().unwrap().contains_key(&id));
    assert!(*gate.released.lock().unwrap_err().into_inner());
}
#[cfg(any(windows, target_os = "macos"))]
struct ActiveCase {
    fixture: Option<Fixture>,
    gate: GateLease,
    job: String,
    old_rate: u64,
    old_pause: bool,
    bytes: BTreeMap<String, Vec<u8>>,
    identities: BTreeMap<String, filesystem::FileIdentity>,
    neighbor: filesystem::FileIdentity,
}
#[cfg(any(windows, target_os = "macos"))]
impl ActiveCase {
    fn new(phase: &'static str) -> Self {
        let io = PacedIo::shared();
        let old_rate = io.bytes_per_second().unwrap();
        let old_pause = io.is_paused().unwrap();
        assert!(
            !old_pause,
            "Run the ignored global-budget matrix in isolation"
        );
        let fixture = Fixture::new();
        for name in [".hidden", "movie.mp4", "movie.xmp"] {
            let payload: Vec<u8> = (0..512 * 1024).map(|i| (i % 251) as u8).collect();
            std::fs::write(fixture.source.join(name), payload).unwrap();
        }
        let mut bytes = BTreeMap::new();
        let mut identities = BTreeMap::new();
        for entry in std::fs::read_dir(&fixture.source).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            bytes.insert(name.clone(), std::fs::read(entry.path()).unwrap());
            identities.insert(
                name,
                filesystem::identity(&std::fs::File::open(entry.path()).unwrap()).unwrap(),
            );
        }
        let (prepared, review) = fixture.prepared();
        std::fs::write(
            fixture.dataset().join("destination-only.bin"),
            b"unique dataset neighbor",
        )
        .unwrap();
        let neighbor = filesystem::identity(
            &std::fs::File::open(fixture.dataset().join("destination-only.bin")).unwrap(),
        )
        .unwrap();
        let gate = GateLease::new(prepared.id.clone(), phase);
        let case = Self {
            fixture: Some(fixture),
            gate,
            job: prepared.id,
            old_rate,
            old_pause,
            bytes,
            identities,
            neighbor,
        };
        let f = case.f();
        let approval = f.approved(&review);
        let started = f.workflow.start(f.library.clone(), approval.id).unwrap();
        assert_eq!(started.id, case.job);
        case.gate.wait();
        case
    }
    fn f(&self) -> &Fixture {
        self.fixture.as_ref().unwrap()
    }
    fn job(&self) -> SyncJob {
        self.f().workflow.job(&self.job).unwrap()
    }
    fn control(&self, request: SyncRequest) -> SyncJob {
        let SyncReply::Job(job) = self
            .f()
            .workflow
            .handle(self.f().library.clone(), request)
            .unwrap()
        else {
            panic!("Expected actual native job reply")
        };
        assert_eq!(job.id, self.job);
        job
    }
    fn pause(&self) -> SyncJob {
        let before = self.job();
        let paused = self.control(SyncRequest::Pause {
            job_id: self.job.clone(),
        });
        assert_eq!(paused.state, SyncState::Paused);
        assert!(PacedIo::shared().is_paused().unwrap());
        assert_eq!(paused.verified_files, before.verified_files);
        self.gate.release();
        let until = Instant::now() + Duration::from_secs(5);
        while PacedIo::shared().pending_request_count().unwrap() == 0 {
            assert!(
                Instant::now() < until,
                "Actual application worker never queued its paused I/O"
            );
            std::thread::yield_now();
        }
        assert_eq!(self.job().verified_files, before.verified_files);
        paused
    }
    fn rate(&self, rate: u64) {
        let SyncReply::Capabilities(cap) = self
            .f()
            .workflow
            .handle(
                self.f().library.clone(),
                SyncRequest::SetLimit {
                    bytes_per_second: rate.to_string(),
                },
            )
            .unwrap()
        else {
            panic!("Expected shared I/O capability reply")
        };
        assert_eq!(cap.bytes_per_second, rate.to_string());
        assert_eq!(self.job().bytes_per_second, rate.to_string());
    }
    fn audit(&self) {
        for (name, expected) in &self.bytes {
            let source = self.f().source.join(name);
            assert_eq!(std::fs::read(&source).unwrap(), *expected);
            assert_eq!(
                filesystem::identity(&std::fs::File::open(source).unwrap()).unwrap(),
                self.identities[name]
            );
            let output = self.f().dataset().join(name);
            if output.exists() {
                assert_eq!(std::fs::read(output).unwrap(), *expected);
            }
        }
        let neighbor = self.f().dataset().join("destination-only.bin");
        assert_eq!(
            std::fs::read(&neighbor).unwrap(),
            b"unique dataset neighbor"
        );
        assert_eq!(
            filesystem::identity(&std::fs::File::open(neighbor).unwrap()).unwrap(),
            self.neighbor
        );
        assert_eq!(
            std::fs::read(self.f().destination.join("outside.bin")).unwrap(),
            b"unique neighbor"
        );
    }
    fn finished(&self, complete: bool) {
        let done = self.f().wait(&self.job);
        if complete {
            assert_eq!(done.state, SyncState::CompletedVerified, "{}", done.message);
            assert_eq!(done.verified_files, done.total_files);
            for name in self.bytes.keys() {
                assert!(self.f().dataset().join(name).is_file());
            }
        } else {
            assert_eq!(done.state, SyncState::NeedsRecovery, "{}", done.message);
            assert!(done.verified_files < done.total_files);
            assert!(
                !self
                    .f()
                    .workflow
                    .events(&self.job)
                    .unwrap()
                    .iter()
                    .any(|(_, phase, _)| phase == "copy/plan_completed_verified")
            );
        }
        assert!(!PacedIo::shared().is_paused().unwrap());
        self.audit();
    }
}
#[cfg(any(windows, target_os = "macos"))]
impl Drop for ActiveCase {
    fn drop(&mut self) {
        if let Some(f) = &self.fixture {
            f.workflow.cancel_all();
        }
        self.gate.release_best_effort();
        let until = Instant::now() + Duration::from_secs(5);
        let mut retain = std::thread::panicking();
        loop {
            let stopped = match self.fixture.as_ref() {
                Some(f) => match f.workflow.active.lock() {
                    Ok(active) => active.is_none(),
                    Err(_) => {
                        retain = true;
                        break;
                    }
                },
                None => true,
            };
            if stopped {
                break;
            }
            if Instant::now() >= until {
                retain = true;
                break;
            }
            std::thread::yield_now();
        }
        if retain {
            // A failed assertion or unknown stop state cannot authorize fixture cleanup.
            std::mem::forget(self.fixture.take());
        }
        let io = PacedIo::shared();
        let _ = io.set_rate(self.old_rate);
        let _ = io.set_paused(self.old_pause);
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
#[ignore = "native generated fixtures change the global application I/O budget; run this matrix alone"]
fn sync_application_native_controls_pause_resume_change_rate_cancel_and_publication_boundary() {
    for phase in ["copy_chunk", "stage_flushed"] {
        // Each cycle reaches a real copied chunk or a flushed stage, then blocks
        // in the application's actual paced read queue before changing live rates.
        let case = ActiveCase::new(phase);
        for iteration in 0..3 {
            case.pause();
            for rate in [1024, 1024 * 1024, 4096, 16 * 1024 * 1024] {
                case.rate(rate);
            }
            if iteration < 2 {
                case.gate.rearm();
            }
            case.control(SyncRequest::Resume {
                job_id: case.job.clone(),
            });
            if iteration < 2 {
                case.gate.wait();
            }
        }
        case.finished(true);
        drop(case);

        let case = ActiveCase::new(phase);
        let paused = case.pause();
        case.rate(2 * 1024 * 1024);
        case.control(SyncRequest::Cancel {
            job_id: case.job.clone(),
        });
        let done = case.f().wait(&case.job);
        assert_eq!(done.verified_files, paused.verified_files);
        case.finished(false);
    }

    // Pause is a byte-admission control. A no-replace metadata publication that
    // was already reached may finish; its final read-back remains paused.
    let case = ActiveCase::new("publish_intent");
    let paused = case.pause();
    let events = case.f().workflow.events(&case.job).unwrap();
    let index = events
        .iter()
        .rev()
        .find(|(_, phase, _)| phase == "copy/publish_intent")
        .unwrap()
        .0
        .unwrap();
    let plan = case.f().workflow.plan(&case.job).unwrap().unwrap();
    assert!(
        case.f().dataset().join(&plan.files[index].source).is_file(),
        "Already admitted metadata publication is allowed to finish"
    );
    case.control(SyncRequest::Cancel {
        job_id: case.job.clone(),
    });
    let done = case.f().wait(&case.job);
    assert_eq!(done.verified_files, paused.verified_files);
    case.finished(false);
}
