# Stillharbour - M4 - Safe one-way sync, versioned backups and verified restore

## Objective

M4A read-only implementation is complete for exact-head Windows/macOS acceptance validation. See [M4A_VALIDATION.md](../docs/M4A_VALIDATION.md) for the requirement matrix and [IMPLEMENTATION_LOG.md](../docs/IMPLEMENTATION_LOG.md) for actual results. Complete the checks and inspect native receipts before marking the gate accepted. M4B–D and real-library mutations remain separately gated.

Policy 3/schema 12 add immutable bounded observation history, destination-edited context, an immutable nonexecutable review descriptor, explicit superseding profile rebind, potential companion relationships and ID-bound side-by-side decoder previews. Original and destination trees remain unchanged.

Provide genuine snapshot backup/restore plus a distinct readable Safe copy / one-way sync workflow. Select the base library automatically; the user selects the destination. Do not disguise destructive mirroring as backup.

## Prerequisites and references

Current continuation: policy-2 conservative name/type conflict blocking, full-report category totals/filter paging and observed entry details. Validate generated case/type conflict byte preservation, Unicode/ancestor policy fixtures, matching rows beyond page one and actual native filter/inspector UI. The read-only gaps are closed by ADR-0075; require the exact-head native platform/failure evidence in M4A_VALIDATION.md. Prospective write-name preparation and transfer/fidelity/recovery gates remain M4B–D.

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
