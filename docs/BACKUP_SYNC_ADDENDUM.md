# Stillharbour - Backup & Sync - default-library workflow

Version 1.1 | 2026-10-23 | Proposed feature specification, not implemented software

Read PRESERVATION_CONTRACT.md first. This extends docs/CODEX_SPEC.md section 13 and milestone M4; it does not replace encrypted snapshots with destructive synchronization.

## 1. User outcome and scope

Use the configured base library automatically. The user chooses an output/backup folder, compares the contents, reviews proposed changes and starts a controlled transfer. Provide a modern review workspace, contextual conflict actions and adjustable transfer load instead of a generic retry-or-ignore loop.

Take interaction inspiration from FreeFileSync, not its source code, branding or exact interface. Its documented comparison, synchronization and versioning features inform the separation of compare, policy and execution [SB01-SB03]. Do not claim this application already equals its feature coverage or performance.

Initial readable-sync targets: local/external folders and mounted SMB shares, followed by direct SMB/SFTP adapters only after their safety tests pass. Existing restic backup targets remain independently gated. A successful snapshot backup test does not prove safe ordinary-file replacement through the same transport.

## 2. Source and destination

The source card displays the base library name, path, root UUID, availability and scope. There is no routine source-folder picker. Bind a saved profile to the resolved root UUID, not a dynamic 'whatever is default now' expression. Changing the base library requires an explicit profile-rebind review and a new comparison; it must not reinterpret old backup contents as disposable files.

Extra library roots are optional and explicit. Each gets its own root-identity namespace at the destination. Do not merge identically named relative paths from separate roots.

The destination picker supports a local/external directory or mounted network folder. A separate remote-connection flow serves direct SMB/SFTP and snapshot transports. Show destination identity, reachability, available/quota space where known, and whether physical independence is verified, owner-declared or unknown.

Use a dedicated application-owned dataset directory beneath the chosen destination. Preview its exact path. Selecting a nonempty folder never grants ownership over all of its content. Offer to compare or adopt a named existing dataset only after inventory and explicit approval. Keep readable-copy data, version history, staging and a restic repository in separate namespaces. Never run file synchronization into a restic repository's internal folders.

Reject source-equals-destination, ancestor/descendant overlap and known aliases to the same tree. Verify drive/share/repository identity on every run and reconnect. Do not assume a reused drive letter or identical share name is the original storage. Never follow links outside authorized roots.

Full-library protection inventories eligible regular files independently of the gallery index. Include hidden files, unsupported formats and non-photo companions unless explicitly excluded in this profile. Keep exclusions, inaccessible paths and special-file limitations visible. Source selection alone never authorizes source mutation.

## 3. Two clearly distinguished modes

| Mode | Storage | Default handling |
| --- | --- | --- |
| Safe copy / one-way sync | Readable file tree plus retained version history | Add source-only files; retain destination-only files; review differences; retain old destination versions before approved replacement |
| Versioned backup | Encrypted restic snapshots | Preserve history; protect catalog and companions; use explicit snapshot browser and restore |

Safe copy is the default for the requested folder-to-folder workflow. Versioned backup remains the recommended historical recovery mechanism, with an independently protected destination. Same-disk version history is not disaster protection. Explain that the encrypted repository needs its recovery secret; a readable copy does not have that format dependency but does not gain restic encryption automatically.

No two-way writes into the library, automatic mirror deletion, source deletion, pruning, or quarantine expiry in this extension. Keep any future mirror/two-way mode behind a separately approved specification and safety gate. 'Sync' does not silently mean 'make destination exactly match source at any cost'.

## 4. Compare is read-only

Compare may write local catalog/job state but does not change either file tree. A connection write-test is a separately disclosed action restricted to job-owned temporary objects.

Offer Quick compare (path/size/time candidate information) and Verified compare (full content). Quick results say 'appears unchanged' or 'content not checked', never 'byte-verified'. The verified mode is the default for protection decisions. A full-content digest may index candidates; exact equality for ordinary files is confirmed by streaming byte comparison before issuing equality evidence. Every read goes through resource limits where the adapter supports them.

Timestamps and equal lengths cannot prove equality. Do not auto-select a winner because a file is newer; preserve creation/modification/capture dates as distinct attributes. Full-content verification also does not prove that an image was never corrupted before it was first observed.

Use these row categories: source-only, destination-only, verified-equal, appears-unchanged-unverified, different-content, destination-edited-since-last-run, source-unavailable, destination-unavailable, excluded, unsupported-fidelity, blocked, and uncertain. Store the prior verified manifest as context, not as unquestionable truth about current content.

Show a side-by-side photo preview where decodable, full relative paths, sizes, timestamps with timezones/precision, content evidence, companion relationships and the proposed action. Permit text-only comparison for unsupported or damaged images. Initial sync has no trustworthy previous state; ambiguous differences require review.

