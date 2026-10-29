# P3 — shared read-only resource admission

The owner's 2026-10-29 “Start new stage” continues the selected performance track after P2 and live-library reconciliation. Implement the available read-only part of P3, report measured results, and stop before preservation operations or P4 stress tuning. Existing uncommitted work is retained.

Use [PERFORMANCE_PLAN.md](../docs/PERFORMANCE_PLAN.md), [ADR-0014](../docs/decisions/ADR-0014-shared-source-admission.md), the preservation contract and P3 acceptance scenarios. Inspect actual backup capabilities: there is no production transfer/backup adapter. Explicit synthetic read contenders may test the scheduling interface; they cannot pass native backup, NAS/HDD, transfer-pacing or recovery acceptance.

Implement shared atomic CPU/memory/source-volume admission for metadata and rendition work. Prioritize visible work, age background requests, bound queues, check cancellation before admission and release permits after failure. Keep job admission/maintenance barriers and all identity/revision/publication checks. Adapt gallery requests to scrolling using existing viewport signals, without changing layout or selection behavior. Do not introduce an unenforceable hard I/O control for opaque helpers.

Run independent scheduler/failure tests, frontend regressions, generated-source core/native checks and an executable release contention fixture with independent source manifests. Record commands, build identity, percentiles/sample counts, failures, unrun tests and coverage limits in [PERFORMANCE_P3.md](../docs/PERFORMANCE_P3.md). No dependency/schema migration, new decoder, original mutation or real-library development test.

Full P3 acceptance remains gated on real backup/transfer integration, reviewed endpoint/physical-device mapping, accounted chunk pacing and native NAS/HDD/platform evidence. Do not advance to P4 or call these unavailable scenarios passed.
