//! Generated native acceptance for recurring jobs, unreadable source scope and coordination.
use super::application_tests::{Fixture, wait};
use super::*;
use std::sync::{Condvar, atomic::AtomicUsize};
mod denied;

fn helper_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/backup-helpers")
        .join(if cfg!(windows) {
            "windows-x86_64"
        } else {
            "macos-aarch64"
        })
        .canonicalize()
        .unwrap()
}
struct StopWorkflow(Arc<SnapshotWorkflow>, Arc<AtomicBool>);
impl Drop for StopWorkflow {
    fn drop(&mut self) {
        self.0.cancel_all();
        let end = Instant::now() + Duration::from_secs(20);
        while !self.0.idle().unwrap_or(false) && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(10));
        }
        if !self.0.idle().unwrap_or(false) {
            self.1.store(true, Ordering::Release);
        }
    }
}
struct NativeFixture {
    fixture: Fixture,
    catalog: String,
    credential: String,
    profile: String,
    retain: Arc<AtomicBool>,
}
impl NativeFixture {
    fn new() -> Self {
        assert!(
            std::env::var_os("STILLHARBOUR_M4_GENERATED_REPOSITORY_DRIVE").is_none(),
            "Native acceptance owns only its generated system-temp fixture."
        );
        let fixture = Fixture::new();
        fixture
            .library
            .configure_snapshot_helpers(helper_root())
            .unwrap();
        let catalog = fixture.library.catalog.summary().unwrap().id;
        let credential = CredentialStore::store(
            &catalog,
            CredentialPurpose::Repository,
            &uuid::Uuid::new_v4().to_string(),
        )
        .unwrap();
        let selection = Fixture::selection(&fixture.backup);
        let profile = fixture
            .library
            .snapshots
            .profile_create(
                &fixture.library,
                "Generated native acceptance".into(),
                vec![fixture.root.clone()],
                credential.clone(),
                None,
                SnapshotDestinationInput::Local {
                    selection_id: selection.id.clone(),
                },
                Some(selection),
            )
            .unwrap()
            .id;
        Self {
            fixture,
            catalog,
            credential,
            profile,
            retain: Arc::new(AtomicBool::new(false)),
        }
    }
    fn review(&self) -> SnapshotReview {
        let s = &self.fixture.library.snapshots;
        s.review(s.profile(&self.profile).unwrap()).unwrap()
    }
    fn start(&self, review: &SnapshotReview) -> SnapshotJob {
        let s = &self.fixture.library.snapshots;
        let approval = s.approve(&review.id, &review.digest, "backup").unwrap();
        let frozen = s.take_approval(&review.id, &approval.id, "backup").unwrap();
        s.start(self.fixture.library.clone(), frozen, "backup")
            .unwrap()
    }
    fn backup(&self) -> SnapshotJob {
        let job = self.start(&self.review());
        let done = wait(&self.fixture.library, &job.id);
        assert_eq!(done.state, "completed_verified", "{}", done.message);
        done
    }
    fn schedule(&self, catch_up: bool) -> ScheduleRecord {
        let review = self.review();
        self.fixture
            .library
            .snapshots
            .schedule_save(
                &self.profile,
                &review.id,
                &review.digest,
                15,
                "UTC",
                true,
                catch_up,
            )
            .unwrap();
        self.fixture
            .library
            .snapshots
            .schedule_record(&self.profile)
            .unwrap()
            .unwrap()
    }
    fn authority(&self, record: &ScheduleRecord) -> Option<ScheduleAuthority> {
        let policy = SnapshotWorkflow::schedule_policy(record).unwrap();
        schedule_transaction()
            .unwrap()
            .read(&ScheduleScope {
                catalog_id: &self.catalog,
                instance: &record.instance,
                profile_id: &self.profile,
                policy_digest: &policy,
            })
            .unwrap()
    }
    fn due(&self, record: &mut ScheduleRecord, overdue: chrono::Duration) {
        record.view.next_due = (chrono::Utc::now() - overdue).to_rfc3339();
        let policy = SnapshotWorkflow::schedule_policy(record).unwrap();
        let scope = ScheduleScope {
            catalog_id: &self.catalog,
            instance: &record.instance,
            profile_id: &self.profile,
            policy_digest: &policy,
        };
        // Exact constructor-owned native consent AND catalog timing are seeded
        // together to simulate elapsed time; row-only replay is tested below.
        let mut transaction = schedule_transaction().unwrap();
        transaction
            .authorize(
                &scope,
                chrono::DateTime::parse_from_rfc3339(&record.view.next_due)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            )
            .unwrap();
        self.fixture
            .library
            .snapshots
            .persist_schedule(record)
            .unwrap();
    }
}
impl Drop for NativeFixture {
    fn drop(&mut self) {
        if let Ok(mut state) = self.fixture.library.snapshots.state.lock() {
            state.test_phase = None;
        } else {
            self.retain.store(true, Ordering::Release);
        }
        self.fixture.library.snapshots.cancel_all();
        let end = Instant::now() + Duration::from_secs(20);
        while !self.fixture.library.snapshots.idle().unwrap_or(false) && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(10));
        }
        if std::thread::panicking()
            || self.retain.load(Ordering::Acquire)
            || !self.fixture.library.snapshots.idle().unwrap_or(false)
        {
            // Do not recursively clean an uncertain fixture or revoke the only
            // credential needed to inspect its retained copies.
            self.fixture._run.disable_cleanup(true);
            eprintln!(
                "Generated native acceptance state is uncertain; its owned fixture and credential remain retained."
            );
            return;
        }
        let _ = crate::backup::credentials::schedule_revoke(
            &self.catalog,
            &self.fixture.library.snapshots.instance().unwrap(),
            &self.profile,
        );
        let _ = CredentialStore::delete_owned(
            &self.catalog,
            CredentialPurpose::Repository,
            &self.credential,
        );
    }
}
#[test]
fn unchecked_catch_up_has_a_precise_thirty_second_poll_tolerance() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-11-02T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert!(!skip_missed_schedule(
        false,
        now - chrono::Duration::seconds(30),
        now
    ));
    assert!(skip_missed_schedule(
        false,
        now - chrono::Duration::seconds(30) - chrono::Duration::nanoseconds(1),
        now
    ));
    for delay in [60, 900, 86400] {
        // sleep, busy backlog, restart
        assert!(skip_missed_schedule(
            false,
            now - chrono::Duration::seconds(delay),
            now
        ));
        assert!(!skip_missed_schedule(
            true,
            now - chrono::Duration::seconds(delay),
            now
        ));
    }
}
#[test]
#[ignore = "actual native OS consent and pinned helpers; generated fixtures only"]
fn native_due_schedule_runs_once_skips_missed_and_revokes_changed_scope() {
    let f = NativeFixture::new();
    f.backup();
    let library = &f.fixture.library;
    let s = &library.snapshots;
    let mut record = f.schedule(true);
    assert!(f.authority(&record).is_some());
    f.due(&mut record, chrono::Duration::hours(2));
    let prior_jobs = s
        .jobs(Some(&f.profile))
        .unwrap()
        .into_iter()
        .map(|j| j.id)
        .collect::<BTreeSet<_>>();
    let before = prior_jobs.len();
    let earliest = chrono::Utc::now() + chrono::Duration::minutes(15);
    s.run_due(library.clone()).unwrap();
    let jobs = s.jobs(Some(&f.profile)).unwrap();
    assert_eq!(jobs.len(), before + 1);
    let job = jobs
        .iter()
        .find(|job| !prior_jobs.contains(&job.id))
        .unwrap();
    let done = wait(library, &job.id);
    assert_eq!(done.state, "completed_verified", "{}", done.message);
    let saved = s.schedule_record(&f.profile).unwrap().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(&saved.view.next_due).unwrap() >= earliest);
    s.run_due(library.clone()).unwrap();
    assert_eq!(s.jobs(Some(&f.profile)).unwrap().len(), before + 1);
    // Replaying only the old catalog timing cannot advance the OS high-water
    // or hide its real last attempt. The current native authority wins.
    let advanced = f.authority(&record).unwrap();
    s.persist_schedule(&record).unwrap();
    let mirrored = s.schedule_get(&f.profile).unwrap().unwrap();
    assert_eq!(mirrored.next_due, advanced.next_admissible_utc);
    assert_eq!(mirrored.last_run, advanced.last_attempt_utc);
    s.run_due(library.clone()).unwrap();
    assert_eq!(s.jobs(Some(&f.profile)).unwrap().len(), before + 1);
    assert_eq!(f.authority(&record).unwrap(), advanced);
    // Recreate the native workflow session. Its real OS permit survives, but
    // the saved row grants no repository execution authority on its own.
    let cold =
        Arc::new(SnapshotWorkflow::new(library.catalog.clone(), s.data.path().unwrap()).unwrap());
    cold.configure_helpers(helper_root()).unwrap();
    let _stop = StopWorkflow(cold.clone(), f.retain.clone());
    let mut resumed = f.schedule(true);
    f.due(&mut resumed, chrono::Duration::hours(3));
    cold.run_due(library.clone()).unwrap();
    assert_eq!(cold.jobs(Some(&f.profile)).unwrap().len(), before + 1);
    assert!(cold.state.lock().unwrap().authorized.is_empty());
    let selection = Fixture::selection(&f.fixture.backup);
    cold.dispatch(
        library.clone(),
        SnapshotRequest::ProfileOpen {
            profile_id: f.profile.clone(),
            selection_id: Some(selection.id.clone()),
        },
        Some(selection),
    )
    .unwrap();
    let known = cold
        .jobs(Some(&f.profile))
        .unwrap()
        .into_iter()
        .map(|j| j.id)
        .collect::<BTreeSet<_>>();
    let began = Instant::now();
    let next_after = chrono::Utc::now() + chrono::Duration::minutes(15);
    cold.tick(library.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(90);
    let timer_job = loop {
        if let Some(job) = cold
            .jobs(Some(&f.profile))
            .unwrap()
            .into_iter()
            .find(|j| !known.contains(&j.id))
        {
            break job;
        }
        assert!(
            Instant::now() < deadline,
            "Native running timer did not admit its due exact-scope job."
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(began.elapsed() >= Duration::from_secs(14));
    let completed = loop {
        let job = cold
            .jobs(Some(&f.profile))
            .unwrap()
            .into_iter()
            .find(|j| j.id == timer_job.id)
            .unwrap();
        if !matches!(
            job.state.as_str(),
            "queued" | "running" | "cancel_requested"
        ) && cold.idle().unwrap()
        {
            break job;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        completed.state, "completed_verified",
        "{}",
        completed.message
    );
    assert!(
        completed.included
            && completed.structural_checked
            && completed.data_read_verified
            && completed.restored_compared
            && !completed.independent_recovery_verified
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(
            &cold
                .schedule_record(&f.profile)
                .unwrap()
                .unwrap()
                .view
                .next_due
        )
        .unwrap()
            >= next_after
    );
    cold.cancel_all();
    assert!(cold.idle().unwrap());
    cold.run_due(library.clone()).unwrap();
    let expected = before + 2;
    assert_eq!(cold.jobs(Some(&f.profile)).unwrap().len(), expected);
    // Each old due time represents a missed wakeup/restart/busy deadline;
    // native OS consent remains real, but unchecked catch-up admits no job.
    for delay in [60, 900, 86400] {
        let mut skipped = f.schedule(false);
        f.due(&mut skipped, chrono::Duration::seconds(delay));
        s.run_due(library.clone()).unwrap();
        assert_eq!(s.jobs(Some(&f.profile)).unwrap().len(), expected);
        assert!(s.schedule_get(&f.profile).unwrap().unwrap().enabled);
    }
    let mut changed = f.schedule(true);
    f.due(&mut changed, chrono::Duration::hours(1));
    std::fs::write(
        f.fixture.source.join("changed-after-consent.txt"),
        b"generated newly changed scope",
    )
    .unwrap();
    s.run_due(library.clone()).unwrap();
    assert_eq!(s.jobs(Some(&f.profile)).unwrap().len(), expected);
    assert!(!s.schedule_get(&f.profile).unwrap().unwrap().enabled);
    assert!(f.authority(&changed).is_none());
    // Disable remains possible after the whole original root is unavailable.
    let mut offline = f.schedule(true);
    f.due(&mut offline, chrono::Duration::hours(1));
    let renamed = f.fixture.source.with_file_name("generated-offline-root");
    std::fs::rename(&f.fixture.source, &renamed).unwrap();
    assert!(!s.schedule_disable(&f.profile).unwrap().unwrap().enabled);
    assert!(f.authority(&offline).is_none());
    s.run_due(library.clone()).unwrap();
    assert_eq!(s.jobs(Some(&f.profile)).unwrap().len(), expected);
    assert_eq!(
        std::fs::read(renamed.join("original.txt")).unwrap(),
        b"generated preserved original"
    );
}

#[derive(Default)]
struct Gate {
    state: Mutex<(bool, bool)>,
    changed: Condvar,
}
impl Gate {
    fn enter(&self) {
        let mut state = self.state.lock().unwrap();
        state.0 = true;
        self.changed.notify_all();
        while !state.1 {
            state = self.changed.wait(state).unwrap();
        }
    }
    fn wait(&self) {
        let state = self.state.lock().unwrap();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(30), |s| !s.0)
            .unwrap();
        let ready = !timeout.timed_out() && state.0;
        drop(state);
        assert!(
            ready,
            "Generated snapshot did not reach its held native phase."
        );
    }
    fn release(&self) {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .1 = true;
        self.changed.notify_all();
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}

#[test]
#[ignore = "actual native permission denial and pinned helper scope; generated fixtures only"]
fn native_two_root_backup_loses_readable_scope_after_first_root_is_staged() {
    let mut f = NativeFixture::new();
    let root2 = f.fixture._run.path().join("second-generated-source");
    std::fs::create_dir(&root2).unwrap();
    let root2 = root2.canonicalize().unwrap();
    let second_file = root2.join("unique-second.txt");
    std::fs::write(
        &second_file,
        b"generated second source must survive unreadable scope",
    )
    .unwrap();
    let file = native::open_root(&root2, None).unwrap();
    let second = store::add_root(
        &f.fixture.library.catalog,
        root2.clone(),
        filesystem::identity(&file).unwrap(),
        native::volume_token(&file).unwrap(),
        "Generated second source".into(),
        super::super::model::RootMode::Referenced,
        super::super::model::ScanFilter::default(),
    )
    .unwrap();
    drop(file);
    let selection = Fixture::selection(&f.fixture.backup);
    f.profile = f
        .fixture
        .library
        .snapshots
        .profile_create(
            &f.fixture.library,
            "Generated two-root dropout".into(),
            vec![f.fixture.root.clone(), second],
            f.credential.clone(),
            None,
            SnapshotDestinationInput::Local {
                selection_id: selection.id.clone(),
            },
            Some(selection),
        )
        .unwrap()
        .id;
    std::fs::write(
        f.fixture.backup.join("neighbor.txt"),
        b"generated destination-only neighbor",
    )
    .unwrap();
    let ordered = f.fixture.library.snapshots.profile(&f.profile).unwrap();
    let first_root = ordered.roots[0].id.clone();
    let unreadable_root = ordered.roots[1].id.clone();
    let denied_path = if ordered.roots[1].id == f.fixture.root {
        f.fixture.source.join("original.txt")
    } else {
        second_file.clone()
    };
    let mut denial =
        denied::DeniedSource::new(f.fixture._run.path(), &denied_path, f.retain.clone());
    let gate = Arc::new(Gate::default());
    let _release = Release(gate.clone());
    let staged = Arc::new(AtomicUsize::new(0));
    let source = first_root.clone();
    let hook_gate = gate.clone();
    let hook_staged = staged.clone();
    f.fixture.library.snapshots.state.lock().unwrap().test_phase =
        Some(Arc::new(move |_, job, phase| {
            if phase == "source_staged" && hook_staged.fetch_add(1, Ordering::AcqRel) == 0 {
                assert_eq!(job.profile.roots[0].id, source);
                let version = job
                    .manifest
                    .files
                    .iter()
                    .find(|v| v.root_id == source)
                    .unwrap();
                let path = job
                    .work
                    .as_ref()
                    .unwrap()
                    .path()
                    .unwrap()
                    .join(format!("payload-{}", job.view.id))
                    .join(&version.archive);
                let mut file = filesystem::open_readonly(&path).unwrap();
                assert_eq!(
                    bridge::digest(&mut file, &AtomicBool::new(false)).unwrap(),
                    version.digest
                );
                hook_gate.enter();
            }
        }));
    let job = f.start(&f.review());
    gate.wait();
    assert!(staged.load(Ordering::Acquire) >= 1);
    denial.deny().unwrap();
    assert!(
        filesystem::open_readonly(&denied_path).is_err(),
        "Generated source did not actually become unreadable."
    );
    gate.release();
    let stopped = wait(&f.fixture.library, &job.id);
    assert_eq!(stopped.state, "needs_recovery", "{}", stopped.message);
    assert!(
        stopped.message.contains(&unreadable_root),
        "The failed included root is not identified."
    );
    assert!(
        !stopped.included
            && !stopped.structural_checked
            && !stopped.data_read_verified
            && !stopped.restored_compared
            && !stopped.independent_recovery_verified
    );
    assert!(stopped.source_retained);
    denial.restore().unwrap();
    assert_eq!(
        std::fs::read(&second_file).unwrap(),
        b"generated second source must survive unreadable scope"
    );
    assert_eq!(
        std::fs::read(f.fixture.source.join("original.txt")).unwrap(),
        b"generated preserved original"
    );
    assert_eq!(
        std::fs::read(f.fixture.source.join("hidden-source/companion.xmp")).unwrap(),
        b"generated sidecar outside gallery filter"
    );
    assert_eq!(
        std::fs::read(f.fixture.backup.join("neighbor.txt")).unwrap(),
        b"generated destination-only neighbor"
    );
    let text = f
        .fixture
        .library
        .catalog
        .call({
            let id = job.id.clone();
            move |db| {
                Ok(db.query_row(
                    "SELECT record FROM backup_snapshot_jobs WHERE id=?",
                    [id],
                    |r| r.get::<_, String>(0),
                )?)
            }
        })
        .unwrap();
    let record: JobRecord = decode(text).unwrap();
    let version = record
        .manifest
        .files
        .iter()
        .find(|v| v.root_id == first_root)
        .unwrap();
    let staged = record
        .work
        .unwrap()
        .path()
        .unwrap()
        .join(format!("payload-{}", job.id))
        .join(&version.archive);
    assert!(
        staged.is_file(),
        "Earlier staged source copies were silently removed."
    );
    assert!(
        f.fixture
            .library
            .snapshots
            .records(&f.fixture.library.snapshots.profile(&f.profile).unwrap())
            .unwrap()
            .is_empty()
    );
}

#[test]
#[ignore = "actual snapshot/scan concurrency with native source leases; generated fixtures only"]
fn native_snapshot_source_lease_admits_scan_and_refuses_organizer_writer() {
    let f = NativeFixture::new();
    let gate = Arc::new(Gate::default());
    let _release = Release(gate.clone());
    let hold = gate.clone();
    f.fixture.library.snapshots.state.lock().unwrap().test_phase =
        Some(Arc::new(move |_, _, phase| {
            if phase == "staging" {
                hold.enter();
            }
        }));
    let job = f.start(&f.review());
    gate.wait();
    let original = std::fs::read(f.fixture.source.join("original.txt")).unwrap();
    // This is the actual central source-write admission used by organizer
    // executors. Production mutation authority is never synthesized for a test.
    let writer = Registry::shared()
        .try_acquire(vec![Scope::new(
            DirectoryChain::open(&f.fixture.source).unwrap(),
            Access::Write,
        )])
        .err()
        .unwrap();
    assert_eq!(writer.code, ErrorCode::Busy);
    let scan = f
        .fixture
        .library
        .scan(
            f.fixture.root.clone(),
            super::super::model::ScanFilter::default(),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = f.fixture.library.status().unwrap();
        let current = status.jobs.iter().find(|j| j.id == scan.id).unwrap();
        if !matches!(current.state.as_str(), "queued" | "running" | "paused") {
            assert_eq!(current.state, "completed");
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read(f.fixture.source.join("original.txt")).unwrap(),
        original
    );
    // No source-tree additions or staged payloads were reconciled as assets.
    let actual = f.fixture.library.catalog.call({let root=f.fixture.root.clone();move|db|Ok(db.prepare("SELECT relative_native FROM assets WHERE root_id=? ORDER BY relative_native")?.query_map([root],|r|r.get::<_,Vec<u8>>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)}).unwrap().iter().map(|p|native::decode_path(p).unwrap()).collect::<BTreeSet<_>>();
    assert_eq!(
        actual,
        BTreeSet::from([
            PathBuf::from("original.txt"),
            PathBuf::from("hidden-source").join("companion.xmp")
        ])
    );
    gate.release();
    let done = wait(&f.fixture.library, &job.id);
    // Catalog scan observations can invalidate the reviewed current catalog,
    // but they must never authorize an unsafe overlapping writer or completion.
    assert_eq!(done.state, "needs_recovery", "{}", done.message);
    assert!(
        done.message
            .contains("Source scope changed before repository creation")
    );
    assert!(
        !done.included
            && !done.structural_checked
            && !done.data_read_verified
            && !done.restored_compared
            && !done.independent_recovery_verified
    );
    let lease = Registry::shared()
        .try_acquire(vec![Scope::new(
            DirectoryChain::open(&f.fixture.source).unwrap(),
            Access::Write,
        )])
        .unwrap();
    drop(lease);
    assert_eq!(
        std::fs::read(f.fixture.source.join("original.txt")).unwrap(),
        original
    );
}

#[test]
#[ignore = "actual native OS timing authority; generated fixtures only"]
fn native_timing_commit_failure_consumes_interval_and_legacy_requires_new_consent() {
    let f = NativeFixture::new();
    let s = &f.fixture.library.snapshots;
    let mut record = f.schedule(true);
    f.due(&mut record, chrono::Duration::hours(1));
    let old = f.authority(&record).unwrap();
    s.state.lock().unwrap().schedule_fail_after_commit = true;
    assert!(s.run_due(f.fixture.library.clone()).is_err());
    assert!(s.jobs(Some(&f.profile)).unwrap().is_empty());
    let advanced = f.authority(&record).unwrap();
    assert_ne!(advanced.generation, old.generation);
    assert!(advanced.deadline().unwrap() > chrono::Utc::now());
    assert!(advanced.last_attempt_utc.is_some());
    // This models an ambiguous native write/crash before catalog or worker
    // admission. The row is still old and can be replayed, never the OS deadline.
    s.persist_schedule(&record).unwrap();
    let cold = Arc::new(
        SnapshotWorkflow::new(f.fixture.library.catalog.clone(), s.data.path().unwrap()).unwrap(),
    );
    cold.configure_helpers(helper_root()).unwrap();
    let _stop = StopWorkflow(cold.clone(), f.retain.clone());
    let selection = Fixture::selection(&f.fixture.backup);
    cold.dispatch(
        f.fixture.library.clone(),
        SnapshotRequest::ProfileOpen {
            profile_id: f.profile.clone(),
            selection_id: Some(selection.id.clone()),
        },
        Some(selection),
    )
    .unwrap();
    cold.run_due(f.fixture.library.clone()).unwrap();
    assert!(cold.jobs(Some(&f.profile)).unwrap().is_empty());
    assert_eq!(
        cold.schedule_get(&f.profile).unwrap().unwrap().next_due,
        advanced.next_admissible_utc
    );
    let policy = SnapshotWorkflow::schedule_policy(&record).unwrap();
    let scope = ScheduleScope {
        catalog_id: &f.catalog,
        instance: &record.instance,
        profile_id: &f.profile,
        policy_digest: &policy,
    };
    schedule_transaction()
        .unwrap()
        .write_legacy_for_test(&scope)
        .unwrap();
    let legacy = cold.schedule_get(&f.profile).unwrap().unwrap();
    assert!(!legacy.enabled && legacy.next_due.is_empty() && legacy.last_run.is_none());
    cold.run_due(f.fixture.library.clone()).unwrap();
    assert!(cold.jobs(Some(&f.profile)).unwrap().is_empty());
    assert!(f.authority(&record).is_none());
    // Only an explicitly reviewed native enable creates SHS2, never a read.
    let renewed = f.schedule(true);
    assert!(f.authority(&renewed).is_some());
    assert!(s.schedule_get(&f.profile).unwrap().unwrap().enabled);
    assert!(s.jobs(Some(&f.profile)).unwrap().is_empty());
    assert_eq!(
        std::fs::read(f.fixture.source.join("original.txt")).unwrap(),
        b"generated preserved original"
    );
}

#[test]
#[ignore = "actual two-catalog native OS consent isolation; generated fixtures only"]
fn native_imported_catalog_disable_cannot_revoke_original_instance_authority() {
    let f = NativeFixture::new();
    let record = f.schedule(true);
    let authority = f.authority(&record).unwrap();
    let imported = f.fixture._run.path().join("Imported.stillharbour");
    f.fixture
        .library
        .catalog
        .snapshot(imported.clone())
        .unwrap();
    let data = f.fixture._run.path().join("imported-data");
    std::fs::create_dir(&data).unwrap();
    let data = data.canonicalize().unwrap();
    let library = Library::new(
        Arc::new(Catalog::open(imported).unwrap()),
        crate::library::LibraryOptions {
            data_directory: data.clone(),
            workers: crate::library::WorkerPaths {
                executable: data.join("missing-worker"),
                exiftool: None,
                working_directory: data.join("worker"),
            },
        },
    )
    .unwrap();
    assert_ne!(library.snapshots.instance().unwrap(), record.instance);
    let view = library.snapshots.schedule_get(&f.profile).unwrap().unwrap();
    assert!(!view.enabled && view.next_due.is_empty() && view.last_run.is_none());
    assert!(
        !library
            .snapshots
            .schedule_disable(&f.profile)
            .unwrap()
            .unwrap()
            .enabled
    );
    assert_eq!(f.authority(&record).unwrap(), authority);
    assert!(
        f.fixture
            .library
            .snapshots
            .schedule_get(&f.profile)
            .unwrap()
            .unwrap()
            .enabled
    );
    library.snapshots.cancel_all();
}
