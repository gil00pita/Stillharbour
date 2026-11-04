# Using Stillharbour during development

Stillharbour is a local desktop photo organizer. The current build browses folders and stores organization in a local catalog. **Signed distribution and clean-machine release acceptance are pending.** See [release readiness](M6_RELEASE_READINESS.md) and [format support](FORMAT_SUPPORT.md) before relying on a capability. No account or telemetry is enabled.

## Add and browse folders

Choose a folder, review its name and scan settings, then select **Save & Start scanning**. Selection permits the reviewed scan; it does not authorize moving or deleting originals. Development tests must use disposable generated folders.

New folders default to **Managed**. Turning on **Read only** saves a referenced root. Both modes currently read originals; Managed is not a blanket write grant. Additional folders must not overlap existing roots. Change location in Settings uses explicit native selection and identity checks. A different disk at the same path is not automatically trusted.

Advanced settings control hidden files, hidden folders, exclusions, extensions and size limits. Hidden-folder traversal and hidden-file inclusion are independent. The 34-extension preset includes MP4 for discovery; this does not establish video playback or every RAW decoder. Existing saved filters keep their scope. Scan filters affect the catalog view, not a promise of full-folder backup coverage.

Use Photos for timeline/grid/text views, Folders for immediate indexed children, filename search and Filter for query choices. Apply commits filter drafts; Close discards them. Text list does not start image-preview work. Explicit Select all results includes unloaded matches through a backend selection; day selection covers only the displayed group. Jobs shows actual work, cancellation requests, retained outcomes and issues. A running discovery has no known total, so its activity bar is not a completion percentage.

Opening a photo gives zoom, pan, neighboring photos, filmstrip and Info. Original-quality viewing is limited to admitted formats and sizes; labelled derived previews remain previews. Closing returns to the collection. Visual Compare of 2–4 selections establishes no duplicate equality.

An offline folder retains catalog records and available cached previews. Offline, inaccessible, missing, unsupported and failed are different states. Reconnect or explicitly relink the original folder, then retry the affected work. A failed listing does not mean the folder is empty. Unsupported/damaged inputs stay preserved; the app does not repair or delete them automatically.

## Catalog annotations and exports

Favourites, ratings, tags, captions and reject flags save to the catalog. Albums are virtual memberships and saved searches store filters. They do not move photos. Catalog provides explicit tag rename/merge/removal review, retained undo history and observed-keyword import; a stale edit requires resolving the current version. Unsaved caption/review drafts last only for the current bounded session and are lost on closure or catalog switching. Save explicitly.

Portable catalog export creates a consistent SQLite copy in a separately chosen folder. It includes organization, not original media, credentials, schedules or live root permissions. It is not a backup of your photographs. Opening/recovering catalogs requires the supported native workflow and fresh root authorization; moving an active database into a network/cloud-sync folder is unsupported. General File > Open/import of arbitrary portable catalogs remains separately gated.

XMP export previews the exact new output files and requires approval. It preserves supported unknown metadata and original companion artifacts; unsupported or ambiguous packets block export. Existing source sidecars and destination files are not overwritten. If sources or the destination change, preview again. A completed export verifies those outputs, not every original or every metadata ecosystem. See [metadata documentation](M5_METADATA.md).

## Organization and duplicates

Organize previews templates for up to 240 explicit photos. Review Before/After, missing-date rules, exact proposed names, conflicts and companions. A preview or saved JSON/CSV plan changes no originals. Companion candidates need explicit confirmation; equal JPEGs cannot prove unique RAW, motion or sidecar components redundant. CSV review output protects spreadsheet formula prefixes; JSON preserves the review data without spreadsheet interpretation.

Production copying, renaming, moving, quarantine and undo remain gated. Generated laboratory controls in instrumented development builds operate only on their own disposable files. They grant no authority over a personal library. A future real operation requires its immutable reviewed plan, separate approval, fresh identities and appropriate independent recovery evidence.

