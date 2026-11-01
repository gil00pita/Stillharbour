//! Reviewed IPC lifecycle over the closed generated scope. No constructor can
//! adopt a caller path. Approvals are session-only and never journal contents.
use super::*;
use crate::operations::generated_rename::*;

pub struct GeneratedRenameSession {
    fixture: Fixture,
    active: Plan,
    reverse: bool,
    approvals: approval::Approvals,
}
impl GeneratedRenameSession {
    pub fn create() -> Result<Self> {
        let fixture = Fixture::new()?;
        Ok(Self {
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
                let view = self.inspect()?;
                let binding = self.binding(&view)?;
                self.approvals
                    .consume(&ticket, &binding)?
                    .authorize(&binding)?;
                if plan_id != view.plan_id || view.state == GeneratedRenameState::Completed {
                    return Err(failed("This generated-file plan is no longer executable."));
                }
                // The closed engine performs its own fresh native validation
                // and narrow worker authorization after this reviewed grant.
                if !self.fixture.execute(&self.active, None)? {
                    return Err(failed(
                        "The generated operation stopped. Inspect before continuing.",
                    ));
                }
                self.inspect()
            }
        }
    }
    fn binding(&self, view: &GeneratedRenameReview) -> Result<approval::Review> {
        approval::Review::new(
            approval::Action::RenameFixture,
            view.plan_id.clone(),
            view.digest.clone(),
            self.fixture.root_id.clone(),
        )
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
        let digest = blake3::hash(
            &serde_json::to_vec(&(
                "generated-rename-review-v1",
                &self.active.digest,
                self.reverse,
                completed,
                pending,
                &locations,
                revisions,
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
                GeneratedRenameState::Completed
            } else if completed > 0 || pending {
                GeneratedRenameState::NeedsContinuation
            } else {
                GeneratedRenameState::Review
            },
            completed_steps: completed as u32,
            total_steps: self.active.steps.len() as u32,
            pending_intent: pending,
            members: self
                .active
                .members
                .iter()
                .map(|member| GeneratedRenameMember {
                    id: member.id.clone(),
                    before: member.before.clone(),
                    after: member.after.clone(),
                    observed: locations[&member.id].clone(),
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
}
