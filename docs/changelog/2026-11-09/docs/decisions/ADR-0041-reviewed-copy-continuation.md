> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0041 — Reviewed continuation of retained copy streams

Date: 2026-10-31. Status: constructor-owned fixture capability only; local Windows full/native validation passes, target-platform continuation CI pending.

## Decision

Keep ordinary execution's zero-history admission rule. Add a separate one-use ResumeFiles fixture approval bound to the immutable plan, private native selection, hash of the exact bounded ordered journal and current native stage/target observations. Rebuild and validate journal phases before review and admission. Reject stale observations/history, wrong-action proofs, foreign/missing objects, multiple occupants, unsupported links and terminal incomplete streams. Persisted facts never reconstruct approval.

Continue the same stream and original native objects. Complete only missing phases with actual evidence: create an absent stage exclusively after intent; reuse an unchanged identified prepared empty stage; compare every retained prefix byte before appending; flush/close/reopen and fully compare staged bytes; record publication intent; publish with native no-replace; fully compare published bytes and commit completion. A publication surviving a process exit is recognized by its recorded original object and fresh full comparison. Never allocate a replacement stream, truncate a retained stage, adopt an unrecorded existing stage or repeat completed events. A completed stream requires fresh complete byte comparisons and remains a zero-write no-op.

Unix retained writers use no-follow O_APPEND; Windows retains the reviewed native writable handle with write/delete sharing denied. Both recheck single-link native identity, source revision, stage membership/length and reviewed observations. Shared paced I/O charges prefix comparison, remaining writes and all verification. Fresh source/destination scope and managed-root grants are checked at admission and scope boundaries, including before publication. Failure/cancel retains source, published outputs and partial stages; no source mutation or cleanup is added.

The new module and ResumeFiles grant are excluded from normal/copy-workflow-only builds. The existing scope-free native fixture diagnostic exercises this engine; its grants are marked executionApproval false. This does not expose an application Resume button or authorize a real library retry.

## Validation and next gate

Generated regressions cover all nine initial interruption boundaries, all nine continuation interruption boundaries, all nine actual continuation child-process exits followed by reopen/fresh approval, completed zero-event retries, corrupt prefixes beyond 64 KiB, stale observations and wrong-action approvals. Separate diagnostics continue a real 64-KiB partial stage through publication interruption, then finish two outputs/thirteen events with the same original stage object. Prepared-namespace continuation preserves both prepared object IDs. Record actual full/native results in IMPLEMENTATION_LOG.md.

Application recovery review/approval/job integration, complete retained namespace inventory for resumed work, cancellable inspection jobs, idempotency across new drafts, import, multi-root/cross-volume/case-cycle/undo and remaining M2 acceptance precede M3. Original mutation and default copy gates remain unchanged. No physical power-loss or macOS result is inferred from local Windows tests.
