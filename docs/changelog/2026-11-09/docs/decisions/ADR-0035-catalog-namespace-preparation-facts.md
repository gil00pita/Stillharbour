> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0035 — Catalog-bound namespace preparation facts

Date: 2026-10-31. Status: constructor-owned fixtures; no production preparation or copy approval.

## Decision

Separate the central event append interface from copy-history inspection. Native preparation can append facts through the actual catalog actor without inheriting copy execution/replay authority. Freeze the backend namespace UUID before the first write and bind its exact generated pending name, retained draft/manifest digests, source/destination native scope and manifest file IDs/target paths in a distinct immutable anchor. Existing copy binding validation still requires the unchanged managed source grant and exact native mappings.

Store preparation anchors/facts separately from copy anchors/events in the existing append-only operation_journal. No schema or dependency change. Validate catalog scope on binding and each append. Atomically compare the anchor and event count in the same transaction as insertion; advance cached state only after commit. Detect duplicate anchors, competing writers, unknown phases, reordered/foreign hash-chain links and terminal replay. Hash chaining detects corruption; it is not authentication or imported approval. Retained facts do not claim that interrupted native changes completed.

Bound the complete anchor to 64 KiB, each fact to 8 MiB, aggregate encoded facts to 32 MiB and count to six facts per proposed file plus two per allowed directory and three namespace facts. Reject before the next native change when a limit/journal write fails. Existing generic preparation limits remain; the catalog anchor may require a smaller selection. Read retained payload length in SQL before allocating its text. Defensively decode completion evidence and recompute its original typed fingerprint, checking exact report/manifest rows, directory/stage counts, unique native objects, stage-parent/path mappings and single-volume evidence. This is internal consistency; future execution still requires fresh native inspection.

The scope-free catalog diagnostic prepares a generated nested JPEG/companion, retains thirteen facts, reopens them, refuses replay and checks unchanged sources/destination neighbor. It does not copy these stages through the catalog adapter. That adapter continues to reject unbound prepared-stage plans. Production ready plans must separately bind this evidence, validate all live objects and obtain explicit session approval before execution. Existing stage presence, completed preparation, persisted records and imported history grant no approval.

## Validation and remaining gate

Exercise actual preparation/reopen/replay refusal; deny the first fact through a SQLite trigger and revoke the managed grant, requiring zero new namespace objects. Reject journaling immediately after native root creation, retain that exact object, reopen only the intent and reject stale/reopened writers without replay. Inject a foreign history link and refuse reopening. The native foundation receipt adds namespacePreparation under catalogJournal. Run full checks and native suites before recording a platform pass.

Ready-plan/session approval/jobs, fresh prepared-stage catalog execution binding, multiple roots, execution capacity checks, import/idempotency, undo and remaining M2 native gates remain open before M3. This supplies no independent backup, power-loss or real-library mutation acceptance.
