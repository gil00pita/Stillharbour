# ADR-0059: Separate durable organization plans and jobs

Date: 2026-11-01
Status: implemented with full local/Windows native validation; target CI pending; M2 remains open

## Decision

Advance the catalog to schema 7 with separate original_operation_plans, original_operation_jobs and original_operation_events. Existing copy jobs keep their copy-plan foreign keys and meanings. Retain a consistent independently openable snapshot before migrating an existing catalog, preserve catalog/root IDs and earlier history, and roll back all new objects if migration fails. Historical root UUIDs remain in immutable plan records even if a root is later forgotten; they do not restore root authorization.

The new plan writer accepts canonical resolved graph data only from the closed generated workflow, validates its current managed/online native root binding and retains catalog/root/graph/parent/digest identity. Scope is explicitly generated_fixture. Plans and job events cannot be rewritten/deleted; job identity is immutable, transitions are constrained, and only one queued/running attempt per plan is admitted. Bounded typed history is data, never filesystem authority or approval.

After an exact one-use approval is consumed, persist queued and running before native movement. A verified terminal job transition, its event, final catalog paths and verification receipt commit in one transaction after fresh native object/byte/metadata checks. Generic job updates cannot claim verified completion. The committing job must reference the exact native plan and move count. Any failure leaves originals/stages retained and records needs_recovery where possible; a failed outcome write remains an explicit error. Startup/cold recovery marks interrupted queued/running workers as needing review, changing catalog metadata only. It never resumes native I/O or restores a ticket. Retry creates a separate attempt; earlier failures remain retained. Approval fingerprints include observed job history as well as native/catalog facts.

Typed LibraryStatus history, the Jobs dropdown and the generated dialog show actual states and verified move counts. Queued/running states are indeterminate; only completed_verified fills the bar. Recovery history has no replay action. Actual execution remains limited to constructor-owned generated roots; default real-library original mutation and quarantine gates are unchanged. This slice retains/observes the synchronous fixture worker lifecycle; general library orchestration, live original-worker queue/control, watcher reconciliation, restart discovery and remaining M2 gates stay open before M3.

## Validation

Verify v6-to-v7 success, schema-object collision rollback, preserved catalog/root/native-path/history fields, read-only independent retained snapshots and migration child exits through version 6. Test immutable plans/events, constrained job transitions, wrong-plan completion refusal, interrupted metadata reconciliation and zero unintended native moves. Extend SQL-denial tests through completed-event insertion and the twelve real process exits: paths, receipt and terminal job must commit together. Native UI verifies real rename/undo job records and stable catalog paths/IDs. Record actual full/feature/native outcomes in the implementation log.
