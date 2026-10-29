use super::*;
use std::{sync::mpsc, thread};

fn acquire(s: &ResourceScheduler, volume: u64) -> Lease {
    s.acquire(
        volume,
        Class::Background,
        Work::Rendition,
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn queued(s: &ResourceScheduler, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while s.0.state.lock().unwrap().waiting.len() != count {
        assert!(Instant::now() < deadline, "waiter did not reach admission");
        thread::yield_now();
    }
}
#[test]
fn same_volume_serializes_and_interactive_overtakes_queued_background() {
    let scheduler = ResourceScheduler::default();
    let held = acquire(&scheduler, 7);
    let (started, receiver) = mpsc::channel();
    let mut threads = Vec::new();
    for class in [Class::Background, Class::Interactive] {
        let worker = scheduler.clone();
        let started = started.clone();
        threads.push(thread::spawn(move || {
            let _lease = worker
                .acquire(7, class, Work::Metadata, &AtomicBool::new(false))
                .unwrap();
            started.send(class).unwrap();
        }));
        queued(&scheduler, threads.len());
    }
    assert!(receiver.try_recv().is_err());
    for request in &mut scheduler.0.state.lock().unwrap().waiting {
        request.queued = Instant::now();
    }
    drop(held);
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
        Class::Interactive
    );
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
        Class::Background
    );
    for thread in threads {
        thread.join().unwrap();
    }
    let state = scheduler.0.state.lock().unwrap();
    assert!(state.active.is_empty() && state.waiting.is_empty());
}
#[test]
fn different_volumes_share_global_limits_and_cancel_without_leaking_admission() {
    let scheduler = ResourceScheduler::default();
    let a = acquire(&scheduler, 1);
    let b = acquire(&scheduler, 2);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let worker = scheduler.clone();
    let pending = thread::spawn(move || {
        worker
            .acquire(3, Class::Interactive, Work::Metadata, &worker_cancel)
            .err()
            .unwrap()
            .code
    });
    queued(&scheduler, 1);
    assert_eq!(scheduler.0.state.lock().unwrap().active.len(), 2);
    cancel.store(true, Ordering::Relaxed);
    assert_eq!(pending.join().unwrap(), ErrorCode::Busy);
    queued(&scheduler, 0);
    drop((a, b));
    let _retry = acquire(&scheduler, 3);
}
#[test]
fn aging_fifo_memory_and_unknown_topology_are_deterministic() {
    let now = Instant::now();
    let request = |id, volume, class, queued| Request {
        id,
        volume,
        class,
        queued,
        memory: 192,
    };
    let mut state = State::default();
    state.waiting.push(request(1, 1, Class::Background, now));
    state.waiting.push(request(2, 1, Class::Interactive, now));
    assert_eq!(state.next_ready(now), Some(1));
    assert_eq!(state.next_ready(now + AGE), Some(0));
    state.active.push(request(3, 0, Class::Background, now));
    assert_eq!(state.next_ready(now), None);
    state.active[0].volume = 2;
    state.active[0].memory = MEMORY_MIB - 191;
    assert_eq!(state.next_ready(now), None);
    state.active[0].memory = MEMORY_MIB - 192;
    assert_eq!(state.next_ready(now), Some(1));
    state.waiting[1].volume = 0;
    assert_eq!(state.next_ready(now), Some(0));
}
#[test]
fn unrelated_volume_is_not_blocked_by_a_waiter_on_a_busy_volume() {
    let scheduler = ResourceScheduler::default();
    let held = acquire(&scheduler, 1);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker = scheduler.clone();
    let worker_cancel = cancel.clone();
    let pending = thread::spawn(move || {
        worker
            .acquire(1, Class::Interactive, Work::Metadata, &worker_cancel)
            .err()
            .unwrap()
            .code
    });
    queued(&scheduler, 1);
    let unrelated = acquire(&scheduler, 2);
    cancel.store(true, Ordering::Relaxed);
    assert_eq!(pending.join().unwrap(), ErrorCode::Busy);
    drop((held, unrelated));
}
#[test]
fn queue_full_and_pre_canceled_requests_do_not_change_accounting() {
    let scheduler = ResourceScheduler::default();
    let held = acquire(&scheduler, 1);
    let mut state = scheduler.0.state.lock().unwrap();
    for id in 1..=MAX_WAITERS as u64 {
        state.waiting.push(Request {
            id,
            volume: 1,
            class: Class::Background,
            memory: 64,
            queued: Instant::now(),
        });
    }
    drop(state);
    assert_eq!(
        scheduler
            .acquire(
                1,
                Class::Background,
                Work::Metadata,
                &AtomicBool::new(false)
            )
            .err()
            .unwrap()
            .code,
        ErrorCode::Busy
    );
    assert_eq!(
        scheduler
            .acquire(
                1,
                Class::Interactive,
                Work::Metadata,
                &AtomicBool::new(true)
            )
            .err()
            .unwrap()
            .code,
        ErrorCode::Busy
    );
    assert_eq!(scheduler.0.state.lock().unwrap().waiting.len(), MAX_WAITERS);
    drop(held);
}
#[test]
fn unwind_releases_capacity_and_never_interrupts_an_admitted_lease() {
    let scheduler = ResourceScheduler::default();
    let cancel = AtomicBool::new(false);
    let lease = scheduler
        .acquire(1, Class::Background, Work::Metadata, &cancel)
        .unwrap();
    cancel.store(true, Ordering::Relaxed);
    assert_eq!(scheduler.0.state.lock().unwrap().active.len(), 1);
    drop(lease);
    assert!(
        std::panic::catch_unwind(|| {
            let _lease = acquire(&scheduler, 1);
            panic!("fixture worker failure");
        })
        .is_err()
    );
    let _recovered = acquire(&scheduler, 1);
}
