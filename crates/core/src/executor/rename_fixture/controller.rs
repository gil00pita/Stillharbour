//! One generated scope, one active worker, no unbounded pending queue. Readers
//! query the catalog independently of the session lock held by native work.
use super::*;
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode},
    library::original_jobs,
    operations::generated_rename::*,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
struct Activity {
    job: Option<String>,
    control: Arc<RunControl>,
    total: u32,
    error: Option<AppError>,
}
#[derive(Default)]
struct Inner {
    // Drop this reader before the session's actor and owned temporary files.
    reader: Mutex<Option<Arc<Catalog>>>,
    session: Mutex<Option<GeneratedRenameSession>>,
    retained: Mutex<Option<super::session::recovery_scope::Retained>>,
    scope_created: AtomicBool,
    activity: Mutex<Activity>,
    busy: AtomicBool,
    #[cfg(feature = "fixture-probes")]
    hold_first: AtomicBool,
    #[cfg(test)]
    step_gate: Option<(usize, Arc<std::sync::Barrier>)>,
}
#[derive(Clone, Default)]
pub struct GeneratedRenameController {
    inner: Arc<Inner>,
}
fn busy() -> AppError {
    AppError::new(
        ErrorCode::Busy,
        "The generated worker is active. Read job status or request a stop; inspect after it stops.",
    )
}
impl GeneratedRenameController {
    fn reader(&self) -> Result<Option<Arc<Catalog>>> {
        Ok(self
            .inner
            .reader
            .lock()
            .map_err(|_| failed("Generated catalog reader unavailable."))?
            .clone())
    }
    fn reopen_retained(&self) -> Result<GeneratedRenameReview> {
        if self
            .inner
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(busy());
        }
        let result: Result<GeneratedRenameReview> = (|| {
            let mut slot = self.inner.session.try_lock().map_err(|_| busy())?;
            let mut retained = self
                .inner
                .retained
                .lock()
                .map_err(|_| failed("Retained generated scope unavailable."))?;
            if let Some(session) = slot.as_mut() {
                let owner = super::session::recovery_scope::Retained::capture(session)?;
                *retained = Some(owner);
                *self
                    .inner
                    .reader
                    .lock()
                    .map_err(|_| failed("Generated catalog reader unavailable."))? = None;
                // Only the owned directory capability survives this drop. No
                // old session, graph, catalog actor or approval supplies state.
                *slot = None;
                *self
                    .inner
                    .activity
                    .lock()
                    .map_err(|_| failed("Generated worker state unavailable."))? =
                    Activity::default();
                #[cfg(feature = "fixture-probes")]
                self.inner.hold_first.store(false, Ordering::Release);
            }
            let mut session = retained
                .as_ref()
                .ok_or_else(|| failed("Create a generated scope before reopening it."))?
                .open()?;
            let review = session.request(GeneratedRenameRequest::Inspect {})?;
            *self
                .inner
                .reader
                .lock()
                .map_err(|_| failed("Generated catalog reader unavailable."))? =
                Some(session.reader());
            *slot = Some(session);
            Ok(review)
        })();
        if let Err(error) = &result
            && let Ok(mut activity) = self.inner.activity.lock()
        {
            activity.error = Some(error.clone());
        }
        self.inner.busy.store(false, Ordering::Release);
        result
    }
    /// Catalog-only history from this controller's constructor-owned scope.
    /// It does not inspect files, grant approval, or resume a worker.
    pub fn history(
        &self,
        job_id: String,
    ) -> Result<crate::operations::original_jobs::OriginalOperationHistory> {
        let catalog = self
            .reader()?
            .ok_or_else(|| failed("Create the generated scope before reviewing its history."))?;
        crate::library::original_history::review(&catalog, job_id)
    }
    pub fn request(&self, request: GeneratedRenameRequest) -> Result<GeneratedRenameReview> {
        if matches!(request, GeneratedRenameRequest::ReopenRetained {}) {
            return self.reopen_retained();
        }
        if let GeneratedRenameRequest::Start { plan_id, ticket } = request {
            return self.start(plan_id, ticket);
        }
        if self.inner.busy.load(Ordering::Acquire) {
            return Err(busy());
        }
        let mut slot = self.inner.session.try_lock().map_err(|_| busy())?;
        if self.inner.busy.load(Ordering::Acquire) {
            return Err(busy());
        }
        if matches!(request, GeneratedRenameRequest::Create {}) {
            if slot.is_some()
                || self
                    .inner
                    .retained
                    .lock()
                    .map_err(|_| failed("Retained scope unavailable."))?
                    .is_some()
            {
                return Err(failed(
                    "Inspect the existing generated scope instead of replacing it.",
                ));
            }
            let session = GeneratedRenameSession::create()?;
            *self
                .inner
                .reader
                .lock()
                .map_err(|_| failed("Generated catalog reader unavailable."))? =
                Some(session.reader());
            *slot = Some(session);
            self.inner.scope_created.store(true, Ordering::Release);
            return slot
                .as_mut()
                .ok_or_else(|| failed("Generated scope unavailable."))?
                .request(GeneratedRenameRequest::Inspect {});
        }
        let session = slot
            .as_mut()
            .ok_or_else(|| failed("Create or reopen the owned generated scope first."))?;
        if matches!(request, GeneratedRenameRequest::Inspect {}) {
            original_jobs::recover(&session.reader())?;
        }
        session.request(request)
    }
    fn start(&self, plan: String, ticket: String) -> Result<GeneratedRenameReview> {
        if self
            .inner
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(busy());
        }
        // A prior job's stop handle must never be presented as the new worker
        // while its queued record is still being prepared.
        match self.inner.activity.lock() {
            Ok(mut activity) => *activity = Activity::default(),
            Err(_) => {
                self.inner.busy.store(false, Ordering::Release);
                return Err(failed("Generated worker state unavailable."));
            }
        }
        let prepared = (|| {
            let slot = self.inner.session.try_lock().map_err(|_| busy())?;
            let session = slot
                .as_ref()
                .ok_or_else(|| failed("Create the generated scope first."))?;
            let (job, view) = session.prepare_start(plan, ticket)?;
            let control = Arc::new(RunControl {
                completed: std::sync::atomic::AtomicUsize::new(view.completed_steps as usize),
                #[cfg(feature = "fixture-probes")]
                hold_first: self.inner.hold_first.swap(false, Ordering::AcqRel),
                #[cfg(test)]
                after_step: self.inner.step_gate.clone(),
                ..RunControl::default()
            });
            *self
                .inner
                .activity
                .lock()
                .map_err(|_| failed("Generated worker state unavailable."))? = Activity {
                job: Some(job.id.clone()),
                control: control.clone(),
                total: view.total_steps,
                error: None,
            };
            Ok((job.id, view, control))
        })();
        let (job, view, control) = match prepared {
            Ok(value) => value,
            Err(error) => {
                self.inner.busy.store(false, Ordering::Release);
                return Err(error);
            }
        };
        let inner = self.inner.clone();
        let worker_job = job.clone();
        let spawned=std::thread::Builder::new().name("generated-original-worker".into()).spawn(move || {
            // Keep the guard outside catch_unwind, so a worker panic does not
            // poison the session lock. Every subsequent inspection revalidates
            // native/journal/catalog facts before accepting another approval.
            let result=match inner.session.lock() {
                Ok(slot)=>match slot.as_ref() {
                    Some(session)=>match std::panic::catch_unwind(std::panic::AssertUnwindSafe(||session.run_job(worker_job.clone(),Some(&control)))) {
                        Ok(result)=>result,
                        Err(_)=>Err(session.record_stopped(&worker_job,failed("The generated worker stopped unexpectedly. Inspect retained state; no automatic replay occurred."))),
                    },
                    None=>Err(failed("Generated worker lost its owned scope.")),
                },
                Err(_)=>Err(failed("Generated worker scope lock is unavailable.")),
            };
            if let Ok(mut activity)=inner.activity.lock() { activity.error=result.err(); }
            inner.busy.store(false,Ordering::Release);
        });
        if let Err(error) = spawned {
            let slot = match self.inner.session.lock() {
                Ok(slot) => slot,
                Err(_) => {
                    self.inner.busy.store(false, Ordering::Release);
                    return Err(failed("Worker launch and scope lock failed."));
                }
            };
            let error = failed("The generated worker could not start. No native movement began.")
                .caused_by(error);
            let error = slot
                .as_ref()
                .map_or(error.clone(), |session| session.record_stopped(&job, error));
            if let Ok(mut activity) = self.inner.activity.lock() {
                activity.error = Some(error.clone());
            }
            self.inner.busy.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(view)
    }
    pub fn worker(&self, request: GeneratedWorkerRequest) -> Result<GeneratedWorkerStatus> {
        if let GeneratedWorkerRequest::Stop { job_id } = request {
            crate::operations::validate_id(&job_id)?;
            let activity = self
                .inner
                .activity
                .lock()
                .map_err(|_| failed("Generated worker state unavailable."))?;
            if !self.inner.busy.load(Ordering::Acquire) || activity.job.as_ref() != Some(&job_id) {
                return Err(failed(
                    "Only the currently active generated job can be stopped.",
                ));
            }
            activity.control.stop.store(true, Ordering::Release);
        }
        let (job, control, total, error) = {
            let activity = self
                .inner
                .activity
                .lock()
                .map_err(|_| failed("Generated worker state unavailable."))?;
            (
                activity.job.clone(),
                activity.control.clone(),
                activity.total,
                activity.error.clone(),
            )
        };
        let reader = self.reader()?;
        let job = match (&reader, job) {
            (Some(reader), Some(id)) => Some(original_jobs::load(reader, id)?),
            _ => None,
        };
        Ok(GeneratedWorkerStatus {
            scope_created: self.inner.scope_created.load(Ordering::Acquire),
            busy: self.inner.busy.load(Ordering::Acquire),
            stop_requested: control.stop.load(Ordering::Acquire),
            completed_moves: control.completed.load(Ordering::Acquire) as u32,
            total_moves: total,
            job,
            error,
        })
    }
    /// Instrumented fixture only. No path, index or user library is accepted.
    #[cfg(feature = "fixture-probes")]
    pub fn arm_native_hold(&self) -> Result<()> {
        if self.inner.busy.load(Ordering::Acquire) || self.reader()?.is_none() {
            return Err(failed(
                "Create an idle generated scope before arming its bounded hold.",
            ));
        }
        if self.inner.hold_first.swap(true, Ordering::AcqRel) {
            return Err(failed("A generated hold is already armed."));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::original_jobs::OriginalJobState;
    fn approve(controller: &GeneratedRenameController) -> GeneratedRenameReview {
        let view = controller
            .request(GeneratedRenameRequest::Inspect {})
            .unwrap();
        controller
            .request(GeneratedRenameRequest::Approve {
                plan_id: view.plan_id,
                acknowledged: true,
            })
            .unwrap()
    }
    fn start(
        controller: &GeneratedRenameController,
        view: GeneratedRenameReview,
    ) -> Result<GeneratedRenameReview> {
        controller.request(GeneratedRenameRequest::Start {
            plan_id: view.plan_id,
            ticket: view.ticket.unwrap(),
        })
    }
    fn wait(
        controller: &GeneratedRenameController,
        condition: impl Fn(&GeneratedWorkerStatus) -> bool,
    ) -> GeneratedWorkerStatus {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let status = controller
                .worker(GeneratedWorkerRequest::Status {})
                .unwrap();
            if condition(&status) {
                return status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Generated worker did not reach its bounded condition: busy={}, moves={}, state={:?}, error={:?}",
                status.busy,
                status.completed_moves,
                status.job.as_ref().map(|job| job.state),
                status.error
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    #[test]
    fn generated_controller_bounds_queue_stops_at_a_durable_step_and_requires_fresh_approval() {
        // Match the desktop's independent process budget. Other native fault
        // matrices intentionally saturate the parent's shared I/O limiter;
        // their pacing is not a worker liveness deadline for this scenario.
        if crate::test_process::isolated(
            "executor::rename_fixture::controller::tests::generated_controller_bounds_queue_stops_at_a_durable_step_and_requires_fresh_approval",
            45,
        ) {
            return;
        }
        let gate = Arc::new(std::sync::Barrier::new(2));
        let controller = GeneratedRenameController {
            inner: Arc::new(Inner {
                step_gate: Some((1, gate.clone())),
                ..Inner::default()
            }),
        };
        controller
            .request(GeneratedRenameRequest::Create {})
            .unwrap();
        let approved = approve(&controller);
        let queued = start(&controller, approved.clone()).unwrap();
        let status = wait(&controller, |status| status.completed_moves == 1);
        let job = status.job.unwrap();
        assert_eq!(job.state, OriginalJobState::Running);
        assert_eq!(queued.completed_steps, 0);
        assert!(
            controller
                .request(GeneratedRenameRequest::Inspect {})
                .is_err()
        );
        assert!(start(&controller, approved.clone()).is_err());
        assert!(
            controller
                .request(GeneratedRenameRequest::ReopenRetained {})
                .is_err()
        );
        assert!(
            controller
                .worker(GeneratedWorkerRequest::Stop { job_id: uuid() })
                .is_err()
        );
        let stopped = controller
            .worker(GeneratedWorkerRequest::Stop { job_id: job.id })
            .unwrap();
        assert!(stopped.busy && stopped.stop_requested);
        gate.wait();
        let stopped = wait(&controller, |status| !status.busy);
        assert_eq!(stopped.completed_moves, 1);
        assert_eq!(stopped.job.unwrap().state, OriginalJobState::NeedsRecovery);
        let inspected = controller
            .request(GeneratedRenameRequest::Inspect {})
            .unwrap();
        assert_eq!(inspected.completed_steps, 1);
        assert_eq!(inspected.state, GeneratedRenameState::NeedsContinuation);
        let reopened = controller
            .request(GeneratedRenameRequest::ReopenRetained {})
            .unwrap();
        assert_eq!(reopened.plan_id, inspected.plan_id);
        assert_eq!(reopened.digest, inspected.digest);
        assert_eq!(reopened.completed_steps, 1);
        assert!(reopened.ticket.is_none());
        assert_eq!(
            serde_json::to_vec(&reopened.members).unwrap(),
            serde_json::to_vec(&inspected.members).unwrap()
        );
        assert!(start(&controller, approved).is_err());
        let fresh = approve(&controller);
        start(&controller, fresh).unwrap();
        let done = wait(&controller, |status| !status.busy);
        assert_eq!(done.job.unwrap().state, OriginalJobState::CompletedVerified);
        let done = controller
            .request(GeneratedRenameRequest::Inspect {})
            .unwrap();
        assert_eq!(done.state, GeneratedRenameState::Completed);
        assert!(
            done.members.iter().all(
                |member| member.catalog_path == member.after && member.observed == member.after
            )
        );
        assert_eq!(done.jobs.len(), 2);
    }

    #[test]
    fn generated_controller_reopen_revokes_unused_approval_and_retains_forward_and_reverse_identity()
     {
        if crate::test_process::isolated(
            "executor::rename_fixture::controller::tests::generated_controller_reopen_revokes_unused_approval_and_retains_forward_and_reverse_identity",
            45,
        ) {
            return;
        }
        let controller = GeneratedRenameController::default();
        let before = controller
            .request(GeneratedRenameRequest::Create {})
            .unwrap();
        let old = approve(&controller);
        let reopened = controller
            .request(GeneratedRenameRequest::ReopenRetained {})
            .unwrap();
        assert_eq!(reopened.plan_id, before.plan_id);
        assert_eq!(reopened.digest, before.digest);
        assert_eq!(reopened.completed_steps, 0);
        assert!(reopened.ticket.is_none() && reopened.jobs.is_empty());
        assert!(start(&controller, old).is_err());
        start(&controller, approve(&controller)).unwrap();
        wait(&controller, |status| !status.busy);
        let forward = controller
            .request(GeneratedRenameRequest::ReopenRetained {})
            .unwrap();
        assert_eq!(forward.state, GeneratedRenameState::Completed);
        let reverse = controller
            .request(GeneratedRenameRequest::ReviewReverse {})
            .unwrap();
        let old = approve(&controller);
        let reopened = controller
            .request(GeneratedRenameRequest::ReopenRetained {})
            .unwrap();
        assert_eq!(reopened.plan_id, reverse.plan_id);
        assert!(reopened.reverse && reopened.ticket.is_none());
        assert!(start(&controller, old).is_err());
        start(&controller, approve(&controller)).unwrap();
        wait(&controller, |status| !status.busy);
        let restored = controller
            .request(GeneratedRenameRequest::ReopenRetained {})
            .unwrap();
        assert_eq!(restored.state, GeneratedRenameState::Completed);
        assert_eq!(restored.jobs.len(), 2);
        assert!(
            restored
                .members
                .iter()
                .all(|member| before.members.iter().any(|old| old.id == member.id
                    && old.catalog_asset_id == member.catalog_asset_id
                    && old.before == member.observed
                    && member.catalog_path == member.observed))
        );
    }
}
