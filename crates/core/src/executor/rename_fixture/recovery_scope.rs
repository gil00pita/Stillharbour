//! Cold reconstruction of a constructor-owned generated scope. This module is
//! fixture-only. A retained record never restores an approval or starts I/O.
use super::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
    sync::Arc,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalScope {
    object: super::super::super::Object,
    version: super::super::super::Object,
    modified: SystemTime,
    bytes: u64,
    readonly: bool,
    created: Option<SystemTime>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    policy: String,
    forward: Plan,
    active: Plan,
    reverse: bool,
    source: super::super::super::Object,
    versions: super::super::super::Object,
    originals: BTreeMap<String, OriginalScope>,
    probes: Vec<super::super::super::Object>,
    catalog: super::super::catalog::ReopenScope,
}
macro_rules! require {
    ($condition:expr) => {
        if !$condition { return Err(failed("Retained generated scope differs from its constructor-owned evidence. Preserve all retained files.")); }
    };
}
const LIMIT: u64 = 128 * 1024;

fn bounded(parent: &File, name: &str, limit: u64) -> Result<Vec<u8>> {
    let file = native::read(parent, name)?;
    require!(filesystem::identity(&file)?.links == 1);
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    require!(bytes.len() as u64 <= limit);
    Ok(bytes)
}
fn retain(parent: &File, name: &str, bytes: &[u8]) -> Result<()> {
    if native::maybe_metadata(parent, name)?.is_some() {
        require!(bounded(parent, name, LIMIT)? == bytes);
        return Ok(());
    }
    let mut file = native::create_file(parent, name)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    require!(bounded(parent, name, LIMIT)? == bytes);
    Ok(())
}
pub(super) fn save(session: &GeneratedRenameSession, name: &str) -> Result<()> {
    let scope = Scope {
        policy: "generated-rename-cold-v1".into(),
        forward: session.fixture.plan.clone(),
        active: session.active.clone(),
        reverse: session.reverse,
        source: session.fixture.source_root.clone(),
        versions: session.fixture.version_root.clone(),
        originals: session
            .fixture
            .originals
            .iter()
            .map(|(id, original)| {
                (
                    id.clone(),
                    OriginalScope {
                        object: super::super::super::Object {
                            volume: original.identity.volume,
                            file: original.identity.file,
                        },
                        version: super::super::super::Object {
                            volume: original.version.volume,
                            file: original.version.file,
                        },
                        modified: original.modified,
                        bytes: original.bytes,
                        readonly: original.readonly,
                        created: original.created,
                    },
                )
            })
            .collect(),
        probes: session.fixture.probes.clone(),
        catalog: session.catalog.reopen_scope(&session.fixture)?,
    };
    let bytes = serde_json::to_vec(&scope).map_err(|error| {
        failed("Could not retain the generated recovery record.").caused_by(error)
    })?;
    require!(bytes.len() as u64 <= LIMIT);
    let container = root(&session.fixture.run.path().canonicalize()?)?;
    retain(
        container.leaf(),
        "catalog-child-owner",
        session.fixture.plan.id.as_bytes(),
    )?;
    retain(container.leaf(), name, &bytes)
}

