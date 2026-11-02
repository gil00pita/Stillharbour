//! Reviewed continuation of an existing copy stream. Never erase a retained
//! stage, allocate a replacement stream or treat old verification as fresh.
use super::*;

#[derive(Clone)]
pub(super) struct Cursor {
    pub phase: u8,
    pub object: Option<Object>,
    pub digest: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Observation {
    object: Object,
    revision: String,
    bytes: u64,
}
impl Observation {
    fn capture(file: &File) -> Result<Self> {
        let identity = filesystem::identity(file)?;
        if identity.links != 1 || !file.metadata()?.is_file() {
            return Err(failed(
                "Recovery requires the exact ordinary single-link output object.",
            ));
        }
        Ok(Self {
            object: Object::of(file)?,
            revision: source_native::revision(file)?,
            bytes: file.metadata()?.len(),
        })
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Member {
    stage: Option<Observation>,
    published: Option<Observation>,
}
#[derive(Clone)]
pub(super) struct Review {
    history_digest: String,
    members: Vec<Member>,
    digest: String,
}
impl Review {
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn digest(&self) -> &str {
        &self.digest
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn observations(&self) -> Vec<(Option<u64>, bool)> {
        self.members
            .iter()
            .map(|member| {
                (
                    member.stage.as_ref().map(|stage| stage.bytes),
                    member.published.is_some(),
                )
            })
            .collect()
    }
    #[cfg(any(test, feature = "copy-workflow"))]
    pub(super) fn record(&self, plan: &CopyPlan) -> serde_json::Value {
        serde_json::json!({"policy":"reviewed-copy-continuation-v1","planDigest":plan.digest,"historyDigest":self.history_digest,"members":self.members,"digest":self.digest})
    }
    pub(super) fn approval(&self, plan: &CopyPlan, selection: &str) -> Result<approval::Review> {
        approval::Review::new(
            approval::Action::ResumeFiles,
            plan.id.clone(),
            self.digest.clone(),
            selection.into(),
        )
    }
    pub(super) fn capture(
        plan: &CopyPlan,
        cursors: &[Cursor],
        history_digest: String,
        inputs: &CopyInputs<'_>,
    ) -> Result<Self> {
        if cursors.len() != plan.files.len()
            || plan.digest()? != plan.digest
            || !plan.source.matches(inputs.source_root)?
            || !plan.destination.matches(inputs.destination)?
            || source_native::volume_token(inputs.source_root)? != plan.source_volume
            || source_native::volume_token(inputs.destination)? != plan.destination_volume
        {
            return Err(failed(
                "Recovery scope differs from its immutable copy plan.",
            ));
        }
        (inputs.validate_scope)()?;
        let mut members = Vec::with_capacity(cursors.len());
        for (file, cursor) in plan.files.iter().zip(cursors) {
            source_member(inputs.source_root, file)?;
            let parent = destination_parent(inputs.destination, file)?;
            let staging = staging_parent(inputs.destination, file)?;
            let stage = native::maybe_read(&staging, &file.stage)?
                .as_ref()
                .map(Observation::capture)
                .transpose()?;
            let published = native::maybe_read(&parent, &file.target)?
                .as_ref()
                .map(Observation::capture)
                .transpose()?;
            let member = Member { stage, published };
            member.validate(file, cursor)?;
            destination_member(inputs.destination, file, &parent)?;
            staging_member(inputs.destination, file, &staging)?;
            members.push(member);
        }
        (inputs.validate_scope)()?;
        let encoded=serde_json::to_vec(&serde_json::json!({"policy":"reviewed-copy-continuation-v1","plan":plan.digest,"history":history_digest,"members":members}))
            .map_err(|error|failed("Could not encode recovery review.").caused_by(error))?;
        if encoded.len() > crate::operations::MAX_PLAN_BYTES {
            return Err(failed("Recovery review exceeds its bound."));
        }
        let digest = blake3::hash(&encoded).to_hex().to_string();
        Ok(Self {
            history_digest,
            members,
            digest,
        })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg(any(test, feature = "copy-workflow"))]
struct RecordedReview {
    policy: String,
    plan_digest: String,
    history_digest: String,
    members: Vec<Member>,
    digest: String,
}
#[cfg(any(test, feature = "copy-workflow"))]
pub(super) fn validate_record(
    value: serde_json::Value,
    plan: &CopyPlan,
    digest: &str,
) -> Result<()> {
    let record: RecordedReview = serde_json::from_value(value)
        .map_err(|error| failed("Invalid retained continuation observations.").caused_by(error))?;
    if record.policy != "reviewed-copy-continuation-v1"
        || record.plan_digest != plan.digest
        || record.members.len() != plan.files.len()
        || record.history_digest.len() != 64
        || !record.history_digest.bytes().all(|b| b.is_ascii_hexdigit())
        || record.digest != digest
    {
        return Err(failed(
            "Retained continuation scope differs from its copy plan.",
        ));
    }
    for (member, file) in record.members.iter().zip(&plan.files) {
        let observed = match (&member.stage, &member.published) {
            (Some(stage), None) | (None, Some(stage)) => stage,
            _ => return Err(failed("Ambiguous retained continuation observations.")),
        };
        let expected = file.prepared_stage.as_ref().ok_or_else(|| {
            failed("Application continuation requires prepared stage identities.")
        })?;
        if observed.object != expected.object
            || observed.bytes > file.bytes
            || (member.published.is_some() && observed.bytes != file.bytes)
            || observed.revision.len() != 64
            || !observed.revision.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(failed(
                "Retained continuation object observations differ from the prepared plan.",
            ));
        }
    }
    let encoded=serde_json::to_vec(&serde_json::json!({"policy":record.policy,"plan":plan.digest,"history":record.history_digest,"members":record.members}))
        .map_err(|error|failed("Could not validate retained continuation digest.").caused_by(error))?;
    if blake3::hash(&encoded).to_hex().as_str() != digest {
        return Err(failed("Retained continuation digest changed."));
    }
    Ok(())
}
impl Member {
    fn validate(&self, file: &CopyFile, cursor: &Cursor) -> Result<()> {
        if cursor.phase > 6 || (self.stage.is_some() && self.published.is_some()) {
            return Err(failed(
                "Recovery found ambiguous stage/target occupants; retain both.",
            ));
        }
        match cursor.phase {
            0 | 1 => {
                if cursor.object.is_some() || cursor.digest.is_some() || self.published.is_some() {
                    return Err(failed(
                        "Unstarted recovery scope has unexpected history or a target occupant.",
                    ));
                }
                match (&self.stage, &file.prepared_stage) {
                    (Some(stage), Some(prepared))
                        if stage.object == prepared.object
                            && stage.revision == prepared.revision
                            && stage.bytes == 0 => {}
                    (None, None) => {}
                    _ => {
                        return Err(failed(
                            "Recovery cannot adopt an unrecorded, changed or missing stage.",
                        ));
                    }
                }
            }
            2..=6 => {
                let expected = cursor
                    .object
                    .as_ref()
                    .ok_or_else(|| failed("Recovery journal lacks the retained stage identity."))?;
                let occupant = self
                    .stage
                    .as_ref()
                    .or(self.published.as_ref())
                    .ok_or_else(|| failed("The recorded recovery object is missing."))?;
                if &occupant.object != expected
                    || occupant.bytes > file.bytes
                    || (cursor.phase >= 3 && occupant.bytes != file.bytes)
                    || (cursor.phase < 5 && self.published.is_some())
                    || (cursor.phase == 6 && self.stage.is_some())
                    || (cursor.phase >= 4 && cursor.digest.is_none())
                {
                    return Err(failed(
                        "Recovery object differs from its journaled identity, size or phase.",
                    ));
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

pub(super) struct Continuation<'a> {
    pub plan: &'a CopyPlan,
    pub journal: &'a dyn CopyJournal,
    pub cursors: &'a [Cursor],
    pub terminal: bool,
    pub review: &'a Review,
    pub history_digest: String,
    pub inputs: &'a CopyInputs<'a>,
    pub fault: Option<CopyFault>,
    pub progress: Option<&'a dyn Fn(u32) -> Result<()>>,
}
impl Continuation<'_> {
    pub(super) fn execute(self) -> Result<CopyOutcome> {
        let fresh = Review::capture(
            self.plan,
            self.cursors,
            self.history_digest.clone(),
            self.inputs,
        )?;
        if fresh.digest != self.review.digest
            || fresh.members != self.review.members
            || fresh.history_digest != self.review.history_digest
        {
            return Err(failed(
                "Recovery observations or journal changed after review; approve a fresh review.",
            ));
        }
        let mut completed = Vec::with_capacity(self.plan.files.len());
        for (index, (file, cursor)) in self.plan.files.iter().zip(self.cursors).enumerate() {
            (self.inputs.validate_scope)()?;
            if self.inputs.cancel.load(Ordering::Acquire) {
                return Ok(CopyOutcome::CanceledWithPartialCopies);
            }
            let parent = destination_parent(self.inputs.destination, file)?;
            let staging = staging_parent(self.inputs.destination, file)?;
            let mut source = read_source(self.inputs.source_root, file)?;
            revision(&source, file)?;
            let mut phase = cursor.phase;
            let mut object = cursor.object.clone();
            let mut expected_digest = cursor.digest.clone();
            if phase == 0 {
                self.journal.append(Some(index),"item_intent",serde_json::json!({"stage":file.stage,"target":file.target,"sourceRevision":file.revision}))?;
                phase = 1;
                if stop(self.fault, CopyFault::AfterIntent) {
                    return Ok(CopyOutcome::Interrupted);
                }
            }
            if phase == 1 {
                let stage = if let Some(prepared) = &file.prepared_stage {
                    let stage = native::reopen_stage(&staging, &file.stage, &prepared.identity())?;
                    prepared.validate_empty(&staging, &file.stage, &stage)?;
                    stage
                } else {
                    native::create_file(&staging, &file.stage)?
                };
                let observed = Observation::capture(&stage)?;
                if observed.bytes != 0 {
                    return Err(failed("An unstarted stage is not empty."));
                }
                object = Some(observed.object.clone());
                self.journal.append(
                    Some(index),
                    "stage_created",
                    serde_json::to_value(observed.object)
                        .map_err(|e| failed("Could not encode stage identity.").caused_by(e))?,
                )?;
                phase = 2;
                if stop(self.fault, CopyFault::AfterStageCreation) {
                    return Ok(CopyOutcome::Interrupted);
                }
            }
            let object =
                object.ok_or_else(|| failed("Recovery lacks an owned output identity."))?;
            let identity = filesystem::FileIdentity {
                volume: object.volume,
                file: object.file,
                links: 1,
            };
            let mut published = if fresh.members[index].published.is_some() {
                if phase < 5 {
                    return Err(failed("Publication has no retained intent."));
                }
                let published = native::read(&parent, &file.target)?;
                if fresh.members[index].published.as_ref()
                    != Some(&Observation::capture(&published)?)
                {
                    return Err(failed(
                        "Retained publication changed after recovery review.",
                    ));
                }
                published
            } else {
                let mut stage = native::reopen_append_stage(&staging, &file.stage, &identity)?;
                if cursor.phase >= 2
                    && fresh.members[index].stage.as_ref() != Some(&Observation::capture(&stage)?)
                {
                    return Err(failed("Retained stage changed after recovery review."));
                }
                if phase == 2 {
                    match append_verified_prefix(
                        &mut source,
                        &mut stage,
                        file,
                        &staging,
                        self.inputs,
                        self.fault,
                    )? {
                        AppendOutcome::Complete => {}
                        AppendOutcome::Canceled => {
                            return Ok(CopyOutcome::CanceledWithPartialCopies);
                        }
                        AppendOutcome::Interrupted => return Ok(CopyOutcome::Interrupted),
                    }
                    revision(&source, file)?;
                    stage.sync_all()?;
                    drop(stage);
                    self.journal.append(
                        Some(index),
                        "stage_flushed",
                        serde_json::json!({"bytes":file.bytes.to_string()}),
                    )?;
                    phase = 3;
                    if stop(self.fault, CopyFault::AfterFlush) {
                        return Ok(CopyOutcome::Interrupted);
                    }
                    stage = native::reopen_append_stage(&staging, &file.stage, &identity)?;
                }
                let IoStep::Completed(witness) = compare(
                    self.inputs.io,
                    &mut source,
                    &mut stage,
                    file,
                    self.inputs.cancel,
                )?
                else {
                    return Ok(CopyOutcome::CanceledWithPartialCopies);
                };
                if cursor
                    .digest
                    .as_ref()
                    .is_some_and(|old| old != &witness.digest)
                {
                    return Err(failed(
                        "Retained verification digest differs from fresh complete bytes.",
                    ));
                }
                expected_digest = Some(witness.digest.clone());
                if phase == 3 {
                    self.journal.append(Some(index),"stage_verified",serde_json::json!({"bytes":file.bytes.to_string(),"digest":witness.digest,"copyRevision":witness.copy_revision}))?;
                    phase = 4;
                    if stop(self.fault, CopyFault::AfterVerification) {
                        return Ok(CopyOutcome::Interrupted);
                    }
                }
                (self.inputs.validate_scope)()?;
                source_member(self.inputs.source_root, file)?;
                destination_member(self.inputs.destination, file, &parent)?;
                staging_member(self.inputs.destination, file, &staging)?;
                witness.revalidate(&staging, &file.stage, &stage)?;
                if self.inputs.cancel.load(Ordering::Acquire) {
                    return Ok(CopyOutcome::CanceledWithPartialCopies);
                }
                if phase == 4 {
                    self.journal.append(Some(index),"publish_intent",serde_json::json!({"target":file.target,"stage":file.stage,"digest":witness.digest}))?;
                    phase = 5;
                }
                if stop(self.fault, CopyFault::BeforePublish) {
                    return Ok(CopyOutcome::Interrupted);
                }
                let published = native::publish_to(
                    &staging,
                    &file.stage,
                    stage,
                    &parent,
                    &file.target,
                    &identity,
                )?;
                if stop(self.fault, CopyFault::AfterPublish) {
                    return Ok(CopyOutcome::Interrupted);
                }
                if self.fault == Some(CopyFault::ReadOnlyJournalAfterPublish) {
                    self.journal.deny_writes_for_fixture_fault()?;
                }
                published
            };
            if filesystem::identity(&published)? != identity {
                return Err(failed("Recovered publication has another identity."));
            }
            let IoStep::Completed(witness) = compare(
                self.inputs.io,
                &mut source,
                &mut published,
                file,
                self.inputs.cancel,
            )?
            else {
                return Ok(CopyOutcome::CanceledWithPartialCopies);
            };
            if expected_digest
                .as_ref()
                .is_some_and(|digest| digest != &witness.digest)
            {
                return Err(failed(
                    "Published bytes differ from the retained verification digest.",
                ));
            }
            (self.inputs.validate_scope)()?;
            source_member(self.inputs.source_root, file)?;
            destination_member(self.inputs.destination, file, &parent)?;
            witness.revalidate(&parent, &file.target, &published)?;
            if phase == 5 {
                self.journal.append(Some(index),"item_completed_verified",serde_json::json!({"digest":witness.digest,"copyRevision":witness.copy_revision,"object":object}))?;
                if stop(self.fault, CopyFault::AfterItemCommit) {
                    return Ok(CopyOutcome::Interrupted);
                }
            }
            completed.push(witness);
            if let Some(progress) = self.progress {
                progress((index + 1) as u32)?;
            }
        }
        (self.inputs.validate_scope)()?;
        for (file, witness) in self.plan.files.iter().zip(completed) {
            source_member(self.inputs.source_root, file)?;
            let parent = destination_parent(self.inputs.destination, file)?;
            destination_member(self.inputs.destination, file, &parent)?;
            witness.revalidate(&parent, &file.target, &native::read(&parent, &file.target)?)?;
        }
        if !self.terminal {
            self.journal.append(
                None,
                "plan_completed_verified",
                serde_json::json!({"files":self.plan.files.len()}),
            )?;
            if stop(self.fault, CopyFault::AfterPlanCommit) {
                return Ok(CopyOutcome::Interrupted);
            }
        }
        Ok(CopyOutcome::CompletedVerified)
    }
}

/// Compare every retained prefix byte before adding bytes. Unix opens use
/// O_APPEND; Windows keeps a handle denying concurrent writers and deleters.
enum AppendOutcome {
    Complete,
    Canceled,
    Interrupted,
}
fn append_verified_prefix(
    source: &mut File,
    stage: &mut File,
    plan: &CopyFile,
    parent: &File,
    inputs: &CopyInputs<'_>,
    fault: Option<CopyFault>,
) -> Result<AppendOutcome> {
    let before = Observation::capture(stage)?;
    if before.bytes > plan.bytes {
        return Err(failed("Retained stage exceeds the reviewed source size."));
    }
    source.seek(SeekFrom::Start(0))?;
    stage.seek(SeekFrom::Start(0))?;
    let mut left = [0u8; CHUNK_BYTES];
    let mut right = [0u8; CHUNK_BYTES];
    let mut total = 0u64;
    while total < before.bytes {
        let n = (before.bytes - total).min(CHUNK_BYTES as u64) as usize;
        let IoStep::Completed(a) = read_block(inputs.io, source, &mut left[..n], inputs.cancel)?
        else {
            return Ok(AppendOutcome::Canceled);
        };
        let IoStep::Completed(b) = read_block(inputs.io, stage, &mut right[..n], inputs.cancel)?
        else {
            return Ok(AppendOutcome::Canceled);
        };
        if a != n || b != n || left[..n] != right[..n] {
            return Err(failed(
                "Retained stage prefix differs from its source. No bytes were appended.",
            ));
        }
        total += n as u64;
    }
    if Observation::capture(stage)? != before {
        return Err(failed("Retained stage changed during prefix verification."));
    }
    source_member(inputs.source_root, plan)?;
    (inputs.validate_scope)()?;
    if !native::entry_matches(parent, &plan.stage, &filesystem::identity(stage)?)? {
        return Err(failed("Retained stage changed membership before append."));
    }
    stage.seek(SeekFrom::End(0))?;
    while total < plan.bytes {
        let n = (plan.bytes - total).min(CHUNK_BYTES as u64) as usize;
        let IoStep::Completed(count) =
            read_block(inputs.io, source, &mut left[..n], inputs.cancel)?
        else {
            return Ok(AppendOutcome::Canceled);
        };
        if count != n {
            return Err(failed("Source ended before the reviewed copy size."));
        }
        if stage.metadata()?.len() != total {
            return Err(failed("Retained stage changed before append."));
        }
        if inputs.io.write_all(stage, &left[..n], inputs.cancel)? == IoStep::Canceled {
            return Ok(AppendOutcome::Canceled);
        }
        total += n as u64;
        if fault == Some(CopyFault::InjectedDiskFullAfterChunk) {
            return Err(std::io::Error::from(std::io::ErrorKind::StorageFull).into());
        }
        if stop(fault, CopyFault::AfterCopyChunk) {
            stage.sync_all()?;
            return Ok(AppendOutcome::Interrupted);
        }
    }
    Ok(AppendOutcome::Complete)
}
