> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0056: Explicit generated rename and undo review lifecycle

Date: 2026-11-01
Status: implemented with full local and Windows native validation; checkpoint CI pending; M2 remains open

## Decision

Expose the existing closed generated-file rename engine through a typed, instrumented-desktop-only lifecycle. The backend constructs one temporary fixture per desktop process. Renderer requests contain only enumerated actions, a retained plan UUID, an acknowledgment and an opaque session ticket. Neither paths nor arbitrary fixture adoption are accepted. Reject replacing an existing session; inspection remains available after a UI close or worker failure. Run native work on a blocking worker and refuse concurrent session requests.

A preview binds actual native positions, complete bytes/essential metadata against retained local versions, root identity, plan digest and exact journal cursor. Approval requires explicit acknowledgment and an unfinished matching plan; start consumes the one-use grant against fresh observations before the central engine's own native checks. A lost completion changes the review digest; continuation needs fresh inspection/approval. Undo uses a new immutable reverse plan and separate acknowledgment/approval. No approval survives as journal evidence. Old-path occupants remain protected by the existing reverse planner.

A visibly labeled generated-fixture dialog is loaded only in desktop-test builds, via a test-scoped event. Actual buttons create, inspect, acknowledge, approve, run and review undo; errors remain errors and late responses after close cannot restore a grant. The fixture survives dialog close for inspection. Existing roots and catalog records are outside this capability. Normal production handlers/capabilities/frontend exclude the route; release checks enforce those exclusions.

This advances actual UI/IPC review over native rename infrastructure. It does **not** implement application catalog path updates, durable application jobs, restart adoption, general source-root organization, independent recovery or real-library mutation authority. Its private SQLite rename journal remains the existing closed fixture journal. These remaining integrations and native acceptance gates still precede M2 acceptance and M3.

## Validation

Core regressions cover missing/incorrect acknowledgment, unknown tickets, separate reverse approval, replay refusal and fresh approval after an interrupted native move. Component tests cover exact typed actions, separate undo consent, late-grant disposal and failed-state inspection. Native desktop tests click the actual controls, inspect real IPC journal positions, verify all five observed final names and restoration, and capture screenshots and bounded receipts without approval tickets. Full/feature-graph/native results belong in the implementation log.
