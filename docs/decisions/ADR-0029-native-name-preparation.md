# ADR-0029 — Native future-name preparation in owned pending outputs

Date: 2026-10-31. Status: disposable-fixture development; production copying remains unavailable.

## Decision

Test proposed names through exclusive native creation in a new constructor-owned pending output namespace. Do not infer future-name equivalence from lowercase strings, Unicode normalization, an existing-target lookup or a volume label. This extends ADR-0028's fixture capability only; the entry point accepts no paths or renderer-supplied plan.

The private preparation accepts at most 15,600 unique digest-identified portable relative targets and 1,024 held directory handles. Input and retained journal records are bounded to 8 MiB. Protected photo-library bundles and invalid paths are rejected before creation. The fixture journal records intent before each directory, placeholder and stage operation. Unknown objects, unsupported entries and unreadable lookups fail closed; only an explicit native absence means absent.

Create exact proposed directory/leaf names using the existing exclusive native adapters. Recognize collisions only against recorded objects from this preparation. Mark every participant in file/file, file/directory and aliased-directory conflicts, including earlier rows. No suffix assignment, overwrite or normalization resolves a conflict. On Windows the adapter uses its conservative case-insensitive object lookup; this may reject distinct names on a case-sensitive directory and does not claim support for every filesystem's naming modes.

Move known zero-byte placeholders to unique retained stage names within their actual parent using native no-replace publication. Reopen with the expected object identity, link count and change-time revision; reject nonempty or changed probes. Before retaining the preparation fingerprint, check directory membership from held parents, every stage's identity/revision/empty length/single link and absence of each original target name. This is sequential evidence, not a reservation against arbitrary external writers or an atomic namespace snapshot. A later ready-plan executor must revalidate it.

The fingerprint binds the policy, parent identity, generated pending namespace, target rows, directories and stage evidence. Reports always state executionAvailable=false. Sources are neither opened for content nor changed by name preparation. No failed stage or pending namespace is deleted or adopted. Generated fixture cleanup remains owned by its temporary run.

## Validation and next gate

Tests cover actual case/Unicode creation, both file/directory conflict orders, invalid/protected paths, prior cancellation, oversized scope, actual SQLite write denial, changed stages and occupied target names. Four injected interruptions occur between native creation/movement and its completion record; no namespace-prepared completion is written and the pending scope remains. These are cooperative failure injections, not additional process-exit or physical power-loss evidence.

The test-only native action organization_name_preparation_fixture creates its own scope and emits owned-name-preparation-OS.json. Production excludes it. Ordinary checks and Windows/macOS native jobs remain required. Production integration still requires complete namespace sealing against unknown entries, capacity/fidelity evidence, manifest/ready-plan approval, catalog/job/lock adapters and safe final namespace publication. Import, undo and the M2/M3 gates remain open. No persisted identity, schema, dependency version or real-library authority changes.

**Namespace inventory refinement:** before retaining preparation, enumerate each held owned directory and require exactly the recorded unique child objects/kinds. Reject unexpected files/directories, unsupported or repeated objects and incomplete enumeration without adopting or deleting anything. Fresh stage revision/link/length and target-absence checks follow the inventory. This is bounded by the known scope plus the first unexpected entry and remains sequential observation; final publication must revalidate against concurrent changes. All seven preparation regressions and fixture-enabled denied-warning Clippy pass.
