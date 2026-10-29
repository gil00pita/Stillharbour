# Stillharbour - Preservation contract - highest-priority product requirement

Version 1.1 supplement | 2026-10-23 | Normative design, not implemented behavior

Owner requirement: the most important outcome is not losing or damaging any file while discovering, organizing, backing up, or synchronizing photos.

Priority: preservation > recoverability > truthful status > responsiveness > speed > storage savings. When safety cannot be established, stop the affected operation and retain every existing version. No user-interface convenience, deadline, performance target, or conflict override may bypass this contract.

This document strengthens docs/CODEX_SPEC.md sections 5, 9-13 and the M2-M4 gates. Where the original specification permits weaker defaults, this contract wins. Unrelated original requirements remain in force. The owner-selected product name is Stillharbour. Trademark/domain clearance has not been established; naming never weakens the preservation contract.

## P-01. Read-only means no intentional source changes

Scanning, searching, content hashing, thumbnail generation, integrity analysis, comparison, and all previews must leave source bytes, paths, directory entries, and descriptive metadata unchanged. Store catalog data, logs and thumbnails outside source roots. Do not write housekeeping or lock files into read-only sources. Operating-system access-time updates caused by ordinary reads must not be represented as an application metadata edit.

Do not silently repair, transcode, rotate, strip metadata, change extensions, or delete a file because its decoder failed. Preserve damaged or unsupported files: an unreadable image may still contain recoverable data. Editing tags starts in the catalog; explicit sidecar export is separately protected.

## P-02. Preserve every relevant file, not just the displayed image

Treat RAWs, JPEGs, motion clips, XMP/ACR companions, edited exports and unknown non-photo files as potentially valuable. Never infer redundancy of a bundle from equality of one component. Inventory all descendants before a whole-folder operation. Unsupported or excluded entries must be reported rather than silently disappearing from a protection claim.

Backup scope is separate from gallery visibility filters. A full-library protection profile includes hidden and unrecognized regular files by default, except explicitly disclosed safety exclusions. No traversal outside the authorized root through links or junctions. Special files and links need a tested preservation policy or a clearly listed unsupported state. A profile with intentional omissions is called scoped, not complete protection of the entire folder.

Content hashes establish evidence about file bytes, not preservation of all filesystem metadata. Track timestamps, attributes, permissions, extended attributes, alternate data streams and resource forks separately where present. Do not destructively relocate a source when essential streams or metadata cannot be preserved and restored. A copy-only job may complete with an explicit fidelity warning while retaining its source; it is not an exact full-fidelity backup.

## P-03. Copy first; source mutation requires a recovery gate

Import and cross-volume organization default to verified copy with the source retained. The organization wizard defaults to creating an organized copy in a separate, nonoverlapping destination; in-place rename/move remains available only behind the gate below. This replaces the earlier default of directly moving selected originals.

Before enabling in-place bulk renaming, source relocation, duplicate quarantine, or explicit source/sidecar rewriting, require recovery evidence for the exact versions affected, including companions and original path mapping. The recovery copy must be on a separate device or host/failure domain, accessible at preflight, and verified by reading back the selected content (and essential metadata) or restoring and comparing that scope. A job-success flag or old snapshot of a different revision is insufficient.

A different folder or partition, an APFS clone, and a hard link do not establish independent-device protection. If independence cannot be detected, show that uncertainty and require explicit destination-role confirmation; do not advertise it as automatically verified physical independence. The application cannot independently establish the storage architecture of every NAS.

Do not expire or automatically prune recovery versions supporting an outstanding operation/recovery record. Changing the source revision invalidates the gate. Revalidate immediately before mutation. If the protected copy becomes unavailable, pause remaining source mutations. Copy-only and read-only work can continue when their own preconditions permit it.

M2 and M3 may implement and test mutation using disposable fixtures before M4 exists. They must not enable it on real libraries until a verified recovery provider and the applicable safety gates pass. This is a dependency gate, not permission to waive backup checks to demonstrate a feature.

## P-04. Review the exact operation, then revalidate

The backend creates an immutable plan: source/destination identities, revisions, paths, companions, before/after names, expected absence or expected target revision, required recovery evidence and reason. Approval is bound to its digest and scope. Changed content, path identity, policy, destination, or user resolution requires a new plan revision and review.

Do not approve an unresolved conflict. Destination nesting, aliases, reused mount points, symlink/reparse escapes and recursive backup scope must be rejected. A disconnected source is not an empty source. A missing, unreadable or partly scanned directory never authorizes deletion elsewhere.

## P-05. Verified publication, never in-place truncation

New copies are written to exclusive, job-owned staging objects, never directly over an existing destination. Stream with bounded buffers; record full-content evidence, flush supported durability primitives, close/reopen and independently read back the destination. Require exact full-byte comparison for ordinary-file copies in preservation mode. For encrypted snapshots, use the engine's authenticated content verification plus a scoped restore/read-back comparison when issuing mutation-gate evidence.

Use stable source handles and platform sharing restrictions where available. Detect changes during reading and invalidate affected evidence. A file-level copy is not an atomic whole-directory snapshot. If an adversarial or undetectable concurrent writer cannot be excluded, do not claim immunity to it.

