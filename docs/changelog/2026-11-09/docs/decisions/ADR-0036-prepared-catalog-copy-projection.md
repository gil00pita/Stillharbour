> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0036 — Project prepared catalog stages into exact copy streams

Date: 2026-10-31. Status: constructor-owned fixture integration; production ready-plan/session approval remains open.

## Decision

Allow the catalog adapter to consume identified preparation stages only through a separate optional namespace reference. The reference binds the frozen UUID, digest of its exact immutable anchor and completed preparation fingerprint. Omit the new field for legacy bindings so their encoding remains unchanged. Direct-destination compatibility bindings still reject prepared stages.

Load one bounded namespace anchor and its bounded hash-linked facts through the catalog actor. Reject missing/duplicate/foreign/recursive anchors, incomplete history, mismatched manifest file IDs/paths and changed fingerprints. Revalidate the original catalog/draft/manifest/managed-root binding. Project a new immutable copy plan only from complete conflict-free preparation: copy the exact original source identity/revision/native locator/size, replace the destination with the owned namespace identity and derive every target/stage/parent identity chain from retained preparation. Check stage names against the frozen UUID and target index. No renderer-supplied target, source, stage or parent overrides enter this projection.

Compare the entire submitted copy plan, including recomputed digest, with that projection before inserting its copy anchor. Admit at most 64 retained copy streams per draft and one stream per prepared namespace, transactionally; reopening the same stream remains allowed for inspection. A new UUID must not bypass partial-copy/replay guards. Existing copy events stay separate from preparation facts. This is evidence binding, not user approval. Neither imported catalog history nor namespace completion grants execution permission.

Fresh source/root/namespace membership checks, parent proofs, exact single-link empty-stage identity/revision checks, paced copying, full byte comparison and no-replace publication remain in the central engine. Reopening completed/interrupted streams validates the retained evidence and inspects current native bytes; it does not require stages to remain empty after copying and does not replay writes.

## Validation and remaining gate

Extend the scope-free native catalog diagnostic with both complete preparation/copy and interruption immediately after publication. The complete case must retain thirteen preparation facts and thirteen copy events, reopen two verified outputs and preserve both prepared native IDs. The interrupted case must retain five copy events, inspect one verified publication plus one original empty stage, preserve its published ID and refuse replay/another stream. Generated sources and unrelated destination bytes stay unchanged.

Reject twelve altered plan/reference variants before any copy anchor is inserted, including stage name/identity, target, source revision/parent, destination root/parent, missing proof, reordered files, foreign UUID, anchor digest and completion fingerprint. Revoked managed grants block reopening. Run the full checks and native suites; no new process-exit matrix, physical power-loss or independent backup evidence is implied by the cooperative interruption diagnostic.

Production namespace preparation approval, ready-plan review/session approval/jobs and controls, multiple roots, full execution capacity checks, import/idempotency, undo and remaining native M2 gates remain open before M3. No schema/dependency change or real-library mutation endpoint is added.
