# Stillharbour - Codex Backup & Sync prompt

All repository paths in this document are relative to the repository root.

Extend Stillharbour with Backup & Sync. Preserve every existing file version; speed and convenience never override data safety.

Read AGENTS.md, docs/CODEX_SPEC.md, docs/PRESERVATION_CONTRACT.md,
docs/BACKUP_SYNC_ADDENDUM.md, docs/PRESERVATION_TESTS.md,
docs/DECISIONS.md, docs/ACCEPTANCE_TESTS.md,
tasks/M4_BACKUP_RESTORE.md and the implementation log before changes.
Inspect the actual repository and respect existing architecture and work.

This package is a specification, not proof that M0-M3 have been implemented.
If no app exists, follow docs/CODEX_START_PROMPT.md and implement M0 only.
If foundations exist but M2 recovery/execution gates do not, report the gap
and complete the relevant foundation before enabling file mutation.

Source is the configured base library, pinned by root UUID. The user picks
only the destination for ordinary use. Additional roots are explicit opt-in.
Implement Compare -> Review changes -> Safe sync, separate from encrypted
restic snapshot backup. Retain destination-only files. Do not delete sources,
prune versions, enable two-way sync, or silently mirror deletions.

Begin with M4A only: read-only comparison, typed plans/issues/capabilities,
fixed-source and destination UI, meaningful conflict previews, preservation
policy enforcement and tests with generated temporary fixtures. If M4A already
exists, assess its evidence and stop at the next approval gate rather than
assuming all later phases are authorized.

Expose meaningful issue actions: Compare, Keep both, Keep destination,
Replace destination and retain previous version, Retry, Pause/reconnect,
and explicitly tracked Skip. Offer only actions valid for that issue and
adapter. Never implement a universal Force/Ignore safety bypass.

Design bounded chunk pacing, global/per-device budgets, low concurrency,
and separate upload/download/read/write verification accounting. The initial
Quiet preset is 10 MiB/s with one transfer; treat this as a proposed default,
not benchmark evidence. Mark unsupported helper/physical-I/O controls honestly.

Later approved execution must use exclusive staging, full-content read-back,
safe publication, retained old destination versions, durable recovery journals,
and strict preconditions. Source mutation additionally requires verified
independent recovery evidence for the exact affected version. Native metadata
and companions matter; a hash of the main data stream is not full fidelity.

Use no real photo folders as test data. No destructive helper shortcuts,
no mock-success production paths, no undocumented privilege elevation,
no direct restic overwrite restore into an active library.

Report actual files changed, commands run, test results, unresolved safety
risks and untested native/protocol behavior. Distinguish proposed tests from
executed tests. Stop after M4A for review before enabling transfer/replacement.

## Selected name - specification 1.2.1

Use **Stillharbour** exactly as spelled. Read docs/PRODUCT_IDENTITY.md for the descriptor, unchanged supplied logo and catalog filename convention. Keep existing persisted identifiers and user data intact; this naming revision does not authorize migrations or expand the milestone approved by this prompt.
