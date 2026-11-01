//! Ephemeral backend session approvals. Persisted review/journal records never
//! reconstruct these grants; the renderer cannot choose their digest or action.
use crate::domain::{AppError, ErrorCode, Result};
use std::{
    collections::BTreeMap,
    sync::Mutex,
    time::{Duration, Instant},
};

const LIMIT: usize = 64;
const LIFETIME: Duration = Duration::from_secs(600);
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    #[cfg(any(test, feature = "fixture-probes"))]
    RenameFixture,
    PrepareNames,
    CopyFiles,
    #[cfg(any(test, feature = "fixture-probes", feature = "copy-workflow"))]
    ResumeFiles,
    #[cfg(any(test, feature = "copy-workflow"))]
    RecordImportSkip,
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Review {
    action: Action,
    id: String,
    digest: String,
    selection: String,
}
impl Review {
    pub(super) fn new(
        action: Action,
        id: String,
        digest: String,
        selection: String,
    ) -> Result<Self> {
        crate::operations::validate_id(&id)?;
        crate::operations::validate_id(&selection)?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(rejected("The review digest is invalid."));
        }
        Ok(Self {
            action,
            id,
            digest,
            selection,
        })
    }
}
struct Entry {
    review: Review,
    issued: Instant,
}
pub(super) struct Approvals {
    entries: Mutex<BTreeMap<String, Entry>>,
}
/// One-use proof deliberately has no Clone/Serialize/Deserialize implementation.
pub(super) struct Proof {
    review: Review,
    issued: Instant,
}
impl Proof {
    pub(super) fn authorize(self, expected: &Review) -> Result<()> {
        if self.review != *expected || !live(self.issued, Instant::now()) {
            return Err(rejected(
                "Approval does not authorize this exact reviewed action.",
            ));
        }
        Ok(())
    }
}
fn rejected(message: &str) -> AppError {
    AppError::new(ErrorCode::Conflict, message)
}
fn live(issued: Instant, now: Instant) -> bool {
    now.checked_duration_since(issued)
        .is_some_and(|age| age < LIFETIME)
}
impl Approvals {
    pub(super) fn new() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
        }
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn revoke_review(&self, id: &str) -> Result<()> {
        self.entries
            .lock()
            .map_err(|_| rejected("Session approvals are unavailable. Review again."))?
            .retain(|_, entry| entry.review.id != id);
        Ok(())
    }
    pub(super) fn issue(&self, review: Review) -> Result<String> {
        self.issue_at(review, Instant::now())
    }
    pub(super) fn consume(&self, ticket: &str, review: &Review) -> Result<Proof> {
        self.consume_at(ticket, review, Instant::now())
    }
    fn issue_at(&self, review: Review, now: Instant) -> Result<String> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| rejected("Session approvals are unavailable. Review again."))?;
        entries.retain(|_, entry| live(entry.issued, now));
        if entries.len() >= LIMIT {
            return Err(AppError::new(
                ErrorCode::Busy,
                "This session has 64 unused approvals. Wait for expiry before approving more work.",
            ));
        }
        let ticket = uuid::Uuid::new_v4().to_string();
        if entries.contains_key(&ticket) {
            return Err(rejected(
                "Could not allocate a unique session approval. Review again.",
            ));
        }
        entries.insert(
            ticket.clone(),
            Entry {
                review,
                issued: now,
            },
        );
        Ok(ticket)
    }
    fn consume_at(&self, ticket: &str, review: &Review, now: Instant) -> Result<Proof> {
        crate::operations::validate_id(ticket)?;
        let entry=self.entries.lock().map_err(|_|rejected("Session approvals are unavailable. Review again."))?.remove(ticket)
            .ok_or_else(||rejected("Approval is missing, already used or belongs to an earlier session. Review again."))?;
        // Every recognized ticket has a single attempt, including scope/expiry
        // rejection. A wrong selection/digest must require a fresh review.
        if !live(entry.issued, now) || entry.review != *review {
            return Err(rejected(
                "Approval expired or the reviewed action/destination selection changed. Review again.",
            ));
        }
        Ok(Proof {
            review: entry.review,
            issued: entry.issued,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn review() -> Review {
        Review::new(
            Action::PrepareNames,
            uuid::Uuid::new_v4().to_string(),
            "a".repeat(64),
            uuid::Uuid::new_v4().to_string(),
        )
        .unwrap()
    }
    #[test]
    fn approvals_bind_action_digest_selection_expiry_and_session_without_replay() {
        let approvals = Approvals::new();
        let expected = review();
        let start = Instant::now();
        for variant in 0..7 {
            let ticket = approvals.issue_at(expected.clone(), start).unwrap();
            let mut changed = expected.clone();
            let mut now = start;
            match variant {
                0 => changed.action = Action::CopyFiles,
                1 => changed.id = uuid::Uuid::new_v4().to_string(),
                2 => changed.digest = "b".repeat(64),
                3 => changed.selection = uuid::Uuid::new_v4().to_string(),
                4 => now = start + LIFETIME,
                5 => now = start + LIFETIME + Duration::from_secs(1),
                _ => now = start.checked_sub(Duration::from_secs(1)).unwrap(),
            }
            assert!(approvals.consume_at(&ticket, &changed, now).is_err());
            assert!(approvals.consume_at(&ticket, &expected, start).is_err());
        }
        let ticket = approvals.issue_at(expected.clone(), start).unwrap();
        let proof = approvals
            .consume_at(
                &ticket,
                &expected,
                start + LIFETIME - Duration::from_nanos(1),
            )
            .unwrap();
        proof.authorize(&expected).unwrap();
        assert!(approvals.consume_at(&ticket, &expected, start).is_err());
        let ticket = approvals.issue_at(expected.clone(), start).unwrap();
        drop(approvals);
        assert!(
            Approvals::new()
                .consume_at(&ticket, &expected, start)
                .is_err()
        );
    }
    #[test]
    fn approval_admission_is_bounded_and_consumption_is_atomic() {
        use std::sync::{Arc, Barrier};
        let approvals = Arc::new(Approvals::new());
        let expected = review();
        let start = Instant::now();
        for _ in 0..LIMIT {
            approvals.issue_at(expected.clone(), start).unwrap();
        }
        assert_eq!(
            approvals
                .issue_at(expected.clone(), start)
                .err()
                .unwrap()
                .code,
            ErrorCode::Busy
        );
        let ticket = approvals
            .issue_at(expected.clone(), start + LIFETIME)
            .unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let (approvals, expected, ticket, barrier) = (
                    approvals.clone(),
                    expected.clone(),
                    ticket.clone(),
                    barrier.clone(),
                );
                std::thread::spawn(move || {
                    barrier.wait();
                    approvals
                        .consume_at(&ticket, &expected, start + LIFETIME)
                        .is_ok()
                })
            })
            .collect();
        barrier.wait();
        assert_eq!(
            threads
                .into_iter()
                .map(|thread| usize::from(thread.join().unwrap()))
                .sum::<usize>(),
            1
        );
    }
}
