# ADR-0070: Read-only inspection jobs and retained evidence

Date: 2026-11-01. Status: in progress. Domain/native jobs, catalog schema and UI are connected; complete milestone and native UI evidence remain pending.

M3 extends the existing Rust core, native source adapters, resource scheduler and catalog actor. It does not create a second catalog or a new filesystem executor. The renderer supplies registered root/job/member IDs and typed comparison policy. Native paths, source handles, object/revision checks and database writes remain backend-owned.

Pure inspection rules use a checked-reader port. The native adapter acquires shared root read scopes and revalidates catalog/root/volume and native file membership around reading. Each read participates in the resource scheduler and aggregate paced I/O budget. Metadata enumeration and opaque decoder limits are disclosed separately. Initial bounds are 64 roots, 100,000 members, a retained-name/metadata accounting bound, one million property/collision comparisons 4,096 reported groups, 50,000 folder nodes and 1,024 members in a backend keeper review. Hitting a bound yields explicit incomplete coverage, never a successful empty report.

The catalog integration uses one active inspection per catalog with no unbounded pending queue, cooperative cancellation and durable read-only job/findings history. Reopening reconciles interrupted metadata states without filesystem I/O or automatic replay. Input/root snapshots remain immutable. File absence, offline roots, access failures, changed objects, unsupported formats and partial enumeration have distinct observations.

Schema 9 adds inspection jobs/findings, versioned fingerprints and append-only integrity baselines inside the existing migration/snapshot transaction. Explicit baseline establishment records newly read complete BLAKE3 bytes, revision, object, time, policy and previous evidence. Comparing or acknowledging a mismatch cannot replace old baselines. A mismatch means changed-since-baseline, independently of decode findings. No baseline is independent recovery evidence or mutation approval.

Exact-content groups require direct complete-byte comparison after candidate filtering, including legacy-hash choices and collision tests. Property rules remain candidates. Normalized pixels require a versioned full-resolution supported decoder policy with precision/color/orientation exclusions; unsupported paths return capabilities rather than thumbnail equality. Decode work belongs in the existing bounded helper-process system, not the UI or async executor. RAW preview success cannot establish RAW deep health.

Keeper review retains manual keep-multiple and explicit preferred/protected folders. Backend guards protect the last accessible independent object and incomplete/unique companions. Any generated quarantine/restore demonstration must use the M2 adapter with fresh full-component comparison and separate approval; production quarantine still fails the independent-recovery gate.

Constructor-owned duplicate recovery retains a bounded immutable scope bound to the owner-held expected digest and native container identity. Cold reconstruction checks known member names/payloads, retained versions, current positions and immutable journal evidence; it restores no approval. Real child exits cover the first quarantine and restore moves, followed by fresh inspection/continuation. This does not create a production adoption or independent recovery provider.

Storage reports use the same retained scope for tables and treemap. Path logical bytes, unique-object logical bytes, reported allocation and physical reclaim remain separate. Unknown allocation/clone/snapshot effects stay unknown. A same-volume quarantine records zero freed bytes. All-eligible-file inspection observes unknown entries without enrolling them as photos or traversing links.

No large frontend/state framework or native binary is added. Required RustCrypto/Unicode source dependencies are pinned with lockfile checksums and recorded licenses. Actual checks, failures, supported decoder policy and target evidence must be recorded before this decision or M3 can be called complete.