Duplicates distinguishes exact content, file-property candidates and the declared static PNG8 normalized-pixel policy. Normalized pixels require manual review and do not establish identical files. Perceptual visual similarity is unavailable. Keeper choices respect protected folders, accessible independent objects, fresh full-byte checks and complete companions. Cached hashes, thumbnails and offline keepers cannot justify removal. Quarantine would retain files and free no same-volume data blocks; no production quarantine/purge action is available.

## Integrity and storage

Quick checks and deep decodes have different coverage. Deep validation is bounded to supported static PNG/JPEG/WebP, not full RAW data, every animation frame or multipage content. A checksum mismatch means bytes changed since the recorded baseline; it does not alone prove damage. Keep earlier evidence and known-good copies. No automatic repair follows.

Storage distinguishes logical size, unique filesystem objects, reported allocation and unknown physical reclaim. Hard links are not independent backups. Sparse files, clones, snapshots and links outside scope can prevent exact savings estimates. Folder-table sorting is within its retained page. Partial/excluded/unreadable scopes remain visible. See [inspection limits](M3_INSPECTIONS.md).

## Backup, keys and recovery

The default build offers read-only comparison of the bound base library and a separate destination. Quick equality is unverified; complete byte comparison has stronger evidence. Destination-only files are retained. A saved comparison profile stays bound to its original root IDs, even if the default changes. Comparison creates no readable mirror, snapshot or recovery guarantee.

Production Safe copy, retained replacement, encrypted snapshots, restore and schedules remain disabled pending their native preservation gates. The following describes the implemented experimental workflow and its limits, not an enabled production service:

- Safe copy retains sources and destination-only files. Replacing a reviewed destination requires retaining and verifying its previous version; there is no universal Force action.
- Encrypted snapshots use pinned restic/rclone helpers. They are not browsable filename mirrors. Store the repository key separately and securely through the reviewed recovery-key workflow; losing it can make the repository unrecoverable. Never put secrets into a catalog export or support log.
- “Included in snapshot,” “structure checked,” “data read verified” and “restored and compared” are distinct evidence levels. A sample restore does not verify a whole library. Partial/offline/failed items stay unprotected.
- Restore defaults to reviewed new output, compares selected restored bytes and retains existing neighbors. Catalog activation is separate, retains the current consistent catalog and takes effect after reopening. Wrong keys, changed roots, collisions or ambiguous results block progress. No surprise unlock, repair, prune or forget runs.
- Experimental schedules run only while the app/tray process lives, for an exact unchanged reviewed manifest. They use elapsed UTC intervals; changed files/endpoints need fresh review. Catch-up allows at most one run. With catch-up off, a deadline over 30 seconds overdue is skipped. Fully quit execution has no background agent.
- Owned transfer/comparison/verification I/O uses shared limits. Helper network caps do not cap every helper disk read or physical-device request. Full fidelity, transport and filesystem claims require their own tests.

If work is interrupted, keep sources, staged outputs, history, catalog and repository objects. Inspect the explicit outcome and obtain fresh review before any continuation. Do not manually delete unfamiliar temporary/history files or force-unlock a repository to make a warning disappear. Same-disk history, clones and hard links do not provide independent disaster recovery. Keep independent off-device/off-site protection and periodically demonstrate restoration.

## Map, privacy and support limits

Map uses a bundled offline world overview with attribution, not street-level online tiles or geocoding. Missing GPS is separate from valid zero coordinates. The photo list works without graphics. Existing scans may need an explicitly requested rescan to acquire previously unextracted GPS; a preview does not silently rescan. No online map provider is configured.

The tested primary paths are Apple Silicon macOS and bounded Windows local NTFS, with exact format/OS evidence in [format support](FORMAT_SUPPORT.md) and [M5 closeout](M5_CLOSEOUT.md). Configured minimum macOS, Intel builds, network-root behavior, untested cameras/color/HDR, cloud imports, basic editing and mobile are not implied supported capabilities. Updates remain disabled pending signature-verification acceptance. Keyboard/focus evidence and a bounded owner VoiceOver review exist; full-app VoiceOver/NVDA, high-DPI and reduced-motion acceptance remain M6 work.
