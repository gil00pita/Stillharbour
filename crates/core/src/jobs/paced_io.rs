//! One aggregate application-issued byte budget for comparison, copy and
//! verification. Charge before I/O, including read-back. This does not cap
//! physical disk traffic, filesystem metadata or opaque helper processes.
use crate::domain::{AppError, ErrorCode, Result};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub const CHUNK_BYTES: usize = 65_536;
/// One token bucket burst plus one already admitted, possibly delayed syscall.
pub const ISSUED_BURST_BYTES: usize = CHUNK_BYTES * 2;
pub const DEFAULT_BYTES_PER_SECOND: u64 = 10 * 1024 * 1024;
const NANOS: u128 = 1_000_000_000;
const MAX_WAITERS: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub enum IoStep<T> {
    Completed(T),
    Canceled,
}

struct Bucket {
    rate: u64,
    credit: u128,
    updated: u128,
}
impl Bucket {
    fn new(rate: u64) -> Self {
        Self {
            rate,
            credit: CHUNK_BYTES as u128 * NANOS,
            updated: 0,
        }
    }
    fn refill(&mut self, now: u128) {
        let elapsed = now.saturating_sub(self.updated);
        self.updated = self.updated.max(now);
        self.credit = self
            .credit
            .saturating_add(elapsed.saturating_mul(u128::from(self.rate)))
            .min(CHUNK_BYTES as u128 * NANOS);
    }
    fn take(&mut self, now: u128, bytes: usize) -> Option<Duration> {
        self.refill(now);
        let requested = bytes as u128 * NANOS;
        if self.credit >= requested {
            self.credit -= requested;
            None
        } else {
            let nanos = (requested - self.credit).div_ceil(u128::from(self.rate));
            Some(Duration::from_nanos(
                u64::try_from(nanos).unwrap_or(u64::MAX),
            ))
        }
    }
    fn rate(&mut self, now: u128, rate: u64) {
        self.refill(now);
        self.rate = rate;
    }
}
struct State {
    bucket: Bucket,
    paused: bool,
    queue: VecDeque<u64>,
    next_ticket: u64,
    in_flight: bool,
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    start: Instant,
}

