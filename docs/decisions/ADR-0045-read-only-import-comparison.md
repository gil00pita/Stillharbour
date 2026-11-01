# ADR-0045 — Bounded read-only byte comparison before import

Date: 2026-10-31. Status: experimental M2 import prerequisite; full checks and Windows/macOS native CI pass.

## Decision

Compare a retained confirmed source manifest with explicitly selected registered catalog roots. Source and candidate access is read-only; referenced roots do not gain managed-copy or original-mutation permission. Requests carry validated plan/manifest/root/job IDs, never filesystem paths, SQL or hash overrides. No report authorizes copy skipping, quarantine or removal. Bundle-aware import planning and fresh keeper revalidation remain subsequent work.

Use size, three bounded 64-KiB sample windows with a versioned BLAKE3 key, then complete BLAKE3 hashing to filter candidates. Confirm every reported match by direct complete byte comparison and EOF/length checks. Bind native root/file objects, volumes, non-lossy parent locators and revisions around reads, and revalidate matched members before returning. Distinguish shared native file objects from independent copies. Hash collisions cannot produce an equality witness. No fingerprint cache is reused across jobs.

A single background worker shares the organization workflow admission gate, native read leases and precharged aggregate byte budget across sampling, full hashes and direct comparison. Pause/rate controls apply to these payload reads. Metadata, SQLite, fixture constructors and opaque helpers retain their disclosed limits. Cancel on user request or maintenance of any source/comparison root; release native scopes, pause ownership and admission before publishing a terminal state. Check maintenance admission before and after reserving a queued job. Retain at most eight ephemeral reports, 512 source files, sixteen scope roots, 4,096 indexed candidate files and four directly verified matches per source file. Mark additional hash candidates as unexamined. Reports disappear on restart and grant no persisted authority.

Coverage is explicitly indexed present files in the selected roots. Report missing/excluded counts separately from inaccessible or unresolved inventory. Offline roots are errors, never empty libraries. Stale candidates, incomplete scans and unreadable entries prevent absence claims; positive byte observations remain distinct. Native/source changes are structured failures. File mismatch does not mean proven damage or provider-original fidelity.

Add a paged read-only review to compiled manifests. Validate response scope/membership/policy before rendering evidence, render filenames as text, expose cancellation/byte-I/O controls and discard valid late jobs after unmount. Actual import selection/skip execution is not wired to this report.

## Validation and limits

Focused native-file tests pass naturally equal samples with different unsampled bytes, forced full-hash collisions rejected by direct comparison, renamed files, empty files, hard-link identity, cancellation and changed versions. Actual application tests pass referenced-source read access without managed-copy authority, renamed exact bytes without journal/source/candidate mutation, offline/stale/inaccessible scope handling, forced candidate-hash collisions, and root cancellation. Eight focused component cases across comparison/manifest review pass explicit scope, text-only hostile names, incomplete coverage, foreign response rejection, cancellation and late-response disposal. Bounds/native UI/full checks follow.

Native fixture setup constructs renamed copies only from its owned generated baseline, registers/scans that disposable referenced root, then independently audits preservation. macOS removes only the catalog registration; Windows retains the disposable registered root because its cache-cleanup native gate remains disabled. Those constructor/audit helper reads are test instrumentation, not production throttling evidence. Default feature and original-mutation gates remain unchanged. This checkpoint does not complete folder import, duplicate skipping, M2 or M3.

Complete checks pass 272 frontend and 187 applicable Windows Rust cases, strict/generated IPC, lint/format, denied-warning Clippy and build. A final Rust rerun covers the additional native identity/volume guards. Separate application-only and fixture-only graphs pass. Actual Windows UI passes four applicable cases with sixteen macOS skips, including renamed matches, honest partial coverage and unchanged independent source/candidate/output audits. The initial test-only teardown encounters the existing disabled Windows cleanup gate; retain the generated registered root instead of bypassing that gate. Production assets and release exclusions pass. The expanded macOS scenario awaits this checkpoint’s own CI.

Both target-platform CI jobs pass in [run 41251665854](https://github.com/gil00pita/Stillharbour/actions/runs/41251665854). This validates this checkpoint only; remaining milestone gates are unchanged.
