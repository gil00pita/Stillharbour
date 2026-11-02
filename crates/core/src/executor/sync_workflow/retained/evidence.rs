//! Authority comes only from an immutable executor binding and its native journal.
use super::*;
use std::fs::File;

pub(crate) struct RetainedRecoveryScope<'a> {
    workflow: &'a SyncWorkflow,
    library: &'a Library,
    binding: &'a Binding,
    originals: Vec<RecoveryFile>,
}

pub(crate) struct RetainedRecoveryObject {
    index: usize,
    file: File,
    revision: String,
}
impl RetainedRecoveryObject {
    pub(crate) fn index(&self) -> usize {
        self.index
    }
    pub(crate) fn file(&self) -> &File {
        &self.file
    }
    pub(crate) fn revision(&self) -> &str {
        &self.revision
    }
}
impl<'a> RetainedRecoveryScope<'a> {
    pub(in crate::executor::sync_workflow) fn new(
        workflow: &'a SyncWorkflow,
        library: &'a Library,
        binding: &'a Binding,
    ) -> Result<Self> {
        let originals = recovery_files(binding)?;
        if originals.is_empty() || originals.len() > 10_000 {
            return Err(failed(
                "Retained recovery requires a bounded complete OLD group.",
            ));
        }
        let scope = Self {
            workflow,
            library,
            binding,
            originals,
        };
        scope.current()?;
        Ok(scope)
    }
    pub(crate) fn originals(&self) -> &[RecoveryFile] {
        &self.originals
    }
    pub(crate) fn current(&self) -> Result<Vec<RetainedRecoveryObject>> {
        self.workflow.validate_binding(self.library, self.binding)?;
        let retained = self
            .binding
            .retained
            .as_ref()
            .filter(|r| r.origin.is_none())
            .ok_or_else(|| {
                failed("Only original OLD replacement groups admit retained recovery evidence.")
            })?;
        let facts = self
            .workflow
            .retained_facts(&retained.operation_id, self.binding)?;
        let root = output_chain(&source::decode_path(&self.binding.scope.destination)?)?;
        let history = facts
            .directory
            .as_ref()
            .map(|expected| {
                let directory =
                    native::read_directory(root.leaf(), &folder(&retained.operation_id))?;
                if !expected.matches(&directory)? {
                    return Err(failed("Retained evidence directory identity changed."));
                }
                Ok(directory)
            })
            .transpose()?;
        let mut result = Vec::with_capacity(retained.versions.len());
        for (index, version) in retained.versions.iter().enumerate() {
            let (parent, leaf) = open_parent(root.leaf(), &self.binding.scope, &version.old)?;
            let expected = Object {
                volume: version.old.identity.volume,
                file: version.old.identity.file,
            };
            let original = native::lookup(&parent, &leaf)?;
            let moved = history
                .as_ref()
                .map(|h| native::lookup(h, &old_leaf(index)))
                .transpose()?
                .flatten();
            let at_original = original
                .as_ref()
                .is_some_and(|e| e.kind == native::EntryKind::File && e.object == expected);
            let at_history = moved
                .as_ref()
                .is_some_and(|e| e.kind == native::EntryKind::File && e.object == expected);
            if at_original && at_history {
                return Err(failed(
                    "Ambiguous OLD object exists at both original and retained locations.",
                ));
            }
            let file = if at_history && facts.old_intents.contains_key(&index) {
                native::reopen_stage(
                    history
                        .as_ref()
                        .ok_or_else(|| failed("Missing retained evidence directory."))?,
                    &old_leaf(index),
                    &version.old.identity,
                )?
            } else if at_original && !facts.old_done.contains_key(&index) && moved.is_none() {
                native::reopen_stage(&parent, &leaf, &version.old.identity)?
            } else {
                return Err(failed(
                    "No uniquely journal-owned OLD recovery object is available.",
                ));
            };
            let revision = source::revision(&file)?;
            if filesystem::identity(&file)? != version.old.identity
                || file.metadata()?.len() != version.old.bytes
                || hash(&file, &AtomicBool::new(false))? != version.old_digest
                || (at_original && revision != version.old.revision)
                || facts
                    .old_done
                    .get(&index)
                    .is_some_and(|(object, recorded)| *object != expected || recorded != &revision)
            {
                return Err(failed(
                    "Journal-owned OLD recovery bytes or native revision changed.",
                ));
            }
            version
                .metadata
                .as_ref()
                .ok_or_else(|| failed("OLD native metadata is unavailable."))?
                .compare(&file)?;
            if source::revision(&file)? != revision {
                return Err(failed("OLD recovery object changed during inspection."));
            }
            result.push(RetainedRecoveryObject {
                index,
                file,
                revision,
            });
        }
        if result.len() != self.originals.len() || result.is_empty() {
            return Err(failed("Incomplete OLD recovery group."));
        }
        Ok(result)
    }
}
