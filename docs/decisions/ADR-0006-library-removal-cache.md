# ADR-0006 — Forget an indexed library and clear owned previews

Date: 2026-10-28. Status: accepted for the owner-requested M1 Settings extension.

## Decision

Provide per-root Remove library and a separate all-library Clear preview cache action. The former intentionally forgets indexed records, catalog annotations and scan history with an explicit warning; the latter removes only regenerable indexed previews. Neither touches original folders or sidecars. User confirmation follows a native label/path/count review. Cache counts are observations; the approved scope is one fixed root UUID or all index-owned previews.

Hold one opaque native plan for at most five minutes, bound to the catalog revision. Execution consumes it and rejects stale/replayed/invented approvals. Take existing scan admission before maintenance; an active scan must finish or be cancelled in Jobs. Cancel pending thumbnail work, stop new admission and hold a write barrier that excludes worker publication/cache reads. Queue workers hold the read barrier for their complete build/publication, so no result can repopulate cleared keys after the operation. All command work runs off the native async/UI executor. No new framework or dependency.

Centralize file deletion in CacheCleanup. Open the private thumbnails directory without following a link; hold its descriptor. Only 64-hex keys obtained from the catalog's cache index may form relative PNG names. Open with no-follow/nonblocking flags, require a regular single-link file with bounded PNG signature, check its current identity, then unlinkat without directory recursion and sync the directory. A replaced path/ancestor does not redirect the held descriptor. Unknown/unindexed cache content stays retained. Checks detect ordinary replacement; this does not claim race-proof authorization against a hostile process writing the private directory.

Stream at most 256 keys per SQL request. Keep index rows until cleanup/sync succeed, making them the existing durable retry list. On partial cleanup, metadata stays registered and already cleared previews can regenerate; a new review can retry missing files idempotently. Recheck catalog revision inside the final foreign-key-enforced transaction. Remove the target's selection tokens, cache rows, issues, scans, directory observations, assets and root in dependency order. Promote a remaining base root without replacing any surviving UUID. Cache-only cleanup removes index rows and marks ready previews pending without changing notes. A crash around commit is recovered by SQLite; no schema migration is needed.

The separate preferences file is not part of the catalog transaction. Effective native preference reads map a no-longer-registered default root to All libraries without silently rewriting the file. Future explicit Save defaults persists that neutral value. Original absence/offline state does not prevent forgetting a root; this operation does not open its original files. Re-adding is a new registration, without automatic annotation recovery. No secure erasure, catalog snapshot/Undo, backup repository or M2 mutation engine is implied.

## Validation and sources

Generated fixtures cover per-root cache isolation, surviving notes/base UUID, foreign keys, stale/replayed plans, scan admission, partial failure/retry, offline removal, last-root reopen, cache regeneration and unchanged source manifests. Descriptor tests cover traversal, link/hardlink/invalid content and directory replacement. Native Settings tests cover confirmation/cancel, real Rust cleanup, effective default fallback and minimum-window layout. Actual results/remaining gates are recorded in IMPLEMENTATION_LOG.md.

Primary references checked: the installed macOS open(2)/unlink(2) manuals (captured under .context/evidence/remove-library-*-man.txt) and [SQLite foreign-key support](https://www.sqlite.org/foreignkeys.html). Open Group online openat/unlinkat requests returned403; no successful read of those pages is claimed. Windows/Linux behavior is not established by macOS fixture results.