// Private adapter input. Runtime callers supply only an owned TempDir
// capability; the existing child-exit harness supplies its parent-owned run.
pub(super) fn reopen(
    path: &Path,
    token: &str,
    name: &str,
    run: Run,
) -> Result<GeneratedRenameSession> {
    crate::operations::validate_id(token)?;
    let path = path.canonicalize()?;
    require!(path == run.path().canonicalize()?);
    let container = root(&path)?;
    require!(bounded(container.leaf(), "catalog-child-owner", 36)? == token.as_bytes());
    let scope: Scope =
        serde_json::from_slice(&bounded(container.leaf(), name, LIMIT)?).map_err(|error| {
            failed("The retained generated recovery record is invalid.").caused_by(error)
        })?;
    require!(scope.policy == "generated-rename-cold-v1" && scope.forward.id == token);
    require!(
        scope.forward.members.len() == MEMBERS.len()
            && scope.active.members.len() == MEMBERS.len()
            && scope.originals.len() == MEMBERS.len()
    );
    require!(scope.active.root_id == scope.forward.root_id && scope.probes.len() <= 64);
    for plan in [&scope.forward, &scope.active] {
        let canonical = graph::build(
            plan.id.clone(),
            plan.members.clone(),
            plan.name_facts.clone(),
        )?;
        require!(
            serde_json::to_vec(&canonical)
                .map_err(|error| failed("Invalid canonical recovery graph.").caused_by(error))?
                == serde_json::to_vec(plan)
                    .map_err(|error| failed("Invalid retained recovery graph.").caused_by(error))?
        );
        require!(plan.issues.is_empty());
    }
    let directory = root(&path.join("originals"))?;
    let versions = root(&path.join("retained"))?;
    require!(scope.source.matches(directory.leaf())? && scope.versions.matches(versions.leaf())?);
    let mut seen = std::collections::BTreeSet::new();
    let mut originals = BTreeMap::new();
    for member in &scope.forward.members {
        let index = MEMBERS
            .iter()
            .position(|(before, after)| member.before == *before && member.after == *after)
            .ok_or_else(|| failed("Foreign generated recovery member."))?;
        require!(seen.insert(index));
        let active = scope
            .active
            .members
            .iter()
            .find(|active| active.id == member.id)
            .ok_or_else(|| failed("A generated recovery member is missing."))?;
        require!(
            (&active.before, &active.after)
                == if scope.reverse {
                    (&member.after, &member.before)
                } else {
                    (&member.before, &member.after)
                }
        );
        let retained = scope
            .originals
            .get(&member.id)
            .ok_or_else(|| failed("A generated retained version is missing."))?;
        let expected = payload(index);
        require!(retained.bytes == expected.len() as u64);
        let version = native::read(versions.leaf(), &member.id)?;
        require!(retained.version.matches(&version)?);
        require!(bounded(versions.leaf(), &member.id, retained.bytes)? == expected.as_bytes());
        originals.insert(
            member.id.clone(),
            Original {
                identity: FileIdentity {
                    volume: retained.object.volume,
                    file: retained.object.file,
                    links: 1,
                },
                version: FileIdentity {
                    volume: retained.version.volume,
                    file: retained.version.file,
                    links: 1,
                },
                modified: retained.modified,
                bytes: retained.bytes,
                readonly: retained.readonly,
                created: retained.created,
            },
        );
    }
    for (index, object) in scope.probes.iter().enumerate() {
        let file = native::read(directory.leaf(), &format!(".stillharbour-probe-{index}"))?;
        require!(
            object.matches(&file)?
                && file.metadata()?.len() == 0
                && filesystem::identity(&file)?.links == 1
        );
    }
    let fixture = Fixture {
        run,
        root_id: scope.forward.root_id.clone(),
        source_root: scope.source,
        version_root: scope.versions,
        originals,
        probes: scope.probes,
        plan: scope.forward,
    };
    let journal = fixture.journal(&scope.active)?;
    let positions = fixture.locations(&scope.active, directory.leaf(), &journal)?;
    fixture.verify(directory.leaf(), versions.leaf(), &positions)?;
    drop(journal);
    let catalog = super::super::catalog::GeneratedCatalog::reopen(&fixture, scope.catalog)?;
    let session = GeneratedRenameSession {
        catalog,
        fixture,
        active: scope.active,
        reverse: scope.reverse,
        approvals: approval::Approvals::new(),
    };
    session.inspect()?;
    Ok(session)
}

