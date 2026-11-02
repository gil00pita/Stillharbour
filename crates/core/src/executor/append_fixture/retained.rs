//! Generated retained-version replacement, built from the reviewed difference.
//! Every move is central native no-replace; no real-library adoption API exists.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    AfterPrepared,
    AfterOldIntent,
    AfterOldMove,
    AfterOldRecorded,
    BeforeNewPublish,
    AfterNewPublish,
    AfterComplete,
}
#[cfg(test)]
thread_local! {static COLD_EXIT: std::cell::Cell<bool> = const {std::cell::Cell::new(false)};}
fn interrupted(fault: Option<Fault>, point: Fault) -> bool {
    if fault != Some(point) {
        return false;
    }
    #[cfg(test)]
    if COLD_EXIT.with(|flag| flag.get()) {
        std::process::exit(79);
    }
    true
}
#[cfg(test)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LabControl {
    fixture: FixtureControl,
    seed: CopyPlan,
    old: Observed,
    incoming: Observed,
    history: String,
    stage: String,
    old_leaf: String,
    baseline_leaf: String,
    digest: String,
    prior: Vec<(Vec<u8>, Vec<u8>)>,
    modified: std::time::SystemTime,
    created: Option<std::time::SystemTime>,
    old_content_digest: String,
}
const PHASES: [&str; 10] = [
    "replacement_scope",
    "stage_intent",
    "stage_ready",
    "incoming_ready",
    "baseline_ready",
    "old_move_intent",
    "old_moved",
    "new_publish_intent",
    "new_published",
    "replacement_completed",
];
struct Lab {
    fixture: Fixture,
    seed: CopyPlan,
    old: Observed,
    incoming: Observed,
    history: String,
    stage: String,
    old_leaf: String,
    baseline_leaf: String,
    digest: String,
    prior: BTreeMap<PathBuf, Vec<u8>>,
    modified: std::time::SystemTime,
    created: Option<std::time::SystemTime>,
    old_content_digest: String,
}
impl Lab {
    #[cfg(test)]
    fn control(&self) -> LabControl {
        LabControl {
            fixture: self.fixture.control(),
            seed: self.seed.clone(),
            old: self.old.clone(),
            incoming: self.incoming.clone(),
            history: self.history.clone(),
            stage: self.stage.clone(),
            old_leaf: self.old_leaf.clone(),
            baseline_leaf: self.baseline_leaf.clone(),
            digest: self.digest.clone(),
            prior: self
                .prior
                .iter()
                .map(|(p, b)| (source::encode_path(p), b.clone()))
                .collect(),
            modified: self.modified,
            created: self.created,
            old_content_digest: self.old_content_digest.clone(),
        }
    }
    #[cfg(test)]
    fn from_control(root: PathBuf, c: LabControl, token: &str) -> Result<Self> {
        let fixture = Fixture::from_control(root, c.fixture, token)?;
        let prior = c
            .prior
            .into_iter()
            .map(|(p, b)| Ok((source::decode_path(&p)?, b)))
            .collect::<Result<_>>()?;
        let lab = Self {
            fixture,
            seed: c.seed,
            old: c.old,
            incoming: c.incoming,
            history: c.history,
            stage: c.stage,
            old_leaf: c.old_leaf,
            baseline_leaf: c.baseline_leaf,
            digest: c.digest,
            prior,
            modified: c.modified,
            created: c.created,
            old_content_digest: c.old_content_digest,
        };
        for name in [&lab.history, &lab.stage, &lab.old_leaf, &lab.baseline_leaf] {
            if !crate::organization::portable_filename(name) {
                return Err(failed("Invalid generated replacement child name."));
            }
        }
        lab.check()?;
        lab.facts()?;
        Ok(lab)
    }
    fn new() -> Result<Self> {
        let fixture = Fixture::new()?;
        let id = uuid::Uuid::new_v4().to_string();
        let path = source::encode_path(Path::new("edited.jpg"));
        let old = fixture
            .projection
            .destinations
            .iter()
            .find(|f| f.path == path)
            .cloned()
            .ok_or_else(|| failed("Missing reviewed destination difference."))?;
        let incoming = fixture
            .projection
            .sources
            .iter()
            .find(|f| f.path == path)
            .cloned()
            .ok_or_else(|| failed("Missing reviewed source difference."))?;
        if old.identity.links != 1 {
            return Err(failed(
                "Replacement requires an ordinary single-link destination.",
            ));
        }
        let (src, dst) = fixture.paths()?;
        let a = source::open_root(&src, None)?;
        let b = output_chain(&dst)?;
        let history = format!(".stillharbour-history-{}", fixture.projection.catalog_id);
        // Constructor baseline: two prior versions already belong to this lab.
        let history_path = dst.join(&history);
        fixture.boundary(&history_path)?;
        std::fs::create_dir(&history_path)?;
        let prior = BTreeMap::from([
            (
                history_path.join(format!("prior-{}.bin", uuid::Uuid::new_v4())),
                b"older destination version one".to_vec(),
            ),
            (
                history_path.join(format!("prior-{}.bin", uuid::Uuid::new_v4())),
                b"older destination version two".to_vec(),
            ),
        ]);
        for (p, bytes) in &prior {
            std::fs::write(p, bytes)?;
        }
        let metadata = std::fs::metadata(
            dst.join(source::decode_path(&fixture.projection.namespace)?)
                .join("edited.jpg"),
        )?;
        let modified = metadata.modified()?;
        let created = metadata.created().ok();
        let mut seed = CopyPlan {
            id: id.clone(),
            digest: String::new(),
            source: Object::of(&a)?,
            destination: Object::of(b.leaf())?,
            source_volume: fixture.projection.source_volume.clone(),
            destination_volume: fixture.projection.destination_volume.clone(),
            files: vec![],
        };
        seed.digest = seed.digest()?;
        let stage = format!(".stillharbour-replacement-{id}");
        let old_leaf = format!("retained-{id}.bin");
        let baseline_leaf = format!("verified-baseline-{id}.bin");
        let old_content_digest = blake3::hash(
            fixture
                .neighbors
                .get(
                    &dst.join(source::decode_path(&fixture.projection.namespace)?)
                        .join("edited.jpg"),
                )
                .ok_or_else(|| failed("Missing independently generated old-version manifest."))?,
        )
        .to_hex()
        .to_string();
        let encoded = serde_json::to_vec(&(
            &fixture.projection.digest,
            &old,
            &incoming,
            &history,
            &stage,
            &old_leaf,
            &baseline_leaf,
            &old_content_digest,
        ))
        .map_err(|e| failed("Could not freeze replacement review.").caused_by(e))?;
        let digest = blake3::hash(&encoded).to_hex().to_string();
        let lab = Self {
            fixture,
            seed,
            old,
            incoming,
            history,
            stage,
            old_leaf,
            baseline_leaf,
            digest,
            prior,
            modified,
            created,
            old_content_digest,
        };
        let _journal = Journal::create(&lab.journal_path()?, &lab.seed)?;
        lab.record(PHASES[0],serde_json::json!({"reviewDigest":lab.digest,"old":lab.old,"incoming":lab.incoming,"history":lab.history,"stage":lab.stage,"oldLeaf":lab.old_leaf,"baselineLeaf":lab.baseline_leaf}))?;
        Ok(lab)
    }
    fn journal_path(&self) -> Result<PathBuf> {
        let p = self.fixture.owned.path().join("data/replacement.sqlite");
        self.fixture.boundary(&p)?;
        Ok(p)
    }
    fn journal(&self) -> Result<Journal> {
        Journal::open(&self.journal_path()?, &self.seed)
    }
    fn facts(&self) -> Result<Vec<(String, serde_json::Value)>> {
        let journal = self.journal()?;
        let mut query=journal.0.prepare("SELECT phase,CASE WHEN length(CAST(evidence AS BLOB))<=8192 THEN evidence END FROM events ORDER BY sequence LIMIT 11")?;
        let mut rows = query.query([])?;
        let mut facts = Vec::new();
        while let Some(row) = rows.next()? {
            let phase: String = row.get(0)?;
            let text: Option<String> = row.get(1)?;
            let value: serde_json::Value = serde_json::from_str(
                &text.ok_or_else(|| failed("Oversized retained replacement fact."))?,
            )
            .map_err(|e| failed("Invalid retained replacement fact.").caused_by(e))?;
            if PHASES.get(facts.len()).copied() != Some(phase.as_str()) {
                return Err(failed(
                    "Unknown, repeated or out-of-order replacement fact.",
                ));
            }
            facts.push((phase, value));
        }
        if facts.first().is_none_or(|(_, v)| {
            v.get("reviewDigest").and_then(|v| v.as_str()) != Some(&self.digest)
        }) {
            return Err(failed("Replacement journal belongs to another review."));
        }
        Ok(facts)
    }
    fn record(&self, phase: &str, value: serde_json::Value) -> Result<()> {
        if phase == PHASES[0] {
            return self.journal()?.append(None, phase, value);
        }
        let facts = self.facts()?;
        let index = PHASES
            .iter()
            .position(|p| *p == phase)
            .ok_or_else(|| failed("Invalid replacement phase."))?;
        if let Some((_, old)) = facts.get(index) {
            if *old != value {
                return Err(failed("Replacement evidence changed after recording."));
            }
            return Ok(());
        }
        if facts.len() != index {
            return Err(failed("Replacement cannot skip a required phase."));
        }
        self.journal()?.append(None, phase, value)
    }
    fn approval(&self, recovery: bool) -> Result<approval::Review> {
        let digest = if recovery {
            let facts = self.facts()?;
            let positions = self.positions()?;
            blake3::hash(
                &serde_json::to_vec(&(&self.digest, facts, positions))
                    .map_err(|e| failed("Could not freeze replacement recovery.").caused_by(e))?,
            )
            .to_hex()
            .to_string()
        } else {
            self.digest.clone()
        };
        approval::Review::new(
            approval::Action::ReplaceRetaining,
            self.seed.id.clone(),
            digest,
            self.fixture.selection.clone(),
        )
    }
    fn positions(&self) -> Result<Vec<Option<(Object, String, u64)>>> {
        let (_, dst) = self.fixture.paths()?;
        let b = output_chain(&dst)?;
        let dataset = native::read_directory(b.leaf(), ".stillharbour-copy")?;
        let cat = native::read_directory(&dataset, &self.fixture.projection.catalog_id)?;
        let target = native::read_directory(&cat, &self.fixture.projection.root_id)?;
        let history = native::read_directory(b.leaf(), &self.history)?;
        let stage = if native::lookup(b.leaf(), &self.stage)?.is_some() {
            Some(native::read_directory(b.leaf(), &self.stage)?)
        } else {
            None
        };
        let files = [
            native::maybe_read(&target, "edited.jpg")?,
            native::maybe_read(&history, &self.old_leaf)?,
            stage
                .as_ref()
                .map(|s| native::maybe_read(s, "incoming.bin"))
                .transpose()?
                .flatten(),
            native::maybe_read(&history, &self.baseline_leaf)?,
        ];
        files
            .into_iter()
            .map(|f| {
                f.map(|f| Ok((Object::of(&f)?, source::revision(&f)?, f.metadata()?.len())))
                    .transpose()
            })
            .collect()
    }
    fn check(&self) -> Result<()> {
        self.fixture.validate_excluding(Some(&self.old.path))?;
        for (p, bytes) in &self.prior {
            self.fixture.boundary(p)?;
            if std::fs::read(p)? != *bytes {
                return Err(failed("A previous retained version changed."));
            }
        }
        Ok(())
    }
    fn comparison_file(&self, file: &File) -> Result<CopyFile> {
        Ok(CopyFile {
            source: "reviewed destination version".into(),
            target: self.old_leaf.clone(),
            stage: "unused.partial".into(),
            object: Object::of(file)?,
            revision: source::revision(file)?,
            bytes: self.old.bytes,
            source_location: None,
            destination_location: None,
            stage_location: None,
            prepared_stage: None,
        })
    }
    fn prepared_copy(
        &self,
        source_root: &File,
        destination: &File,
        relative_source: &Path,
        parent: &str,
        target: &str,
        journal_name: &str,
    ) -> Result<CopyPlan> {
        let location = locations::Source::capture(source_root, relative_source)?;
        let file = location.open(source_root)?;
        let mut plan = CopyPlan {
            id: uuid::Uuid::new_v4().to_string(),
            digest: String::new(),
            source: Object::of(source_root)?,
            destination: Object::of(destination)?,
            source_volume: source::volume_token(source_root)?,
            destination_volume: source::volume_token(destination)?,
            files: vec![CopyFile {
                source: relative_source.to_string_lossy().into_owned(),
                target: target.into(),
                stage: format!("{}.partial", uuid::Uuid::new_v4()),
                object: Object::of(&file)?,
                revision: source::revision(&file)?,
                bytes: file.metadata()?.len(),
                source_location: Some(location),
                destination_location: Some(locations::Destination::capture(destination, parent)?),
                stage_location: Some(locations::Destination::capture(destination, parent)?),
                prepared_stage: None,
            }],
        };
        plan.digest = plan.digest()?;
        let path = self.fixture.owned.path().join("data").join(journal_name);
        self.fixture.boundary(&path)?;
        let journal = Journal::create(&path, &plan)?;
        let result = CopyContext {
            plan: &plan,
            journal: &journal,
            source_root,
            destination,
            io: &PacedIo::shared(),
            cancel: &AtomicBool::new(false),
            validate_scope: &|| self.check(),
        }
        .execute(None)?;
        if result != CopyOutcome::CompletedVerified {
            return Err(failed("Replacement preparation did not verify."));
        }
        Ok(plan)
    }
    fn execute(
        &self,
        recovery: bool,
        expected: &approval::Review,
        proof: approval::Proof,
        fault: Option<Fault>,
    ) -> Result<CopyOutcome> {
        proof.authorize(expected)?;
        let _lease = self.fixture.lease()?;
        self.check()?;
        if self.approval(recovery)? != *expected {
            return Err(failed(
                "Replacement recovery observations changed after review.",
            ));
        }
        let mut facts = self.facts()?;
        if !recovery && facts.len() != 1 {
            return Err(failed("Inspect replacement recovery before any retry."));
        }
        let (src, dst) = self.fixture.paths()?;
        let a = source::open_root(&src, Some(&self.fixture.projection.source_identity))?;
        let b = output_chain(&dst)?;
        let namespace = relative(&source::decode_path(&self.fixture.projection.namespace)?)?;
        let parent = locations::Destination::capture(b.leaf(), &namespace)?.open(b.leaf())?;
        let history = native::read_directory(b.leaf(), &self.history)?;
        if facts.len() < 5 {
            if recovery {
                return Err(failed(
                    "Replacement preparation needs separate native recovery review; retain all objects.",
                ));
            }
            check_file(&parent, &self.old)?;
            self.record(PHASES[1], serde_json::json!({"stage":self.stage}))?;
            let stage = native::create_directory(b.leaf(), &self.stage)?;
            self.record(PHASES[2], serde_json::json!({"object":Object::of(&stage)?}))?;
            let incoming = self.prepared_copy(
                &a,
                b.leaf(),
                &source::decode_path(&self.incoming.path)?,
                &self.stage,
                "incoming.bin",
                "replacement-incoming.sqlite",
            )?;
            let incoming_file = native::read(&stage, "incoming.bin")?;
            self.record(PHASES[3],serde_json::json!({"planDigest":incoming.digest,"object":Object::of(&incoming_file)?}))?;
            let baseline = self.prepared_copy(
                b.leaf(),
                b.leaf(),
                &source::decode_path(&self.fixture.projection.namespace)?.join("edited.jpg"),
                &self.history,
                &self.baseline_leaf,
                "replacement-baseline.sqlite",
            )?;
            let baseline_file = native::read(&history, &self.baseline_leaf)?;
            self.record(PHASES[4],serde_json::json!({"planDigest":baseline.digest,"object":Object::of(&baseline_file)?}))?;
            facts = self.facts()?;
        }
        if interrupted(fault, Fault::AfterPrepared) {
            return Ok(CopyOutcome::Interrupted);
        }
        let stage = native::read_directory(b.leaf(), &self.stage)?;
        let expected_stage: Object = serde_json::from_value(facts[2].1["object"].clone())
            .map_err(|e| failed("Invalid retained replacement staging identity.").caused_by(e))?;
        if !expected_stage.matches(&stage)? {
            return Err(failed("Replacement staging directory identity changed."));
        }
        let expected_incoming: Object = serde_json::from_value(facts[3].1["object"].clone())
            .map_err(|e| failed("Invalid retained incoming identity.").caused_by(e))?;
        let expected_baseline: Object = serde_json::from_value(facts[4].1["object"].clone())
            .map_err(|e| failed("Invalid retained baseline identity.").caused_by(e))?;
        let mut baseline = native::read(&history, &self.baseline_leaf)?;
        if !expected_baseline.matches(&baseline)? {
            return Err(failed("Retained recovery baseline was replaced."));
        }
        self.record(
            PHASES[5],
            serde_json::json!({"old":self.old.identity,"historyLeaf":self.old_leaf}),
        )?;
        if interrupted(fault, Fault::AfterOldIntent) {
            return Ok(CopyOutcome::Interrupted);
        }
        let retained = if let Some(existing) = native::maybe_read(&history, &self.old_leaf)? {
            existing
        } else {
            let old = native::reopen_stage(&parent, "edited.jpg", &self.old.identity)?;
            if source::revision(&old)? != self.old.revision {
                return Err(failed(
                    "Reviewed destination version changed before preservation.",
                ));
            }
            let IoStep::Completed(witness) = compare(
                &PacedIo::shared(),
                &mut old.try_clone()?,
                &mut baseline,
                &self.comparison_file(&old)?,
                &AtomicBool::new(false),
            )?
            else {
                return Err(failed("Old version verification canceled."));
            };
            witness.revalidate(&history, &self.baseline_leaf, &baseline)?;
            if witness.digest != self.old_content_digest {
                return Err(failed(
                    "The prior version no longer matches its reviewed complete-byte manifest.",
                ));
            }
            native::publish_to(
                &parent,
                "edited.jpg",
                old,
                &history,
                &self.old_leaf,
                &self.old.identity,
            )?
        };
        if interrupted(fault, Fault::AfterOldMove) {
            return Ok(CopyOutcome::Interrupted);
        }
        if filesystem::identity(&retained)? != self.old.identity
            || retained.metadata()?.modified()? != self.modified
            || retained.metadata()?.created().ok() != self.created
        {
            return Err(failed(
                "Exact retained object or descriptive times changed.",
            ));
        }
        let IoStep::Completed(old_bytes) = compare(
            &PacedIo::shared(),
            &mut retained.try_clone()?,
            &mut baseline,
            &self.comparison_file(&retained)?,
            &AtomicBool::new(false),
        )?
        else {
            return Err(failed("Retained old version verification canceled."));
        };
        if old_bytes.digest != self.old_content_digest {
            return Err(failed(
                "The retained old version changed; replacement remains blocked.",
            ));
        }
        self.record(PHASES[6],serde_json::json!({"oldObject":Object::of(&retained)?,"fullByteDigest":old_bytes.digest}))?;
        if interrupted(fault, Fault::AfterOldRecorded) {
            return Ok(CopyOutcome::Interrupted);
        }
        let incoming_location =
            locations::Source::capture(&a, &source::decode_path(&self.incoming.path)?)?;
        let mut original = incoming_location.open(&a)?;
        let incoming_comparison = CopyFile {
            source: "reviewed incoming".into(),
            target: "edited.jpg".into(),
            stage: "incoming.bin".into(),
            object: Object {
                volume: self.incoming.identity.volume,
                file: self.incoming.identity.file,
            },
            revision: self.incoming.revision.clone(),
            bytes: self.incoming.bytes,
            source_location: Some(incoming_location),
            destination_location: Some(locations::Destination::capture(b.leaf(), &namespace)?),
            stage_location: Some(locations::Destination::capture(b.leaf(), &self.stage)?),
            prepared_stage: None,
        };
        self.record(
            PHASES[7],
            serde_json::json!({"incoming":expected_incoming,"oldRetained":self.old_leaf}),
        )?;
        if interrupted(fault, Fault::BeforeNewPublish) {
            return Ok(CopyOutcome::Interrupted);
        }
        let mut published = if let Some(existing) = native::maybe_read(&parent, "edited.jpg")? {
            if !expected_incoming.matches(&existing)? {
                return Err(failed(
                    "A late destination occupant is retained; no override is available.",
                ));
            }
            existing
        } else {
            let identity = filesystem::FileIdentity {
                volume: expected_incoming.volume,
                file: expected_incoming.file,
                links: 1,
            };
            let incoming = native::reopen_stage(&stage, "incoming.bin", &identity)?;
            let IoStep::Completed(witness) = compare(
                &PacedIo::shared(),
                &mut original,
                &mut incoming.try_clone()?,
                &incoming_comparison,
                &AtomicBool::new(false),
            )?
            else {
                return Err(failed("Incoming comparison canceled."));
            };
            witness.revalidate(&stage, "incoming.bin", &incoming)?;
            self.check()?;
            native::publish_to(
                &stage,
                "incoming.bin",
                incoming,
                &parent,
                "edited.jpg",
                &identity,
            )?
        };
        if interrupted(fault, Fault::AfterNewPublish) {
            return Ok(CopyOutcome::Interrupted);
        }
        let IoStep::Completed(new_bytes) = compare(
            &PacedIo::shared(),
            &mut original,
            &mut published,
            &incoming_comparison,
            &AtomicBool::new(false),
        )?
        else {
            return Err(failed("Published comparison canceled."));
        };
        new_bytes.revalidate(&parent, "edited.jpg", &published)?;
        self.check()?;
        self.record(
            PHASES[8],
            serde_json::json!({"object":Object::of(&published)?,"fullByteDigest":new_bytes.digest}),
        )?;
        self.record(
            PHASES[9],
            serde_json::json!({"files":1,"sourceRetained":true,"previousVersionsRetained":3}),
        )?;
        if interrupted(fault, Fault::AfterComplete) {
            return Ok(CopyOutcome::Interrupted);
        }
        Ok(CopyOutcome::CompletedVerified)
    }
    fn approved(&self, recovery: bool, fault: Option<Fault>) -> Result<CopyOutcome> {
        let expected = self.approval(recovery)?;
        let ticket = self.fixture.approvals.issue(expected.clone())?;
        self.execute(
            recovery,
            &expected,
            self.fixture.approvals.consume(&ticket, &expected)?,
            fault,
        )
    }
    fn restore_old_to_new_output(&self) -> Result<()> {
        let _lease = self.fixture.lease()?;
        self.check()?;
        let (_, dst) = self.fixture.paths()?;
        let chain = output_chain(&dst)?;
        let id = uuid::Uuid::new_v4().to_string();
        let name = format!(".stillharbour-restored-{id}");
        let digest = blake3::hash(
            &serde_json::to_vec(&(&self.digest, &self.old_content_digest, &name))
                .map_err(|e| failed("Could not freeze retained restore scope.").caused_by(e))?,
        )
        .to_hex()
        .to_string();
        let review = approval::Review::new(
            approval::Action::RestoreRetained,
            id.clone(),
            digest.clone(),
            self.fixture.selection.clone(),
        )?;
        let ticket = self.fixture.approvals.issue(review.clone())?;
        self.fixture
            .approvals
            .consume(&ticket, &review)?
            .authorize(&review)?;
        let mut preparation = self.seed.clone();
        preparation.id = id.clone();
        preparation.digest = preparation.digest()?;
        let path = self
            .fixture
            .owned
            .path()
            .join("data")
            .join(format!("restore-preparation-{id}.sqlite"));
        self.fixture.boundary(&path)?;
        let journal = Journal::create(&path, &preparation)?;
        journal.append(None,"restore_output_intent",serde_json::json!({"reviewDigest":digest,"name":name,"oldContentDigest":self.old_content_digest}))?;
        let restored = native::create_directory(chain.leaf(), &name)?;
        journal.append(
            None,
            "restore_output_ready",
            serde_json::json!({"object":Object::of(&restored)?}),
        )?;
        let relative = PathBuf::from(&self.history).join(&self.old_leaf);
        self.prepared_copy(
            chain.leaf(),
            chain.leaf(),
            &relative,
            &name,
            "recovered-old.bin",
            &format!("old-restore-{id}.sqlite"),
        )?;
        let file = native::read(&restored, "recovered-old.bin")?;
        let history = native::read_directory(chain.leaf(), &self.history)?;
        let mut old = native::read(&history, &self.old_leaf)?;
        let comparison = self.comparison_file(&old)?;
        let IoStep::Completed(witness) = compare(
            &PacedIo::shared(),
            &mut old,
            &mut file.try_clone()?,
            &comparison,
            &AtomicBool::new(false),
        )?
        else {
            return Err(failed("Retained-version restore comparison canceled."));
        };
        if witness.digest != self.old_content_digest {
            return Err(failed(
                "Restored old version differs from the independent baseline.",
            ));
        }
        witness.revalidate(&restored, "recovered-old.bin", &file)?;
        journal.append(
            None,
            "restore_completed",
            serde_json::json!({"fullByteDigest":witness.digest,"sourcePreserved":true}),
        )?;
        self.audit()
    }
    fn audit(&self) -> Result<()> {
        self.check()?;
        for (path, bytes) in &self.fixture.originals {
            if std::fs::read(path)? != *bytes {
                return Err(failed("Replacement source changed."));
            }
        }
        for (path, bytes) in &self.fixture.neighbors {
            if path.file_name().is_some_and(|name| name == "edited.jpg") {
                continue;
            }
            if std::fs::read(path)? != *bytes {
                return Err(failed("Replacement destination-only neighbor changed."));
            }
        }
        let (_, dst) = self.fixture.paths()?;
        let b = output_chain(&dst)?;
        let history = native::read_directory(b.leaf(), &self.history)?;
        if let Some(file) = native::maybe_read(&history, &self.old_leaf)? {
            let mut baseline = native::read(&history, &self.baseline_leaf)?;
            if filesystem::identity(&file)? != self.old.identity {
                return Err(failed("Retained old object changed."));
            }
            let IoStep::Completed(witness) = compare(
                &PacedIo::shared(),
                &mut file.try_clone()?,
                &mut baseline,
                &self.comparison_file(&file)?,
                &AtomicBool::new(false),
            )?
            else {
                return Err(failed("Retained old-version audit canceled."));
            };
            if witness.digest != self.old_content_digest {
                return Err(failed(
                    "Retained old-version audit differs from the frozen byte manifest.",
                ));
            }
        }
        Ok(())
    }
}

