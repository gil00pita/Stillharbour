//! Atomic admission for read-only source work. Never preempts an admitted step.
//! Reservations are estimates, not an RSS quota or an opaque codec byte-rate cap.
use crate::{
    domain::{AppError, ErrorCode, Result},
    performance::{self, Endpoint, Metric},
};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

const MAX_WAITERS: usize = 64;
const MAX_ACTIVE: usize = 2;
const MEMORY_MIB: u32 = 384;
const AGE: Duration = Duration::from_millis(500);
const CHECK: Duration = Duration::from_millis(20);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Interactive,
    Background,
}

#[derive(Clone, Copy)]
pub enum Work {
    Metadata,
    Rendition,
}
impl Work {
    fn memory_mib(self) -> u32 {
        match self {
            Self::Metadata => 64,
            Self::Rendition => 192,
        }
    }
}

struct Request {
    id: u64,
    volume: u64,
    class: Class,
    memory: u32,
    queued: Instant,
}
#[derive(Default)]
struct State {
    next: u64,
    waiting: Vec<Request>,
    active: Vec<Request>,
}
impl State {
    fn next_ready(&self, now: Instant) -> Option<usize> {
        let memory: u32 = self.active.iter().map(|r| r.memory).sum();
        if self.active.len() >= MAX_ACTIVE {
            return None;
        }
        self.waiting
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                memory + r.memory <= MEMORY_MIB
                    // Unknown topology (0) conflicts with every active source.
                    && !self.active.iter().any(|a| {
                        a.volume == r.volume || a.volume == 0 || r.volume == 0
                    })
            })
            .min_by_key(|(_, r)| {
                let rank = if now.saturating_duration_since(r.queued) >= AGE {
                    0
                } else if r.class == Class::Interactive {
                    1
                } else {
                    2
                };
                (rank, r.id)
            })
            .map(|(index, _)| index)
    }
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}
#[derive(Clone, Default)]
pub struct ResourceScheduler(Arc<Shared>);

/// Held through source reading, helper completion and validation/publication.
/// Drop also releases capacity after errors/panics; no filesystem action here.
pub struct Lease {
    owner: Arc<Shared>,
    id: u64,
}
impl ResourceScheduler {
    /// Blocking worker-thread call only. No queue lock is held during work.
    /// A caller canceled while queued never starts work. The returned lease does
    /// not stop running helpers; their existing supervision owns cancellation.
    pub fn acquire(
        &self,
        volume: u64,
        class: Class,
        work: Work,
        cancel: &AtomicBool,
    ) -> Result<Lease> {
        let started = performance::clock();
        let mut state = self.0.state.lock().map_err(|_| unavailable())?;
        if cancel.load(Ordering::Relaxed) {
            return Err(canceled());
        }
        if state.waiting.len() >= MAX_WAITERS {
            return Err(AppError::new(ErrorCode::Busy, "Source work queue is full."));
        }
        state.next = state.next.checked_add(1).ok_or_else(unavailable)?;
        let id = state.next;
        state.waiting.push(Request {
            id,
            volume,
            class,
            memory: work.memory_mib(),
            queued: Instant::now(),
        });
        loop {
            if cancel.load(Ordering::Relaxed) {
                state.waiting.retain(|r| r.id != id);
                self.0.changed.notify_all();
                return Err(canceled());
            }
            if let Some(index) = state.next_ready(Instant::now())
                && state.waiting[index].id == id
            {
                let request = state.waiting.remove(index);
                state.active.push(request);
                self.0.changed.notify_all();
                drop(state);
                performance::since(
                    match class {
                        Class::Interactive => Metric::ResourceWaitInteractive,
                        Class::Background => Metric::ResourceWaitBackground,
                    },
                    Endpoint::Worker,
                    started,
                    0,
                );
                return Ok(Lease {
                    owner: self.0.clone(),
                    id,
                });
            }
            state = self
                .0
                .changed
                .wait_timeout(state, CHECK)
                .map_err(|_| unavailable())?
                .0;
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        // A poisoned scheduler remains failed closed for new admissions, but
        // release this lease so its accounting never leaks on unwind.
        let mut state = self.owner.state.lock().unwrap_or_else(|e| e.into_inner());
        state.active.retain(|r| r.id != self.id);
        self.owner.changed.notify_all();
    }
}
fn unavailable() -> AppError {
    AppError::new(ErrorCode::WorkerFailed, "Source scheduler is unavailable.")
}
fn canceled() -> AppError {
    AppError::new(ErrorCode::Busy, "Source work canceled before admission.")
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
