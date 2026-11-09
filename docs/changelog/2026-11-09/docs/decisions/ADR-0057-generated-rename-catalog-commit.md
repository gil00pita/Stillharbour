> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0057: Atomic generated rename catalog commits

Date: 2026-11-01
Status: implemented with full local and Windows native validation; checkpoint CI pending; M2 remains open

## Decision

Extend the generated rename session with a separate actual Catalog actor and current schema-6 library tables, created inside the constructor-owned run. Register only that run's generated root, discover its five members and retain stable asset IDs plus explicit catalog-only annotations. No existing catalog, root or path is accepted. Close the actor before the owned directory drops. This changes no production catalog schema or persisted identity.

Native movement and catalog completion are separate facts. After complete native journaling, reacquire the source write/version read scopes and freshly compare exact original objects, bytes, essential metadata and movement positions. Only then update catalog paths and retain a bounded commit receipt in one transaction. Validate managed/online/native root identity and compare exact old asset IDs, paths, objects, revisions and annotations. Stage unique database path keys inside that transaction to resolve case/cycle uniqueness, then install all final paths/revisions while retaining IDs, annotations and descriptive metadata. These database keys are never filesystem paths used for execution. Existing target rows cause a uniqueness failure and full rollback.

The private fixture catalog adds a unique receipt stream index and update/delete refusal triggers for its generated-rename job-event receipts only. Receipts bind exact plan/root/asset/object/path/revision mappings and never contain approvals. Inspection validates receipt and catalog state against fresh native facts. A native-complete operation without its catalog receipt reports needs_catalog_commit, never completed. Fresh acknowledgment and approval can reverify and commit catalog state without repeating completed moves. Undo is unavailable until both forward phases complete, then uses a new reverse plan and commit.

The instrumented UI shows observed native names beside catalog paths, distinguishes the pending catalog state and offers a specifically named commit action. Native receipts verify stable asset IDs and matching catalog/native final paths in both directions. This advances actual generated-scope UI/catalog integration; general application root organization, durable application jobs, restart adoption, multi-root policies and the other M2 gates remain open. Real-library mutations and duplicate quarantine remain disabled pending independent recovery/native gates.

## Validation

Exercise case/cycle forward and reverse commits, stable IDs and unchanged favorites/ratings/tags. Inject actual SQLite trigger write denials after database staging and before receipt insertion: all catalog paths roll back, native objects remain in their verified final names, undo stays unavailable and an old grant cannot retry. Fresh approval commits once with no extra native journal facts. Protect retained receipt updates/deletes. Component tests distinguish completed native movement from a pending catalog commit; native UI tests inspect actual IDs and paths. Record actual checks and limitations in the implementation log.
