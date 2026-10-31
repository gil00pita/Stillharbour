# ADR-0034 — Execute only identified empty preparation stages

Date: 2026-10-31. Status: integrated constructor-owned preparation/copy fixture; production copying remains unavailable.

## Decision

Extend private copy files with optional prepared-stage identity/revision evidence. Omitted fields retain previous plan serialization/digests. If present, reopen that exact stage without creation/truncation and require its native identity, single link, empty length, recorded change-time revision and fresh membership before any write. Failure retains every object; never recreate a missing stage, adopt a replacement, clear changed data or fall back to another name.

Keep preparation facts and copy execution events in separate immutable journals. The copy intent precedes adoption/write, and its stage_created phase retains the actual stage identity for that stream; physical creation occurred earlier in the preparation journal. Empty objects and old intent are not execution approvals. Recovery with no copy-stage event reports prepared_empty_stage only if the initial proof still matches; missing, occupied, replaced, linked or changed objects require attention. An empty file is not silently presented as a completed/published copy. Once execution has stage identity evidence, existing fresh complete byte comparison and native revision/link checks determine partial/staged/published state.

The scope-free native preparation diagnostic now feeds its actual native-name-prepared stages into a distinct immutable copy plan/journal. It binds nested parent proofs, checks all three unstarted stages, streams generated source bytes under the shared budget, verifies/publishes without replacement and reopens outputs. Require the published native IDs to equal the three prepared IDs, with unchanged sources/destination neighbor. The ordinary preparation report still states executionAvailable=false; this is fixture pipeline evidence, not user approval or production ready-plan authority. The catalog adapter explicitly rejects prepared-stage evidence until its separate reviewed namespace/ready-plan binding is implemented.

## Validation and remaining gate

Regressions change a prepared stage to nonempty data, change and empty it while restoring mtime, replace its identity, remove it to a retained name and add a hard-link alias. Copy/inspection must reject without publishing or overwriting/recreating those entries; preserve generated source/neighbor bytes and retained objects. Native receipt copyFromPreparedStages records prepared_empty_stage before and published_verified after, with the same three object IDs. Original stage/copy/verification/recovery tests remain required.

This does not add a renderer path, approval nonce, real-library mutation, new dependency/schema or cleanup policy. Production catalog namespace ownership, ready plans/session approval/jobs, multi-root execution, full capacity/fidelity, import/idempotency/undo and remaining native gates still precede M2/M3 acceptance. Actual platform checks belong in the implementation log.