/// Clone this controller across all owned operation streams. A single global
/// aggregate budget is conservative for unknown/shared device topology.
#[derive(Clone)]
pub struct PacedIo(Arc<Shared>);
impl Default for PacedIo {
    fn default() -> Self {
        Self::new(DEFAULT_BYTES_PER_SECOND).expect("The fixed default rate is valid")
    }
}
fn failed() -> AppError {
    AppError::new(
        ErrorCode::WorkerFailed,
        "The owned I/O budget is unavailable; the operation must stop.",
    )
}
fn valid_rate(rate: u64) -> Result<()> {
    if !(1024..=1024 * 1024 * 1024).contains(&rate) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Owned I/O rate must be between 1 KiB/s and 1 GiB/s. Pause is a separate control.",
        ));
    }
    Ok(())
}
struct Waiting(Arc<Shared>, u64);
impl Drop for Waiting {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            if let Some(index) = state.queue.iter().position(|ticket| *ticket == self.1) {
                state.queue.remove(index);
            }
            self.0.changed.notify_all();
        }
    }
}
struct Admission(Arc<Shared>);
impl Drop for Admission {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.in_flight = false;
            self.0.changed.notify_all();
        }
    }
}
impl PacedIo {
    pub fn shared() -> Self {
        static OWNED_IO: std::sync::OnceLock<PacedIo> = std::sync::OnceLock::new();
        OWNED_IO.get_or_init(Self::default).clone()
    }
    pub fn new(bytes_per_second: u64) -> Result<Self> {
        valid_rate(bytes_per_second)?;
        Ok(Self(Arc::new(Shared {
            state: Mutex::new(State {
                bucket: Bucket::new(bytes_per_second),
                paused: false,
                queue: VecDeque::new(),
                next_ticket: 0,
                in_flight: false,
            }),
            changed: Condvar::new(),
            start: Instant::now(),
        })))
    }
    pub fn set_rate(&self, bytes_per_second: u64) -> Result<()> {
        valid_rate(bytes_per_second)?;
        let mut state = self.0.state.lock().map_err(|_| failed())?;
        state
            .bucket
            .rate(self.0.start.elapsed().as_nanos(), bytes_per_second);
        self.0.changed.notify_all();
        Ok(())
    }
    pub fn bytes_per_second(&self) -> Result<u64> {
        Ok(self.0.state.lock().map_err(|_| failed())?.bucket.rate)
    }
    /// Does not interrupt an admitted syscall. Resume occurs at a byte boundary.
    pub fn set_paused(&self, paused: bool) -> Result<()> {
        let mut state = self.0.state.lock().map_err(|_| failed())?;
        state.paused = paused;
        self.0.changed.notify_all();
        Ok(())
    }
    fn acquire(&self, bytes: usize, cancel: &AtomicBool) -> Result<IoStep<Admission>> {
        if bytes == 0 || bytes > CHUNK_BYTES {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Owned I/O requests must be 1–65,536 bytes.",
            ));
        }
        if cancel.load(Ordering::Acquire) {
            return Ok(IoStep::Canceled);
        }
        let ticket = {
            let mut state = self.0.state.lock().map_err(|_| failed())?;
            if state.queue.len() >= MAX_WAITERS {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "The bounded owned I/O queue is full.",
                ));
            }
            let ticket = state.next_ticket;
            state.next_ticket = state.next_ticket.checked_add(1).ok_or_else(failed)?;
            state.queue.push_back(ticket);
            ticket
        };
        let _waiting = Waiting(self.0.clone(), ticket);
        self.wait_for_budget(bytes, cancel, ticket)
    }
    fn wait_for_budget(
        &self,
        bytes: usize,
        cancel: &AtomicBool,
        ticket: u64,
    ) -> Result<IoStep<Admission>> {
        let mut state = self.0.state.lock().map_err(|_| failed())?;
        loop {
            if cancel.load(Ordering::Acquire) {
                return Ok(IoStep::Canceled);
            }
            let wait = if state.paused || state.in_flight || state.queue.front() != Some(&ticket) {
                Duration::from_millis(20)
            } else {
                match state.bucket.take(self.0.start.elapsed().as_nanos(), bytes) {
                    None => {
                        state.in_flight = true;
                        return Ok(IoStep::Completed(Admission(self.0.clone())));
                    }
                    Some(wait) => wait.min(Duration::from_millis(20)),
                }
            };
            state = self
                .0
                .changed
                .wait_timeout(state, wait)
                .map_err(|_| failed())?
                .0;
        }
    }
    /// The requested length is charged even on EOF/short reads/errors; unused
    /// budget is never refunded into a burst for a concurrent operation.
    pub fn read(
        &self,
        source: &mut impl Read,
        buffer: &mut [u8],
        cancel: &AtomicBool,
    ) -> Result<IoStep<usize>> {
        let IoStep::Completed(_admission) = self.acquire(buffer.len(), cancel)? else {
            return Ok(IoStep::Canceled);
        };
        Ok(IoStep::Completed(source.read(buffer)?))
    }
    pub fn write_all(
        &self,
        output: &mut impl Write,
        buffer: &[u8],
        cancel: &AtomicBool,
    ) -> Result<IoStep<()>> {
        let IoStep::Completed(_admission) = self.acquire(buffer.len(), cancel)? else {
            return Ok(IoStep::Canceled);
        };
        output.write_all(buffer)?;
        Ok(IoStep::Completed(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_full_queue_rejects_more_work_without_entering_io_and_honors_prior_cancellation() {
        let io = PacedIo::default();
        {
            let mut state = io.0.state.lock().unwrap();
            state.queue = (0..MAX_WAITERS as u64).collect();
            state.next_ticket = MAX_WAITERS as u64;
        }
        let mut source = std::io::Cursor::new(b"unchanged");
        assert_eq!(
            io.read(&mut source, &mut [0u8; 1], &AtomicBool::new(false))
                .unwrap_err()
                .code,
            ErrorCode::Busy
        );
        assert_eq!(source.position(), 0);
        assert_eq!(
            io.read(&mut source, &mut [0u8; 1], &AtomicBool::new(true))
                .unwrap(),
            IoStep::Canceled
        );
        assert_eq!(source.position(), 0);
        assert_eq!(io.0.state.lock().unwrap().queue.len(), MAX_WAITERS);
    }
    fn queued(io: &PacedIo, count: usize) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if io.0.state.lock().unwrap().queue.len() == count {
                return true;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        false
    }

    #[test]
    fn queued_streams_enter_in_order_and_release_admission_after_io_error() {
        struct Ordered {
            id: u8,
            order: Arc<Mutex<Vec<u8>>>,
        }
        impl Read for Ordered {
            fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
                self.order.lock().unwrap().push(self.id);
                bytes[0] = self.id;
                Ok(1)
            }
        }
        let io = PacedIo::default();
        io.set_paused(true).unwrap();
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut workers = Vec::new();
        let mut registered = true;
        for id in 0..2u8 {
            let child = io.clone();
            let order = order.clone();
            workers.push(std::thread::spawn(move || {
                child.read(
                    &mut Ordered { id, order },
                    &mut [0u8; 1],
                    &AtomicBool::new(false),
                )
            }));
            registered &= queued(&io, usize::from(id) + 1);
        }
        io.set_paused(false).unwrap();
        for worker in workers {
            assert_eq!(worker.join().unwrap().unwrap(), IoStep::Completed(1));
        }
        assert!(registered);
        assert_eq!(*order.lock().unwrap(), [0, 1]);
        struct Denied;
        impl Read for Denied {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::PermissionDenied.into())
            }
        }
        assert!(
            io.read(&mut Denied, &mut [0u8; 1], &AtomicBool::new(false))
                .is_err()
        );
        assert_eq!(
            io.read(
                &mut std::io::Cursor::new(b"x"),
                &mut [0u8; 1],
                &AtomicBool::new(false)
            )
            .unwrap(),
            IoStep::Completed(1)
        );
        let state = io.0.state.lock().unwrap();
        assert!(!state.in_flight);
        assert!(state.queue.is_empty());
    }

    #[test]
    fn a_stalled_syscall_does_not_admit_more_io_but_queued_cancellation_still_completes() {
        use std::sync::mpsc;
        struct Stalled {
            started: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
        }
        impl Read for Stalled {
            fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
                self.started.send(()).unwrap();
                self.release.recv().unwrap();
                bytes[0] = 1;
                Ok(1)
            }
        }
        struct ObservedWrite(Arc<AtomicBool>);
        impl Write for ObservedWrite {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.store(true, Ordering::Release);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let io = PacedIo::default();
        let child = io.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let first = std::thread::spawn(move || {
            child.read(
                &mut Stalled {
                    started: started_tx,
                    release: release_rx,
                },
                &mut [0u8; 1],
                &AtomicBool::new(false),
            )
        });
        let started = started_rx.recv_timeout(Duration::from_secs(5)).is_ok();
        let child = io.clone();
        let called = Arc::new(AtomicBool::new(false));
        let observed = called.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let token = cancel.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let second = std::thread::spawn(move || {
            let result = child.write_all(&mut ObservedWrite(observed), b"second", &token);
            let _ = done_tx.send(matches!(result, Ok(IoStep::Canceled)));
            result
        });
        let registered = queued(&io, 1);
        cancel.store(true, Ordering::Release);
        let canceled_before_release = done_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or(false);
        let _ = release_tx.send(());
        assert_eq!(first.join().unwrap().unwrap(), IoStep::Completed(1));
        assert_eq!(second.join().unwrap().unwrap(), IoStep::Canceled);
        assert!(started && registered && canceled_before_release);
        assert!(!called.load(Ordering::Acquire));
        assert_eq!(ISSUED_BURST_BYTES, 2 * CHUNK_BYTES);
    }
    #[test]
    fn mixed_concurrent_stream_requests_obey_one_precharged_rate_and_burst_envelope() {
        let rate = 10_000u64;
        let mut bucket = Bucket::new(rate);
        let mut now = 0u128;
        let mut issued = 0u128;
        let mut observations = vec![(now, issued)];
        // Interleave comparison/copy/verification-sized requests on the same
        // clock. No syscall is issued until its complete charge is admitted.
        for index in 0..2_000usize {
            let bytes = 1 + (index * 7919) % CHUNK_BYTES;
            loop {
                match bucket.take(now, bytes) {
                    None => {
                        issued += bytes as u128;
                        break;
                    }
                    Some(wait) => {
                        assert!(
                            issued * NANOS <= u128::from(rate) * now + CHUNK_BYTES as u128 * NANOS
                        );
                        now += wait.as_nanos();
                    }
                }
            }
            assert!(issued * NANOS <= u128::from(rate) * now + CHUNK_BYTES as u128 * NANOS);
            for (before, previous) in observations.iter().rev().take(64) {
                assert!(
                    (issued - previous) * NANOS
                        <= u128::from(rate) * (now - before) + CHUNK_BYTES as u128 * NANOS
                );
            }
            observations.push((now, issued));
        }
        assert!(bucket.take(now, CHUNK_BYTES).is_some());
    }
    #[test]
    fn live_rate_changes_refill_at_the_previous_rate_without_minting_new_credit() {
        let mut bucket = Bucket::new(1024);
        assert!(bucket.take(0, CHUNK_BYTES).is_none());
        bucket.rate(NANOS / 2, 2048);
        assert_eq!(bucket.credit, 512 * NANOS);
        assert!(bucket.take(NANOS / 2, 512).is_none());
        assert_eq!(
            bucket.take(NANOS / 2, 1024),
            Some(Duration::from_millis(500))
        );
        assert!(bucket.take(NANOS, 1024).is_none());
        bucket.rate(NANOS, 1024);
        assert_eq!(bucket.credit, 0);
    }
    #[test]
    fn cancellation_before_io_and_during_pause_never_enters_the_underlying_stream() {
        struct Never;
        impl Read for Never {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                panic!("canceled source was read")
            }
        }
        impl Write for Never {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                panic!("canceled output was written")
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let io = PacedIo::default();
        let cancel = AtomicBool::new(true);
        assert_eq!(
            io.read(&mut Never, &mut [0u8; 1], &cancel).unwrap(),
            IoStep::Canceled
        );
        assert_eq!(
            io.write_all(&mut Never, b"x", &cancel).unwrap(),
            IoStep::Canceled
        );
        assert!(io.read(&mut Never, &mut [], &cancel).is_err());
        io.set_paused(true).unwrap();
        let child = io.clone();
        let cancellation = Arc::new(AtomicBool::new(false));
        let token = cancellation.clone();
        let thread = std::thread::spawn(move || child.read(&mut Never, &mut [0u8; 1], &token));
        cancellation.store(true, Ordering::Release);
        assert_eq!(thread.join().unwrap().unwrap(), IoStep::Canceled);
        assert!(io.0.state.lock().unwrap().queue.is_empty());
        assert!(io.set_rate(0).is_err());
    }
}