Any file, scope or destination change after comparison invalidates relevant preconditions. A partial/inaccessible source inventory cannot generate removals. Directory renames initially use copy-and-retain rather than delete-and-recreate shortcuts.

## 5. Review actions and conflict handling

| Situation | Available action | Preservation rule |
| --- | --- | --- |
| Source-only | Copy | Exclusive staging, full read-back verification, no-replace publication |
| Destination-only | Keep | Never remove it merely because it is absent from the source |
| Different content | Compare; keep both; keep destination; replace destination and retain previous version | Explicit per-row or reviewed scoped decision; never replace source in this flow |
| Same content | Leave unchanged | Do not rewrite merely to normalize timestamps or attributes |
| File changes during read | Recompare | Invalidate evidence, keep originals and any untrusted partial separate |
| Network interruption | Pause; reconnect; bounded retry | Revalidate destination identity; inspect uncertain commits before replay |
| Destination full/quota exceeded | Pause; show storage details; choose a new destination | No automatic purge or deletion to free space; changing target creates a new plan |
| Access denied / locked file | Explain affected path; retry after remedy; skip explicitly | Do not elevate privileges or relax permissions automatically |
| Verification mismatch | Stop affected operation; preserve both; diagnose; retry as new attempt | No 'accept anyway' protection status or source removal |
| Invalid TLS certificate / changed SFTP host key | Stop; inspect connection trust | No generic ignore/override action |
| Uncertain commit | Inspect and recover | Never repeat a possibly completed replacement blindly |

'Compare' is a read-only inspection action, not an error bypass. A file-not-found or permission error is not resolved by an overwrite button. Replace is offered only when the preservation contract can be satisfied by the actual adapter.

'Keep both' displays the exact new name before approval and avoids all existing names/history identities. 'Keep destination' leaves the source untouched and reports that its differing revision was not copied. 'Skip' lists the unprotected item and makes completion partial. Bulk rules apply only to the selected, reviewed conflict category and plan; no permanent 'override all errors' preference.

## 6. State machine and safe transfer sequence

Profile states: not_configured, ready_to_compare, comparing, needs_review, approved, running, paused, needs_attention, needs_recovery, completed_verified, completed_partial, canceled, failed.

Per-item sequence:
1. Validate approved plan, source/destination identities, required recovery/history evidence, fidelity and storage budget.
2. Record durable intent, reserve an exclusive staging object and acquire the applicable concurrency/resource budget.
3. Stream the source with bounded buffers and content evidence; detect concurrent modification. Never disable source-change checks to get a green result [SB10].
4. Flush supported durability primitives and re-open/read the destination staging object. Verify full bytes against a stable source revision, including required companion/fidelity scope.
5. For approved replacement, preserve and verify the exact old destination version first. Commit only through tested conditional/no-replace semantics with revalidated target identity. An unsupported adapter must keep a separate versioned file or block replacement.
6. Record the resulting path, digest, version identity, preservation evidence and outcome; update catalog/job state. Leave the source untouched.

Crash recovery reconciles the journal with actual staged, history and published objects before continuing. A durable journal cannot make a network/filesystem transaction atomic. An incomplete bundle is reported as incomplete and blocks any later source cleanup. Never delete unrecognized temporary objects during cleanup.

Backends with weak publication semantics may support append-only versioned output without supporting replacement. Expose a capability matrix (exclusive-create, no-replace/conditional publish, durable flush, stable identity, link handling, metadata fidelity, read-back and safe history). Test capabilities against each actual filesystem/server; a protocol name alone is not evidence.

## 7. Transfer throttling and smoother I/O

The goal is to reduce this application's burstiness and contention, particularly on spinning disks and network shares. It is not a guarantee that operating-system caches, NAS buffers, other applications or physical drives will never generate bursts. FreeFileSync documents background-priority and verification controls, while rclone exposes bandwidth, concurrency and buffer controls [SB04, SB05].

Proposed presets are starting values to validate, not claims of optimal performance:

| Preset | Logical transfer cap | Active transfers | Per-file streams | Intent |
| --- | --- | --- | --- | --- |
| Quiet (default) | 10 MiB/s | 1 | 1 | Network/HDD-friendly starting point |
| Balanced | 40 MiB/s | At most 2, at most 1 per known HDD | 1 | Faster with bounded concurrency |
| Custom | User-set rate or explicitly unlimited | Explicit bounded count | Off by default | Device-specific tuning |

Display MiB/s versus Mbit/s correctly. Expose maximum source read rate, destination write rate, network upload/download rate, verification-read rate, concurrency and scheduled caps as separate controls only when they are actually enforceable for the chosen adapter. Apply global limits across jobs, not independently per worker. When source and destination share a physical device, use a shared device budget; do not multiply it by drive letter.

