//! Cold reconstruction only for an existing constructor-owned duplicate fixture.
//! The owner retains cleanup and the expected immutable record digest. No path,
//! scope record or approval is accepted from the renderer.
use super::*;
use std::{
    io::{Read, Write},
    sync::Arc,
};
const LIMIT: u64 = 128 * 1024;
const NAME: &str = "duplicate-owned-scope.json";
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    policy: String,
    container: Object,
    plan: Plan,
    source: Object,
    versions: Object,
    originals: BTreeMap<String, Original>,
    probes: Vec<Object>,
    quarantine: Object,
    volume: String,
    selected: Vec<String>,
    keeper: DuplicateKeeper,
}
pub(super) struct Retained {
    owner: Arc<tempfile::TempDir>,
    digest: blake3::Hash,
}
fn bytes(parent: &File) -> Result<Vec<u8>> {
    let file = native::read(parent, NAME)?;
    if filesystem::identity(&file)?.links != 1 || file.metadata()?.len() > LIMIT {
        return Err(failed(
            "Invalid retained duplicate scope size or link identity.",
        ));
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(failed("Duplicate scope exceeded its bound."));
    }
    Ok(bytes)
}
impl Retained {
    pub(super) fn capture(fixture: Quarantine) -> Result<Self> {
        let path = fixture.fixture.run.path().canonicalize()?;
        let directory = root(&path)?;
        let owner = match &fixture.fixture.run {
            Run::Owned(owner) => owner.clone(),
            #[cfg(test)]
            Run::Child(_) => {
                return Err(failed(
                    "A child cannot acquire duplicate cleanup ownership.",
                ));
            }
        };
        let scope = Scope {
            policy: "generated-duplicate-cold-v1".into(),
            container: Object::of(directory.leaf())?,
            plan: fixture.fixture.plan.clone(),
            source: fixture.fixture.source_root.clone(),
            versions: fixture.fixture.version_root.clone(),
            originals: fixture.fixture.originals.clone(),
            probes: fixture.fixture.probes.clone(),
            quarantine: fixture.root.clone(),
            volume: fixture.volume.clone(),
            selected: fixture.selected.clone(),
            keeper: fixture
                .duplicate_keeper
                .clone()
                .ok_or_else(|| failed("Only generated duplicate fixtures can be retained."))?,
        };
        let encoded = serde_json::to_vec(&scope).map_err(|error| {
            failed("Could not retain duplicate fixture scope.").caused_by(error)
        })?;
        if encoded.len() as u64 > LIMIT {
            return Err(failed("Duplicate scope exceeded its bound."));
        }
        if native::maybe_metadata(directory.leaf(), NAME)?.is_some() {
            if bytes(directory.leaf())? != encoded {
                return Err(failed("Retained duplicate scope cannot be replaced."));
            }
        } else {
            let mut file = native::create_file(directory.leaf(), NAME)?;
            file.write_all(&encoded)?;
            file.sync_all()?;
        }
        if bytes(directory.leaf())? != encoded {
            return Err(failed("Retained duplicate scope did not verify."));
        }
        Ok(Self {
            owner,
            digest: blake3::hash(&encoded),
        })
    }
    pub(super) fn open(&self) -> Result<Quarantine> {
        reopen(Run::Owned(self.owner.clone()), self.digest)
    }
}
fn reopen(run: Run, digest: blake3::Hash) -> Result<Quarantine> {
    let path = run.path().canonicalize()?;
    let directory = root(&path)?;
    let encoded = bytes(directory.leaf())?;
    if blake3::hash(&encoded) != digest {
        return Err(failed(
            "Retained duplicate record changed; no authority is reconstructed.",
        ));
    }
    let scope: Scope = serde_json::from_slice(&encoded)
        .map_err(|error| failed("Malformed retained duplicate scope.").caused_by(error))?;
    if scope.policy != "generated-duplicate-cold-v1"
        || !scope.container.matches(directory.leaf())?
        || scope.plan.members.len() != MEMBERS.len()
        || scope.originals.len() != MEMBERS.len()
        || scope.probes.len() > 64
        || scope.selected.len() != 2
        || scope.keeper.members.len() != 2
    {
        return Err(failed("Foreign or incomplete duplicate fixture scope."));
    }
    crate::operations::validate_id(&scope.plan.id)?;
    crate::operations::validate_id(&scope.plan.root_id)?;
    let canonical = graph::build(
        scope.plan.id.clone(),
        scope.plan.members.clone(),
        scope.plan.name_facts.clone(),
    )?;
    if serde_json::to_vec(&canonical)
        .map_err(|error| failed("Invalid canonical duplicate scope.").caused_by(error))?
        != serde_json::to_vec(&scope.plan)
            .map_err(|error| failed("Invalid retained duplicate plan.").caused_by(error))?
        || !canonical.issues.is_empty()
    {
        return Err(failed("Retained duplicate planning is not canonical."));
    }
    let versions = root(&path.join("retained"))?;
    let mut seen = std::collections::BTreeSet::new();
    for member in &scope.plan.members {
        let index = MEMBERS
            .iter()
            .position(|(before, after)| *before == member.before && *after == member.after)
            .ok_or_else(|| failed("Foreign duplicate member names."))?;
        let original = scope
            .originals
            .get(&member.id)
            .ok_or_else(|| failed("Missing duplicate original facts."))?;
        if !seen.insert(index)
            || original.identity.links != 1
            || original.version.links != 1
            || original.bytes != payload(index).len() as u64
        {
            return Err(failed("Foreign duplicate original evidence."));
        }
        let mut file = native::reopen_stage(versions.leaf(), &member.id, &original.version)?;
        let expected = payload(index);
        let equal = crate::inspection::hashing::equal(
            &mut file,
            &mut std::io::Cursor::new(expected.as_bytes()),
            original.bytes,
            &PacedIo::shared(),
            &AtomicBool::new(false),
            &mut crate::inspection::hashing::Stats::default(),
        )?;
        if !matches!(equal, IoStep::Completed(true)) {
            return Err(failed("Generated retained duplicate bytes changed."));
        }
    }
    let selected = scope
        .plan
        .members
        .iter()
        .filter(|member| matches!(member.before.as_str(), "Case.JPG" | "Case.xmp"))
        .map(|member| member.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if scope
        .selected
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        != selected
        || scope
            .keeper
            .members
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            != selected
    {
        return Err(failed("The duplicate bundle membership changed."));
    }
    let fixture = Quarantine {
        fixture: Fixture {
            template: None,
            run,
            root_id: scope.plan.root_id.clone(),
            source_root: scope.source,
            version_root: scope.versions,
            originals: scope.originals,
            probes: scope.probes,
            plan: scope.plan,
        },
        root: scope.quarantine,
        volume: scope.volume,
        selected: scope.selected,
        duplicate_keeper: Some(scope.keeper),
    };
    let _lease = fixture.lease(Access::Read)?;
    let held = fixture.open()?;
    fixture.inspect(&held)?;
    Ok(fixture)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_cold_exit_child() {
        let Some(path) = std::env::var_os("STILLHARBOUR_DUPLICATE_CHILD_ROOT") else {
            return;
        };
        let path = std::path::PathBuf::from(path).canonicalize().unwrap();
        assert!(path.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("stillharbour-rename-fixture-")
        );
        let digest = std::env::var("STILLHARBOUR_DUPLICATE_CHILD_DIGEST")
            .unwrap()
            .parse()
            .unwrap();
        let restore = std::env::var("STILLHARBOUR_DUPLICATE_CHILD_RESTORE").unwrap() == "true";
        let fixture = reopen(Run::Child(path), digest).unwrap();
        assert!(!fixture.execute(restore, Some(Fault::AfterMove(0))).unwrap());
        std::process::exit(85);
    }
    #[test]
    fn duplicate_quarantine_and_restore_survive_real_child_exit_with_cold_reconstruction() {
        let retained = Retained::capture(Quarantine::duplicate().unwrap()).unwrap();
        for restore in [false, true] {
            let mut child=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","executor::rename_fixture::quarantine::duplicate_recovery::tests::duplicate_cold_exit_child","--nocapture"]).env("STILLHARBOUR_DUPLICATE_CHILD_ROOT",retained.owner.path()).env("STILLHARBOUR_DUPLICATE_CHILD_DIGEST",retained.digest.to_hex().to_string()).env("STILLHARBOUR_DUPLICATE_CHILD_RESTORE",restore.to_string()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::inherit()).spawn().unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    assert_eq!(status.code(), Some(85));
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("Owned duplicate child exceeded 30 seconds");
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let fixture = retained.open().unwrap();
            assert!(fixture.resume(restore, None).unwrap());
        }
        let fixture = retained.open().unwrap();
        let held = fixture.open().unwrap();
        assert!(
            fixture
                .inspect(&held)
                .unwrap()
                .positions
                .values()
                .all(|position| !*position)
        );
        fixture.duplicate_guard(&held).unwrap();
    }
}
