//! Reviewed IPC lifecycle over the closed generated scope. No constructor can
//! adopt a caller path. Approvals are session-only and never journal contents.
use super::*;
use crate::operations::generated_rename::*;
#[cfg(test)]
#[path = "restart_tests.rs"]
mod restart_tests;

pub struct GeneratedRenameSession {
    // Close the catalog actor before its constructor-owned directory drops.
    catalog: super::catalog::GeneratedCatalog,
    fixture: Fixture,
    active: Plan,
    reverse: bool,
    approvals: approval::Approvals,
}
impl GeneratedRenameSession {
    pub fn create() -> Result<Self> {
        let fixture = Fixture::new()?;
        Ok(Self {
            catalog: super::catalog::GeneratedCatalog::create(&fixture)?,
            active: fixture.plan.clone(),
            fixture,
            reverse: false,
            approvals: approval::Approvals::new(),
        })
    }
    pub fn request(&mut self, request: GeneratedRenameRequest) -> Result<GeneratedRenameReview> {
        match request {
            GeneratedRenameRequest::Create {} => Err(failed(
                "A generated session already exists. Inspect its retained state.",
            )),
            GeneratedRenameRequest::Inspect {} => self.inspect(),
            GeneratedRenameRequest::ReviewReverse {} => {
                if self.reverse || self.inspect()?.state != GeneratedRenameState::Completed {
                    return Err(failed(
                        "Complete the forward operation before reviewing its undo.",
                    ));
                }
                let reverse = self.fixture.reverse_plan()?;
                if !reverse.issues.is_empty() {
                    return Err(failed(
                        "The reverse preview has native conflicts. Preserve every occupant.",
                    ));
                }
                self.fixture.journal(&reverse)?;
                self.catalog
                    .retain_plan(&reverse, Some(self.fixture.plan.id.clone()))?;
                self.active = reverse;
                self.reverse = true;
                self.inspect()
            }
            GeneratedRenameRequest::Approve {
                plan_id,
                acknowledged,
            } => {
                let mut view = self.inspect()?;
                if !acknowledged
                    || plan_id != view.plan_id
                    || view.state == GeneratedRenameState::Completed
                {
                    return Err(failed(
                        "Acknowledge this exact unfinished generated-file review.",
                    ));
                }
                view.ticket = Some(self.approvals.issue(self.binding(&view)?)?);
                Ok(view)
            }
            GeneratedRenameRequest::Start { plan_id, ticket } => {
                let (job, _) = self.prepare_start(plan_id, ticket)?;
                self.run_job(job.id, None)
            }
        }
    }
    pub(super) fn reader(&self) -> std::sync::Arc<crate::catalog::Catalog> {
        self.catalog.db.clone()
    }
    pub(super) fn prepare_start(
        &self,
        plan_id: String,
        ticket: String,
    ) -> Result<(
        crate::operations::original_jobs::OriginalOperationJob,
        GeneratedRenameReview,
    )> {
        let view = self.inspect()?;
        let binding = self.binding(&view)?;
        self.approvals
            .consume(&ticket, &binding)?
            .authorize(&binding)?;
        if plan_id != view.plan_id || view.state == GeneratedRenameState::Completed {
            return Err(failed("This generated-file plan is no longer executable."));
        }
        let job = crate::library::original_jobs::create(&self.catalog.db, self.active.id.clone())?;
        match self.inspect() {
            Ok(queued) => Ok((job, queued)),
            Err(error) => Err(self.record_stopped(&job.id, error)),
        }
    }
    pub(super) fn record_stopped(
        &self,
        job_id: &str,
        error: crate::domain::AppError,
    ) -> crate::domain::AppError {
        let recovery = crate::library::original_jobs::transition(&self.catalog.db, job_id.to_owned(), crate::operations::original_jobs::OriginalJobState::NeedsRecovery, "The worker stopped. Files and stages remain retained; inspect current facts before approving another attempt.".into());
        match recovery {
            Ok(_) => error,
            Err(recovery_error) => failed("The worker stopped and its durable outcome could not be recorded. Inspect retained files and history.").caused_by(format!("{error}; {recovery_error}")),
        }
    }
    pub(super) fn run_job(
        &self,
        job_id: String,
        control: Option<&RunControl>,
    ) -> Result<GeneratedRenameReview> {
        let outcome = (|| {
            crate::library::original_jobs::transition(
                &self.catalog.db,
                job_id.clone(),
                crate::operations::original_jobs::OriginalJobState::Running,
                "Checking and moving generated native objects.".into(),
            )?;
            if !self
                .fixture
                .execute_controlled(&self.active, None, control)?
                || control
                    .is_some_and(|control| control.stop.load(std::sync::atomic::Ordering::Acquire))
            {
                return Err(failed(
                    "The generated worker stopped at a journal boundary. Inspect retained files and approve a fresh continuation.",
                ));
            }
            self.commit_catalog(job_id.clone())
        })();
        if let Err(error) = outcome {
            return Err(self.record_stopped(&job_id, error));
        }
        self.inspect()
    }
    fn binding(&self, view: &GeneratedRenameReview) -> Result<approval::Review> {
        approval::Review::new(
            approval::Action::RenameFixture,
            view.plan_id.clone(),
            view.digest.clone(),
            self.fixture.root_id.clone(),
        )
    }
    fn commit_catalog(&self, job_id: String) -> Result<()> {
        let path = self.fixture.run.path().canonicalize()?;
        let directory = root(&path.join("originals"))?;
        let versions = root(&path.join("retained"))?;
        let _lease = Registry::shared().try_acquire(vec![
            Scope::new(root(&path.join("originals"))?, Access::Write),
            Scope::new(root(&path.join("retained"))?, Access::Read),
        ])?;
        let db = self.fixture.journal(&self.active)?;
        if Fixture::cursor(&db, &self.active)? != (self.active.steps.len(), false) {
            return Err(failed("Complete native movement before catalog commit."));
        }
        let locations = self
            .fixture
            .locations(&self.active, directory.leaf(), &db)?;
        let revisions = self
            .fixture
            .verify(directory.leaf(), versions.leaf(), &locations)?;
        self.catalog
            .commit(&self.fixture, &self.active, revisions, job_id)
    }
    fn inspect(&self) -> Result<GeneratedRenameReview> {
        let path = self.fixture.run.path().canonicalize()?;
        let directory = root(&path.join("originals"))?;
        let versions = root(&path.join("retained"))?;
        let _lease = Registry::shared().try_acquire(vec![
            Scope::new(root(&path.join("originals"))?, Access::Read),
            Scope::new(root(&path.join("retained"))?, Access::Read),
        ])?;
        let db = self.fixture.journal(&self.active)?;
        let (completed, pending) = Fixture::cursor(&db, &self.active)?;
        let locations = self
            .fixture
            .locations(&self.active, directory.leaf(), &db)?;
        let revisions = self
            .fixture
            .verify(directory.leaf(), versions.leaf(), &locations)?;
        let catalog = self
            .catalog
            .observe(&self.fixture, &self.active, &revisions)?;
        if catalog.committed && (completed != self.active.steps.len() || pending) {
            return Err(failed(
                "Catalog completion has no complete native movement proof.",
            ));
        }
        let jobs = crate::library::original_jobs::list(&self.catalog.db)?;
        let digest = blake3::hash(
            &serde_json::to_vec(&(
                "generated-rename-review-v1",
                &self.active.digest,
                self.reverse,
                completed,
                pending,
                &locations,
                revisions,
                &catalog.rows,
                catalog.committed,
                &jobs,
                source_native::revision(directory.leaf())?,
            ))
            .map_err(|error| failed("Cannot fingerprint generated review.").caused_by(error))?,
        )
        .to_hex()
        .to_string();
        Ok(GeneratedRenameReview {
            plan_id: self.active.id.clone(),
            digest,
            reverse: self.reverse,
            state: if completed == self.active.steps.len() {
                if catalog.committed {
                    GeneratedRenameState::Completed
                } else {
                    GeneratedRenameState::NeedsCatalogCommit
                }
            } else if completed > 0 || pending {
                GeneratedRenameState::NeedsContinuation
            } else {
                GeneratedRenameState::Review
            },
            completed_steps: completed as u32,
            total_steps: self.active.steps.len() as u32,
            pending_intent: pending,
            catalog_committed: catalog.committed,
            jobs,
            members: self
                .active
                .members
                .iter()
                .map(|member| GeneratedRenameMember {
                    id: member.id.clone(),
                    before: member.before.clone(),
                    after: member.after.clone(),
                    observed: locations[&member.id].clone(),
                    catalog_asset_id: catalog
                        .rows
                        .iter()
                        .find(|row| row.member == member.id)
                        .expect("validated exact generated membership")
                        .asset
                        .clone(),
                    catalog_path: catalog
                        .rows
                        .iter()
                        .find(|row| row.member == member.id)
                        .expect("validated exact generated membership")
                        .path
                        .clone(),
                })
                .collect(),
            ticket: None,
            production_authorization: false,
            independent_recovery_proven: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grant(session: &mut GeneratedRenameSession) -> GeneratedRenameReview {
        let view = session.request(GeneratedRenameRequest::Inspect {}).unwrap();
        session
            .request(GeneratedRenameRequest::Approve {
                plan_id: view.plan_id,
                acknowledged: true,
            })
            .unwrap()
    }
    fn start(
        session: &mut GeneratedRenameSession,
        view: GeneratedRenameReview,
    ) -> Result<GeneratedRenameReview> {
        session.request(GeneratedRenameRequest::Start {
            plan_id: view.plan_id,
            ticket: view.ticket.unwrap(),
        })
    }
    #[test]
    fn generated_review_requires_exact_grant_and_separate_reverse_approval() {
        let mut session = GeneratedRenameSession::create().unwrap();
        let view = session.inspect().unwrap();
        assert_eq!(view.completed_steps, 0);
        assert!(
            session
                .request(GeneratedRenameRequest::ReviewReverse {})
                .is_err()
        );
        assert!(
            session
                .request(GeneratedRenameRequest::Approve {
                    plan_id: view.plan_id.clone(),
                    acknowledged: false
                })
                .is_err()
        );
        assert!(
            session
                .request(GeneratedRenameRequest::Start {
                    plan_id: view.plan_id.clone(),
                    ticket: uuid()
                })
                .is_err()
        );
        let rejected_attempt = grant(&mut session);
        assert!(
            session
                .request(GeneratedRenameRequest::Start {
                    plan_id: uuid(),
                    ticket: rejected_attempt.ticket.clone().unwrap(),
                })
                .is_err()
        );
        assert!(start(&mut session, rejected_attempt).is_err());
        let approved = grant(&mut session);
        let completed = start(&mut session, approved.clone()).unwrap();
        assert!(completed.catalog_committed);
        assert!(
            completed
                .members
                .iter()
                .all(|member| member.catalog_path == member.after)
        );
        assert_eq!(completed.state, GeneratedRenameState::Completed);
        assert!(completed.members.iter().all(|m| m.after == m.observed));
        assert!(start(&mut session, approved).is_err());
        let reverse = session
            .request(GeneratedRenameRequest::ReviewReverse {})
            .unwrap();
        assert!(reverse.reverse);
        assert!(reverse.ticket.is_none());
        assert_eq!(reverse.completed_steps, 0);
        let approved = grant(&mut session);
        let completed = start(&mut session, approved).unwrap();
        assert!(completed.catalog_committed);
        assert!(
            completed
                .members
                .iter()
                .all(|member| member.catalog_path == member.observed
                    && view.members.iter().any(|initial| initial.id == member.id
                        && initial.catalog_asset_id == member.catalog_asset_id))
        );
        assert!(completed.members.iter().all(|m| m.after == m.observed));
        assert!(completed.members.iter().all(|m| {
            session
                .fixture
                .plan
                .members
                .iter()
                .any(|original| original.id == m.id && original.before == m.observed)
        }));
        assert!(!completed.production_authorization && !completed.independent_recovery_proven);
    }
    #[test]
    fn generated_review_continuation_does_not_inherit_the_old_grant() {
        let mut session = GeneratedRenameSession::create().unwrap();
        let old = grant(&mut session);
        assert!(
            !session
                .fixture
                .execute(&session.active, Some(Fault::AfterMove(0)))
                .unwrap()
        );
        let inspection = session.inspect().unwrap();
        assert_eq!(inspection.state, GeneratedRenameState::NeedsContinuation);
        assert!(inspection.pending_intent);
        assert!(start(&mut session, old).is_err());
        let approved = grant(&mut session);
        assert_eq!(
            start(&mut session, approved).unwrap().state,
            GeneratedRenameState::Completed
        );
    }
    fn catalog_annotations(session: &GeneratedRenameSession) -> Vec<(String, bool, u32, String)> {
        session
            .catalog
            .db
            .call(|db| {
                Ok(db
                    .prepare("SELECT id,favorite,rating,tags FROM assets ORDER BY id")?
                    .query_map([], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?)
            })
            .unwrap()
    }
    #[test]
    fn generated_review_jobs_keep_immutable_plans_and_cannot_invent_verified_completion() {
        use crate::{library::original_jobs as jobs, operations::original_jobs::OriginalJobState};
        let session = GeneratedRenameSession::create().unwrap();
        let job = jobs::create(&session.catalog.db, session.active.id.clone()).unwrap();
        assert!(
            jobs::transition(
                &session.catalog.db,
                job.id.clone(),
                OriginalJobState::CompletedVerified,
                "invalid shortcut".into()
            )
            .is_err()
        );
        let id = job.id.clone();
        session
            .catalog
            .db
            .call(move |db| {
                assert!(
                    db.execute("UPDATE original_operation_plans SET payload='{}'", [])
                        .is_err()
                );
                assert!(
                    db.execute("DELETE FROM original_operation_plans", [])
                        .is_err()
                );
                assert!(
                    db.execute(
                        "UPDATE original_operation_jobs SET state='completed_verified' WHERE id=?",
                        [&id]
                    )
                    .is_err()
                );
                assert!(
                    db.execute("UPDATE original_operation_events SET payload='{}'", [])
                        .is_err()
                );
                assert!(
                    db.execute("DELETE FROM original_operation_events", [])
                        .is_err()
                );
                Ok(())
            })
            .unwrap();
        jobs::transition(
            &session.catalog.db,
            job.id.clone(),
            OriginalJobState::Running,
            "worker admission".into(),
        )
        .unwrap();
        let wrong = uuid();
        let id = job.id.clone();
        assert!(
            session
                .catalog
                .db
                .call(move |db| {
                    let tx = db.transaction()?;
                    jobs::finish_verified_db(&tx, &id, &wrong, 10)?;
                    tx.commit()?;
                    Ok(())
                })
                .is_err()
        );
        jobs::recover(&session.catalog.db).unwrap();
        let view = session.inspect().unwrap();
        assert_eq!(view.completed_steps, 0);
        assert!(
            view.members
                .iter()
                .all(|member| member.observed == member.before)
        );
        assert_eq!(view.jobs.len(), 1);
        assert_eq!(view.jobs[0].state, OriginalJobState::NeedsRecovery);
        assert_eq!(view.jobs[0].verified_moves, 0);
        assert!(view.ticket.is_none());
    }
    #[test]
    fn generated_review_catalog_root_path_rebinding_cannot_adopt_another_scope() {
        let mut session = GeneratedRenameSession::create().unwrap();
        let approved = grant(&mut session);
        session
            .catalog
            .db
            .call(|db| {
                db.execute("UPDATE roots SET native_path=X'00'", [])?;
                Ok(())
            })
            .unwrap();
        assert!(start(&mut session, approved).is_err());
        let journal = session.fixture.journal(&session.active).unwrap();
        assert_eq!(
            Fixture::cursor(&journal, &session.active).unwrap(),
            (0, false)
        );
        let path = session.fixture.run.path().canonicalize().unwrap();
        let directory = root(&path.join("originals")).unwrap();
        let versions = root(&path.join("retained")).unwrap();
        let locations = session
            .fixture
            .locations(&session.active, directory.leaf(), &journal)
            .unwrap();
        session
            .fixture
            .verify(directory.leaf(), versions.leaf(), &locations)
            .unwrap();
    }
    #[test]
    fn generated_review_catalog_failures_roll_back_cycles_and_require_fresh_commit_approval() {
        for boundary in [
            "BEFORE UPDATE OF relative_path ON assets",
            "BEFORE INSERT ON job_events WHEN NEW.job_id LIKE 'generated-rename-catalog:%'",
            "BEFORE INSERT ON original_operation_events WHEN NEW.state='completed_verified'",
        ] {
            let mut session = GeneratedRenameSession::create().unwrap();
            let initial = session.inspect().unwrap();
            let annotations = catalog_annotations(&session);
            let trigger = format!(
                "CREATE TEMP TRIGGER generated_commit_denial {boundary} BEGIN SELECT RAISE(ABORT,'injected catalog commit denial'); END;"
            );
            session
                .catalog
                .db
                .call(move |db| {
                    db.execute_batch(&trigger)?;
                    Ok(())
                })
                .unwrap();
            let approved = grant(&mut session);
            assert!(start(&mut session, approved.clone()).is_err());
            let stopped = session.inspect().unwrap();
            assert_eq!(stopped.state, GeneratedRenameState::NeedsCatalogCommit);
            assert!(!stopped.catalog_committed);
            assert_eq!(stopped.completed_steps, stopped.total_steps);
            assert!(
                stopped
                    .members
                    .iter()
                    .all(|member| member.observed == member.after
                        && member.catalog_path == member.before)
            );
            assert!(
                session
                    .request(GeneratedRenameRequest::ReviewReverse {})
                    .is_err()
            );
            assert!(start(&mut session, approved).is_err());
            assert_eq!(catalog_annotations(&session), annotations);
            let journal = session.fixture.journal(&session.active).unwrap();
            let facts: i64 = journal
                .query_row("SELECT count(*) FROM events", [], |row| row.get(0))
                .unwrap();
            session
                .catalog
                .db
                .call(|db| {
                    db.execute_batch("DROP TRIGGER generated_commit_denial")?;
                    Ok(())
                })
                .unwrap();
            let approved = grant(&mut session);
            let done = start(&mut session, approved).unwrap();
            assert_eq!(done.state, GeneratedRenameState::Completed);
            assert!(done.catalog_committed);
            assert_eq!(
                journal
                    .query_row("SELECT count(*) FROM events", [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                facts
            );
            assert_eq!(catalog_annotations(&session), annotations);
            assert!(done.members.iter().all(|member| {
                member.catalog_path == member.after
                    && initial.members.iter().any(|old| {
                        old.id == member.id && old.catalog_asset_id == member.catalog_asset_id
                    })
            }));
            session.catalog.db.call(|db| {
                assert_eq!(db.query_row("SELECT count(*) FROM job_events WHERE job_id LIKE 'generated-rename-catalog:%'", [], |row| row.get::<_,i64>(0))?, 1);
                assert!(db.execute("UPDATE job_events SET state='{}' WHERE job_id LIKE 'generated-rename-catalog:%'", []).is_err());
                assert!(db.execute("DELETE FROM job_events WHERE job_id LIKE 'generated-rename-catalog:%'", []).is_err());
                Ok(())
            }).unwrap();
        }
    }
}