Only publish with backend-tested no-replace or compare-and-publish semantics. If a target appears or changes, preserve it and return to review. On a transport without a safe replacement primitive, publish a new versioned name in an exclusive application-owned namespace or block replacement. Do not emulate safety with check-then-overwrite.

Never remove the source as an implicit transfer-completion step. Any later source quarantine is a separate approved, journaled operation with its own recovery evidence. An incompletely verified copy cannot satisfy that evidence.

## P-06. Replacement retains the old destination version

The user action is 'Replace destination and retain previous version', not a universal 'Force' or 'Override'. It applies only to a specific reviewed file conflict.

Before replacing a destination, preserve its exact previous bytes and relevant metadata under a unique immutable history identity; record and verify that history. Do not reuse a filename that could overwrite an earlier history version. Revalidate the currently installed destination against the reviewed revision before publication. Keep old and incoming copies if commit safety is uncertain.

No automatic expiry of history or quarantine in the initial release. Running out of space pauses the operation; it does not trigger deletion of originals, historical versions or unknown temporary files. Capacity estimates include staging, verification/restore space, old versions and a configurable reserve, but do not replace runtime free-space handling.

## P-07. Duplicate cleanup is conservative and recoverable

Only fresh full-byte equality can qualify exact duplicate candidates for quarantine. Similarity, equal filenames, equal dates, same rendered pixels, or a cached hash alone never authorize removal. Keep an accessible verified keeper outside quarantine and independent recovery evidence for the affected version. Never count an offline drive, thumbnail, another hard-linked path, or unavailable backup as the sole protected survivor.

Quarantine retains content and a recoverable path mapping. It is not permanent deletion, independent backup, or immediate reclaimed disk space. Offer restore through a new collision-safe plan. First release has no automatic purge and no 'remove all originals' shortcut. Protected folders and unique companions cannot be bypassed by selecting all rows.

## P-08. Journal, recover, and report honestly

Journal intent durably before mutation and completion after the filesystem result. A database transaction is not a transaction over arbitrary filesystem/network operations. Recovery inspects identities and contents before deciding what occurred; it never blindly repeats replacement or deletion after an unknown response.

Cancellation, disk-full, disconnection, denied permissions, helper crashes and application termination must leave existing content intact or in an identified recoverable location. Rollback is itself a previewed operation. If another application has occupied an old path, retain both versions and request review rather than overwrite it.

Use states such as completed_verified, completed_with_unverified_items, needs_attention, paused_offline, canceled_with_partial_copies, and needs_recovery. Skipped or failed items remain visible. Never mark a full library verified because one sample restore passed. Verification evidence is tied to file revision, scope, method and time.

## P-09. Fail closed on dangerous conditions

No 'continue anyway' for wrong destination identity, escaped paths, changed SFTP host identity, invalid TLS certificates, verification mismatch, lost recovery evidence, protected-keeper removal, or uncertain commit state. Diagnose and re-establish trust or replan instead. Recoverable network errors may get bounded backoff; disk-full or permission errors need a relevant remedy, not endless retries.

Corruption suspicion must never trigger automatic replacement of an apparently damaged backup with a possibly damaged source, or vice versa. Preserve both, show independent verification evidence, and use a separate reviewed restore/repair workflow. Decoding a thumbnail is not proof of full image integrity.

## P-10. Test before enabling real mutations

Every mutating state transition needs fault injection, restart recovery and idempotency tests. Test source/destination manifests before and after failures, including unique destination-only content, hidden files, companions and metadata fixtures. Run native Windows/macOS and real transport tests before claiming those capabilities.

No tests against a real photo collection, no mocked success in production, and no 'passed' status for unrun scenarios. See PRESERVATION_TESTS.md. Release blockers include any unexplained loss, truncation, silent omission, unsafe overwrite, or false protection claim.

## Limits and protection guidance

This is an engineering objective and a release gate, not a promise that software can eliminate hardware failure, fire, theft, malicious external changes, lost encryption keys, or every operating-system defect. Explain residual limits plainly. Recommend independent backup copies, including an off-device/offline or off-site copy, and periodically demonstrate restoration. Same-volume version history helps undo mistakes but is not independent disaster protection.

For actual upstream behavior, consult BACKUP_SYNC_SOURCES.md: verification may be served from caches [SB04], ordinary restore defaults may overwrite existing files [SB08], and snapshot structural checks differ from full data verification [SB07]. The application must enforce this stricter contract rather than inherit unsafe defaults.

## Version 1.2 clarification - 2026-10-26

The full contract above remains binding. Integrity audits cannot auto-repair or erase old hash evidence. Crop/rotate/mirror and tags default to catalog-only revisions; no original or sidecar rewriting is implicit. Catalog snapshots protect organization/recipe/baseline data but do not themselves contain or independently protect original media.

Provider archives, source folders/accounts, raw metadata artifacts and all related media resources remain preserved. Verified local copying proves only the bytes available through that route; it does not establish whole-cloud coverage, camera-original fidelity or preservation of metadata/resources the provider did not supply. Opening an old/imported catalog cannot replay stale operations or reuse stale approval/recovery evidence. See LIBRARY_FEATURES_ADDENDUM.md for the normative extension.
