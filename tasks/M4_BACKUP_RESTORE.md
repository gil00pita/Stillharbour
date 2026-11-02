# Stillharbour - M4 - Safe one-way sync, versioned backups and verified restore

## Objective

Owner continuation, 2026-11-02: finish all M4 slices, commit/push and create a PR; use disposable local protocol servers. ADR-0077 adds generated shared-executor append/verification/recovery with separate staging. This supersedes the earlier authorization-only pause before M4C/D, while preserving every acceptance gate and the stop before M5. Full M4 completion must be based on actual implementation/native/transport results, not the generated lab alone.

M4A read-only implementation and Windows/macOS native validation are complete at tested head 90acf91 (CI 41356788575). See [M4A_VALIDATION.md](../docs/M4A_VALIDATION.md) for the requirement matrix and [IMPLEMENTATION_LOG.md](../docs/IMPLEMENTATION_LOG.md) for actual results. Both platform receipts and screenshots have been inspected. The owner authorized starting M4B on 2026-11-01; its first increment is backend-only append planning and native revalidation (ADR-0076), with execution still unavailable. M4C–D and real-library mutation gates remain separate.

M4B/C application integration now connects native complete-set preparation, separate approvals, durable staging/read-back/publication, transfer controls, retained history and cold recovery. Twenty focused Windows application tests pass, including actual child exits and preparation gaps; eight native metadata cases pass separately. M4D has passed eight actual Windows snapshot cases and five actual application-adapter transport round trips. The opt-in generated C:/E: fixture also passes its separate-device assertion and recovery flow. These results precede the final integrated rerun and do not certify strict replacement evidence or complete the 40-scenario matrix. ADR-0079 records schema 13, native credentials, packaged helpers, snapshot history/restore and exact-manifest schedules. Record results at each acceptance gate; the owner has authorized continuing through M4C/D.

Policy 3/schema 12 add immutable bounded observation history, destination-edited context, an immutable nonexecutable review descriptor, explicit superseding profile rebind, potential companion relationships and ID-bound side-by-side decoder previews. Original and destination trees remain unchanged.

Provide genuine snapshot backup/restore plus a distinct readable Safe copy / one-way sync workflow. Select the base library automatically; the user selects the destination. Do not disguise destructive mirroring as backup.

## Prerequisites and references

Current continuation: policy-2 conservative name/type conflict blocking, full-report category totals/filter paging and observed entry details. Validate generated case/type conflict byte preservation, Unicode/ancestor policy fixtures, matching rows beyond page one and actual native filter/inspector UI. The read-only gaps are closed by ADR-0075; the exact-head native platform/failure evidence is recorded in M4A_VALIDATION.md. Prospective write-name preparation and transfer/fidelity/recovery gates remain M4B–D.

M1 catalog and M2 coordination/execution contracts; docs/CODEX_SPEC.md section 13; acceptance AC-066 through AC-073, AC-081 and AC-084.

## Work

Wrap pinned bundled restic and rclone with structured arguments/output, fixed executable paths, cancellation, credential references and redacted logs. Implement profile setup, strict SFTP host trust, FTPS certificate validation and explicit legacy FTP warning. Validate local/mounted destinations and direct SMB/SFTP/FTPS/FTP remotes in isolated test areas.

Implement snapshot scope preview, source-version manifest, consistent catalog snapshot, overlapping-root mutation coordination, partial/offline results, and correct interpretation of helper exits. Preserve historical snapshots when sources disappear. Record snapshot IDs and per-file-version backup evidence.

Build backup history, snapshot browser, restore preview to a new directory, safe path/collision handling, and content verification. Separate uploaded/included, structural check, data-read verification and restore-comparison states. Implement retention preview but no automatic pruning unless the owner approves and tests it separately.

Provide running/tray schedules with clear fully-quit limitations and catch-up behavior. Do not force-unlock/repair a repository to make an error disappear. Document the repository-key recovery dependency and offer a safe recovery-key export flow.

## Protocol release gate

For every advertised destination, run backup -> remove disposable source -> restore -> full-byte comparison using actual protocol endpoints. Record server implementation/version, OS, transport settings, result and failure-injection evidence. Unsupported/unavailable protocols remain explicitly unverified and cannot be advertised as supported.

A completed upload or connection test is not sufficient. Stop before M5 after recording the real results and blockers.

## Version 1.1 staged extension

Read docs/PRESERVATION_CONTRACT.md, docs/BACKUP_SYNC_ADDENDUM.md and docs/PRESERVATION_TESTS.md before implementation. Original restic/transport/restore requirements above remain mandatory.

- M4A: fixed-source read-only compare, contextual issues, plan/capability types and review UI. Stop for approval.
- M4B: verified source-retaining copy, exclusive staging, pacing, durable recovery and destination-only retention. Stop for approval.
- M4C: retained-version destination replacement only for tested capable adapters; exact-version recovery-evidence gate. Stop for approval.
- M4D: integrate snapshot backup/restore and protection reporting; pass real OS/transport/failure tests before enabling affected production features.

Throttle comparison and verification as well as copying in the owned I/O engine. Disclose helper limits. A universal override action, unchecked source cleanup or direct mirror command is prohibited. All 40 PS scenarios are required in addition to original M4 tests. Record unrun checks as unrun, not passed.