pub(super) fn probe() -> Result<serde_json::Value> {
    let lab = Lab::new()?;
    if lab.approved(false, None)? != CopyOutcome::CompletedVerified {
        return Err(failed("Generated retained replacement did not complete."));
    }
    lab.audit()?;
    lab.restore_old_to_new_output()?;
    Ok(
        serde_json::json!({"scope":"constructor-owned-generated-only","nativeOs":std::env::consts::OS,"replacedFiles":1,"priorVersionsRetained":3,"sameOldNativeObjectRetained":true,"oldBytesVerified":true,"newBytesVerified":true,"oldRestoredToNewOutput":true,"sourcePreserved":true,"destinationOnlyPreserved":true,"independentRecoveryVerified":false,"productionExecutionAvailable":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const FAULTS: [Fault; 7] = [
        Fault::AfterPrepared,
        Fault::AfterOldIntent,
        Fault::AfterOldMove,
        Fault::AfterOldRecorded,
        Fault::BeforeNewPublish,
        Fault::AfterNewPublish,
        Fault::AfterComplete,
    ];
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4c_retained_cold_child() {
        let Some(root) = std::env::var_os("STILLHARBOUR_M4C_ROOT") else {
            return;
        };
        let root = PathBuf::from(root).canonicalize().unwrap();
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("stillharbour-m4b-")
        );
        let token = std::env::var("STILLHARBOUR_M4C_TOKEN").unwrap();
        assert_eq!(
            std::fs::read(root.join("fixture-owner")).unwrap(),
            token.as_bytes()
        );
        let path = root.join("data/replacement-control.json");
        assert!(
            !std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(std::fs::metadata(&path).unwrap().len() < 32 * 1024 * 1024);
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(
            blake3::hash(&bytes).to_hex().as_str(),
            std::env::var("STILLHARBOUR_M4C_HASH").unwrap()
        );
        let lab = Lab::from_control(root, serde_json::from_slice(&bytes).unwrap(), &token).unwrap();
        let selected = std::env::var("STILLHARBOUR_M4C_FAULT").unwrap();
        let fault = FAULTS
            .into_iter()
            .find(|f| format!("{f:?}") == selected)
            .unwrap();
        COLD_EXIT.with(|flag| flag.set(true));
        let _ = lab.approved(false, Some(fault)).unwrap();
        panic!("Cold replacement child missed requested boundary");
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4c_actual_child_exit_reconciles_all_seven_replacement_boundaries() {
        for fault in FAULTS {
            let lab = Lab::new().unwrap();
            let bytes = serde_json::to_vec(&lab.control()).unwrap();
            let path = lab
                .fixture
                .owned
                .path()
                .join("data/replacement-control.json");
            lab.fixture.boundary(&path).unwrap();
            std::fs::write(path, &bytes).unwrap();
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "executor::append_fixture::retained::tests::m4c_retained_cold_child",
                    "--nocapture",
                ])
                .env("STILLHARBOUR_M4C_ROOT", lab.fixture.owned.path())
                .env("STILLHARBOUR_M4C_TOKEN", &lab.fixture.selection)
                .env(
                    "STILLHARBOUR_M4C_HASH",
                    blake3::hash(&bytes).to_hex().as_str(),
                )
                .env("STILLHARBOUR_M4C_FAULT", format!("{fault:?}"))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let stderr = crate::test_process::drain(child.stderr.take().unwrap());
            // CI exceeded the old 20-second budget without diagnostics. Allow
            // bounded scheduling headroom and retain the actual exit assertion.
            let deadline = Instant::now() + Duration::from_secs(60);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() > deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!(
                        "Cold replacement child timed out at {fault:?}: {}",
                        stderr.finish()
                    );
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            assert_eq!(status.code(), Some(79), "{fault:?}: {}", stderr.finish());
            lab.audit().unwrap();
            assert!(lab.approved(false, None).is_err());
            assert_eq!(
                lab.approved(true, None).unwrap(),
                CopyOutcome::CompletedVerified,
                "{fault:?}"
            );
            lab.audit().unwrap();
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4c_retained_replacement_preserves_three_old_versions_and_both_sources() {
        let receipt = probe().unwrap();
        assert_eq!(receipt["priorVersionsRetained"], 3);
        assert_eq!(receipt["sameOldNativeObjectRetained"], true);
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4c_retained_replacement_reconciles_uncertain_moves_before_fresh_approved_continuation() {
        for fault in FAULTS {
            let lab = Lab::new().unwrap();
            assert_eq!(
                lab.approved(false, Some(fault)).unwrap(),
                CopyOutcome::Interrupted,
                "{fault:?}"
            );
            lab.audit().unwrap();
            let count = lab.journal().unwrap().history_count().unwrap();
            assert!(lab.approved(false, None).is_err());
            assert_eq!(lab.journal().unwrap().history_count().unwrap(), count);
            assert_eq!(
                lab.approved(true, None).unwrap(),
                CopyOutcome::CompletedVerified,
                "{fault:?}"
            );
            lab.audit().unwrap();
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4c_late_new_destination_after_old_preservation_keeps_every_version() {
        let lab = Lab::new().unwrap();
        lab.approved(false, Some(Fault::AfterOldRecorded)).unwrap();
        let (_, dst) = lab.fixture.paths().unwrap();
        let path = dst
            .join(source::decode_path(&lab.fixture.projection.namespace).unwrap())
            .join("edited.jpg");
        lab.fixture.boundary(&path).unwrap();
        std::fs::write(&path, b"late unique version").unwrap();
        assert!(lab.approved(true, None).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"late unique version");
        lab.audit().unwrap();
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4c_corrupt_baseline_blocks_old_move_and_preserves_target() {
        let lab = Lab::new().unwrap();
        lab.approved(false, Some(Fault::AfterPrepared)).unwrap();
        let (_, dst) = lab.fixture.paths().unwrap();
        let baseline = dst.join(&lab.history).join(&lab.baseline_leaf);
        lab.fixture.boundary(&baseline).unwrap();
        std::fs::write(baseline, b"corrupt recovery baseline").unwrap();
        assert!(lab.approved(true, None).is_err());
        let old = dst
            .join(source::decode_path(&lab.fixture.projection.namespace).unwrap())
            .join("edited.jpg");
        assert_eq!(
            blake3::hash(&std::fs::read(old).unwrap()).to_hex().as_str(),
            lab.old_content_digest
        );
        assert!(!dst.join(&lab.history).join(&lab.old_leaf).exists());
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn m4c_history_collision_preserves_every_object_and_blocks_publication() {
        let lab = Lab::new().unwrap();
        let (_, dst) = lab.fixture.paths().unwrap();
        let collision = dst.join(&lab.history).join(&lab.old_leaf);
        lab.fixture.boundary(&collision).unwrap();
        std::fs::write(&collision, b"unrelated history occupant").unwrap();
        assert!(lab.approved(false, None).is_err());
        assert_eq!(
            std::fs::read(collision).unwrap(),
            b"unrelated history occupant"
        );
        let old = dst
            .join(source::decode_path(&lab.fixture.projection.namespace).unwrap())
            .join("edited.jpg");
        assert_eq!(
            blake3::hash(&std::fs::read(old).unwrap()).to_hex().as_str(),
            lab.old_content_digest
        );
    }
}
