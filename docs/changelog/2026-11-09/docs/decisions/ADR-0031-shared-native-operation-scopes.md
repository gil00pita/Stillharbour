> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0031 — Shared native directory scopes for scan and copy admission

Date: 2026-10-31. Status: scan and disposable executor integration; production mutation remains unavailable.

## Decision

Use one process-wide bounded registry to admit compatible native directory scopes atomically. A scope owns an independently opened DirectoryChain and read/write intent. Keys consist of each held ancestor's native volume/file identity. Read scopes coexist. Any write conflicts with an equal, ancestor or descendant scope; sibling directories sharing only more distant ancestors remain compatible. No path lowercase/prefix comparison, catalog UUID alone or renderer string supplies lock identity.

Validate at most 64 scopes per request, each with at most 96 unique ancestor identities. This accommodates the existing 32 source roots and separate destination. Keep at most 64 active leases, checked and inserted under one short registry mutex. Acquire all scopes or none; do not wait holding a subset or perform filesystem I/O under that mutex. Incompatible/full admission returns typed Busy. No unbounded waiter queue or lock-order deadlock. RAII leases retain the directory handles and release their complete registry entry on normal/error exit or failed worker spawn. A poisoned registry refuses new admission until restart; dropping an existing known lease may safely remove its entry.

This is application job coordination, not a cross-process/OS lock, path authorization, atomic namespace snapshot, independent backup or permission to mutate originals. Callers must still revalidate native membership, source revisions, volume/root authorization, immutable plans and approvals. An offline/replaced root never becomes a successful empty scope. Leases are not serializable and cannot be imported or restored as execution approvals.

## Integration

Before creating a scan catalog job, open the resolved root chain, bind its leaf identity/volume to the selected registered root and acquire a shared read lease. Keep it through the scan worker and initial-preview enqueue handoff; do not mark an incompatible request as started. Existing scan slot/cancellation/maintenance/resource accounting remains intact. Failed admission leaves the job list and originals unchanged.

ADR-0028's constructor-owned copy engine atomically acquires read source/write output scopes before journaling. Its recovery inspection takes read source/read output scopes; its existing per-operation mutex still excludes execution/recovery overlap on that operation. The native preparation diagnostic holds read source/write output scopes while inspecting/creating its disposable output tree. Native fixture receipts additionally prove incompatible copying and recovery are blocked with zero journal events, release permits completion and a shared source reader can coexist.

Other readers such as background rendition/monitor operations are not yet integrated into this registry; the lease does not cover asynchronous rendition workers after enqueue. There is no production copy/backup writer yet. Those adapters and any future in-place writer must participate before their gates pass. No AC-081 full concurrency acceptance is claimed merely from this registry.

## Validation and next gate

Generated native-directory tests cover ancestor/descendant conflicts, shared readers, disjoint siblings, no partial hold on a multi-scope failure, whole-lease release, capacity and simultaneous opposite-order requests admitting exactly one lease. A real Library scan fixture proves a held incompatible scope returns Busy before catalog job creation and leaves the scan slot/source intact. Copy/recovery tests prove zero journal changes on rejection, release and shared-source-reader completion with byte verification and preserved neighbors. Existing failure/recovery and native suites must still pass. Actual results belong in the implementation log.

No schema/dependency/IPC change or additional native binary. Complete production manifest/ready-plan/approval/catalog-job integration, capacity/fidelity checks, import/idempotency, undo and native failure/recovery gates remain necessary before M2 completion and M3.
