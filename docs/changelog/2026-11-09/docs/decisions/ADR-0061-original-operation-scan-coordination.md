> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0061: Original-operation scan coordination

Date: 2026-11-01
Status: implemented and locally validated; target CI pending; M2 remains open

## Decision

Retain one constructor-owned native source-write/retained-version-read lease across generated original movement, final byte verification and the atomic catalog/path/receipt/job transaction. Do not release and reacquire the source scope between movement and catalog commit. The private held scope is bound to its fixture; it cannot authorize caller paths or another fixture. Direct native fault tests acquire the same scope.

Retain the prior fresh directory-name/native-object binding check before catalog commit, independently of the held lease. Windows held directory handles refuse delete sharing; Unix open handles alone cannot prevent directory renaming. Replaced names must refuse completion and preserve the new occupant.

While a root has queued/running original work or an unresolved needs-recovery attempt, refuse scan admission and library removal with a typed Busy error. Check durable state while holding the scan's native read lease, before creating its scan job. A verified completed attempt resolves earlier needs-recovery attempts for that exact plan only; it does not resolve an interrupted reverse or another plan. A previewed plan without any execution attempt does not block scans. Malformed/unavailable catalog state remains an error.

The monitor retains pending invalidation on Busy, exposes its reason as deferred, advances fairly to other roots and retries after reconciliation. Never interpret staging names as a new settled library namespace, delete stopped history or automatically resume work. Metadata-only startup recovery remains unchanged. This adds coordination, not production original execution, restart-scope adoption, backup integration or independent recovery evidence.

## Validation

Exercise actual generated originals, retained native scopes, the real Library scan/removal admission and catalog commits. Require overlapping scan/backup-style readers to fail before native completion and through commit, stopped attempts to retain old paths without creating a scan job, and fresh approved completion to release the block. Rescan the settled namespace and verify stable asset IDs and no staged duplicate rows. Check reverse-plan isolation, startup recovery and unrelated roots. Record actual results and limitations in the implementation log before claiming this partial AC-081 coverage.

The owned isolated integration case also starts the real monitor and requires both a visible recovery deferral and an automatic unrelated-root scan. Windows refuses moving the held source directory; the Unix branch swaps in a generated foreign directory, requires commit refusal/no verified terminal job, restores the original binding and verifies the foreign keeper survives undo. Runtime/CI results remain distinct from focused local evidence. The shared test-only launcher isolates the CI mixed-import liveness case without changing its ten-second inner wait; it refuses zero-test child success.