pub(crate) struct Retained {
    owner: Arc<tempfile::TempDir>,
    container: super::super::super::Object,
    forward: String,
    active: String,
    record_digest: blake3::Hash,
}
impl Retained {
    pub(crate) fn capture(session: &mut GeneratedRenameSession) -> Result<Self> {
        session.approvals = approval::Approvals::new();
        let name = format!("catalog-recovery-{}.json", session.active.id);
        save(session, &name)?;
        let directory = root(&session.fixture.run.path().canonicalize()?)?;
        let container = super::super::super::Object::of(directory.leaf())?;
        let record_digest = blake3::hash(&bounded(directory.leaf(), &name, LIMIT)?);
        let forward = session.fixture.plan.id.clone();
        let active = session.active.id.clone();
        let owner = match &session.fixture.run {
            Run::Owned(owner) => owner.clone(),
            #[cfg(test)]
            Run::Child(_) => {
                return Err(failed("A child cannot acquire parent cleanup ownership."));
            }
        };
        Ok(Self {
            owner,
            container,
            forward,
            active,
            record_digest,
        })
    }
    pub(crate) fn open(&self) -> Result<GeneratedRenameSession> {
        let path = self.owner.path().canonicalize()?;
        let directory = root(&path)?;
        require!(self.container.matches(directory.leaf())?);
        require!(
            blake3::hash(&bounded(
                directory.leaf(),
                &format!("catalog-recovery-{}.json", self.active),
                LIMIT
            )?) == self.record_digest
        );
        let session = reopen(
            &path,
            &self.forward,
            &format!("catalog-recovery-{}.json", self.active),
            Run::Owned(self.owner.clone()),
        )?;
        require!(session.active.id == self.active);
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_reopen_retains_cleanup_ownership_and_rejects_record_changes_before_reconstruction() {
        let mut session = GeneratedRenameSession::create().unwrap();
        let path = session.fixture.run.path().to_owned();
        let owner = Retained::capture(&mut session).unwrap();
        let record = path.join(format!("catalog-recovery-{}.json", owner.active));
        let bytes = std::fs::read(&record).unwrap();
        drop(session);
        assert!(path.exists());
        let mut changed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        changed["active"]["id"] = serde_json::json!(uuid());
        std::fs::write(&record, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(owner.open().is_err());
        // Deliberate corruption of only the owned fixture's metadata record.
        // The runtime adapter never replaces a conflicting record.
        std::fs::write(&record, &bytes).unwrap();
        let restored = owner.open().unwrap();
        drop(owner);
        assert!(path.exists());
        let review = restored.inspect().unwrap();
        assert_eq!(review.completed_steps, 0);
        assert!(review.ticket.is_none());
        assert!(
            review
                .members
                .iter()
                .all(|member| member.observed == member.before)
        );
        drop(restored);
        assert!(
            !path.exists(),
            "Only the final owned capability may clean up this generated run"
        );
    }
    #[test]
    fn failed_scope_retention_keeps_the_live_session_and_revokes_unused_approval() {
        let mut session = GeneratedRenameSession::create().unwrap();
        let initial = session.inspect().unwrap();
        let grant = session
            .request(
                crate::operations::generated_rename::GeneratedRenameRequest::Approve {
                    plan_id: initial.plan_id.clone(),
                    acknowledged: true,
                },
            )
            .unwrap();
        let path = session.fixture.run.path().to_owned();
        let record = path.join(format!("catalog-recovery-{}.json", initial.plan_id));
        std::fs::write(&record, b"unexpected retained metadata").unwrap();
        assert!(Retained::capture(&mut session).is_err());
        assert!(path.exists());
        assert_eq!(
            std::fs::read(&record).unwrap(),
            b"unexpected retained metadata"
        );
        assert!(
            session
                .request(
                    crate::operations::generated_rename::GeneratedRenameRequest::Start {
                        plan_id: grant.plan_id,
                        ticket: grant.ticket.unwrap()
                    }
                )
                .is_err()
        );
        let current = session.inspect().unwrap();
        assert_eq!(current.completed_steps, 0);
        assert_eq!(
            serde_json::to_vec(&initial.members).unwrap(),
            serde_json::to_vec(&current.members).unwrap()
        );
        assert!(current.jobs.is_empty() && current.ticket.is_none());
    }
}