Use paced chunks with small bounded buffers and token buckets with limited burst capacity. Proposed maximum burst is min(256 KiB, 250 ms of the configured rate); reduce chunk sizes when necessary. Disabling a cap is explicit. Acquire budget before issuing I/O rather than copying a large buffer first and sleeping afterward. Avoid multiple large-file streams and aggressive read-ahead in Quiet mode.

Account for comparison, hashing, destination read-back, full-byte comparison, retries and history-copy I/O, not just outgoing photo bytes. This means 10 MiB/s of transfer may cause more total I/O; show actual logical read/write/network rates separately. Coordinate thumbnail/indexing jobs so they do not defeat the global limit. Keep the UI responsive and expose pause/resume plus dynamic limit changes without invalidating file evidence.

The Rust copy engine owns read/write pacing for ordinary-file transfers. For rclone, evaluate and pin --bwlimit, --transfers, --checkers, --buffer-size and --multi-thread-streams (0 disables multi-thread transfers) against the selected version [SB05]. Never assume a network cap is a source-disk cap. Restic/helper processes may not expose every requested local-I/O control; mark unsupported controls honestly, serialize competing jobs and test available limits. Do not claim exact physical I/O shaping from process priority or periodic process suspension.

Adaptive slowdown may reduce concurrency/rate on increasing latency or timeouts, within the user cap. It must never skip verification, change overwrite policy or delete history. Security, permission and content-integrity failures are not throughput signals to auto-ignore.

Record effective cap, supported scope, applied changes and measured bytes in job history. Test sustained application-issued rates and permitted burst envelope; separately report OS/NAS measurements as observations, not guaranteed ceilings. For an uninstrumentable helper, do not claim tests of internal source-read pacing.

## 8. Workspace and progress UX

Use the application's dark neutral style and clear spacing. The top row has 'Library: <name> (source)' and a destination selector, a mode selector and Compare. After comparison, show an action table with filters, a comparison inspector and a persistent summary of added, retained, replaced-with-history, blocked and unverified files.

Primary flow: Choose destination -> Compare -> Review changes -> Start safe sync / Start backup. Keep the source visibly fixed, show direction Library -> Destination, and never hide a direction reversal in an icon. A source picker remains unnecessary for ordinary usage.

While running, show separate phases (comparing, copying, verifying, preserving previous version, committing), counts, current rate, configured cap, bytes remaining, verification state and contextual issues. Do not show success when transfer reaches 100% but verification is pending. Time estimates are estimates, not guarantees.

Use a persistent issues drawer grouped by cause rather than a blocking dialog for every file. Offer the context-appropriate actions above, readable technical details, a redacted diagnostic export, and per-item outcomes. Avoid repeated prompts for an unchanged known error. Automatically continue independent safe items only if the user-approved policy permits it; global trust/integrity/identity errors pause the affected job.

History shows recoverable file versions and exact operation outcomes. Restore to a new folder by default. Never direct restic's default overwriting restore into an active library; stage and review separately [SB08].

## 9. Implementation contracts

Add typed Rust-domain records for SyncProfile, ComparisonRun, ComparisonEntry, ImmutableSyncPlan, TransferPolicy, PreservationEvidence, DestinationCapability, RetainedVersion, SyncIssue and ItemOutcome. Paths remain native/lossless; IDs and relative paths are validated by the backend. Include policy/schema versions and source/destination identity in persisted records.

Use typed IPC families such as sync.compare, sync.plan, sync.approve, sync.start, sync.pause, sync.resume, sync.set_limits, sync.resolve_issue and sync.history. The renderer cannot send raw executable arguments or authorize arbitrary paths. Approval/resolution changes generate a new digest or a journaled, narrowly scoped operational action.

Reuse the shared planner/executor, locking, catalog backup and credential adapters. Implement transport operations behind adapters, not a scattered collection of shell calls. Do not run generic rclone sync/move/delete/purge directly against libraries. Even rclone copy, while not deleting destination-only files, is not by itself the complete reviewed/versioned/no-overwrite workflow [SB06].

Keep existing snapshot verification distinctions: included, structurally checked, data verified, restored-and-compared. Structural checks do not read all repository data [SB07]. Do not issue strict source-mutation evidence based on repository structural checks or an unrelated sample restore.

## 10. Delivery gates

M4A: read-only source-fixed compare workspace, typed issues and fixtures. No mutation enabled.
M4B: safe append/copy with pacing, staging, full verification and crash recovery; original and destination-only files retained.
M4C: explicit retained-version replacement only on capable tested destinations, full recovery gate integration and conflict UX.
M4D: snapshot backup/restore integration, real transport round trips, native fault-injection and complete protection reporting.

M0-M3 prerequisites remain mandatory. Do not declare all M4 functionality complete after a UI-only demonstration. Execute the original acceptance backlog plus PRESERVATION_TESTS.md and record evidence. No real collection is used as a fixture.
