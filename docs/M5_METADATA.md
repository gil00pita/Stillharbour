# M5 catalog metadata continuation

Current gate, 2026-11-03: [M5 closeout](M5_CLOSEOUT.md) accepts the implemented M5 development scope, including schema-17 organization, reversible tags, keyword overrides and reviewed catalog/XMP exports, against passing Windows/macOS CI and inspected native receipts. Earlier unavailable/open statements below describe preceding increments. Release and production original-file gates remain explicit.

Current evidence, 2026-11-03: native Windows M5 checks pass at `a2b3d10`; fresh local map and metadata/export audits preserve all nine generated source/companion entries. The owner passes the requested actual VoiceOver map/heading/dialog checks, including Catalog form labels and focus behavior. [M5 validation](M5_VALIDATION.md) separates that manual result from automated evidence and tracks remaining PR-wide validation. Earlier limitations below remain historical.

Current continuation: [ADR-0084](decisions/ADR-0084-reviewed-metadata-exports-and-tag-history.md) and [M5 validation](M5_VALIDATION.md) add schema-17 tag rename/merge/removal/undo, explicit keyword import overrides and reviewed portable catalog/XMP export. Current activation uses schema 17; the schema-16 snapshot and measurements below remain historical evidence. The unavailable/open statements describe that earlier increment, not current controls.

The owner's “continue” request on 2026-11-02 continues the authorized M5 milestone. [ADR-0082](decisions/ADR-0082-catalog-metadata-organization.md) defines this bounded catalog-only increment. M5 remains in progress, with M4 production mutation restrictions intact.

## Behavior and limits

Open **Catalog** from the Photos header to manage virtual albums, saved searches and tag categories. **Add to album** in the selected-photo toolbar opens the same review with that explicit/frozen selection. Album membership changes only catalog references. Archive/restore retains members; collection rename never renames files. Album filters use normal keyset paging and retain offline media records when included in the query.

Save the current library filters as a named search; viewing it runs those filters against the current catalog. Updating its filters requires **Replace saved filters with current filters**. Saved searches are validated data, with no SQL, scripts, schedules, root grants or automatic jobs. Archived searches cannot be viewed until restored. Collection count is limited to 200 including archived entries; names are at most 128 UTF-8 bytes. Search JSON is at most 8192 bytes.

The photo Info inspector now saves captions and reject flags with optimistic version checking. A stale save retains the draft and explains reload; reloading explicitly discards the draft. Captions allow up to 4096 UTF-8 bytes, tabs and line breaks. Each successful save atomically retains an immutable saved-version history row and bumps the catalog revision. If history persistence fails, the entire note write rolls back. The history retains every successfully saved version; the unsaved default version zero has no history row. Current notes are forgotten with a removed asset, while immutable audit rows remain retained by its old stable ID.

Embedded Keywords/Subject are displayed separately as last-observed provenance, capped at 64 labels of 128 bytes, from at most 1 MiB of retained raw metadata. The owner-approved 2026-11-06 [Albums / Tags](ALBUMS_TAGS.md) slice now merges validated observations during authorized fresh/cached scans while retaining provenance and removal overrides; the earlier manual-import-only behavior is superseded. Removing a manual tag and rescanning cannot resurrect it from the source observation. This is separate provenance display, not the full N2 imported-tag/tombstone workflow.

Tag categories connect exact labels by explicit parent IDs. They do not interpret slashes as separators or normalize case/Unicode. Existing flat labels stay intact; a parent view includes its own label and descendants. Limit categories to 512 and depth to sixteen. Cycle, missing-parent and stale-version edits fail. Category arrangement preserves labels; tag rename/merge and undo remain unavailable.

Album edits require the expected collection version and exact selection count, preventing partial success when a selected asset disappears. Explicit selections use the existing maximum of 500 IDs; a backend-frozen set supports up to 50,000 and expires after one hour. Creating a collection after freezing is allowed because it does not change the captured asset IDs. Duplicate membership is idempotent. Archived collections retain their names and memberships.

Rating/reject/category/album predicates share ordinary library/map queries, counts and neighbor paging. Favorites, ratings and flat tags retain their existing behavior. Neither these commands nor the notes UI open originals, sidecars or helpers. There is no new metadata export button claiming success.

## Catalog upgrade

Schema 16 adds notes/history, virtual collection membership and explicit category relationships. Schemas 14 and 15 retain their original compiled definitions. Identity-bound startup validates the exact known historical layout before migrating; unknown schemas remain blocked. The normal transactional path retains a consistent pre-migration catalog and preserves UUIDs, roots and existing annotations. Current restore activation requires the exact schema-16 layout; old-schema restore activation remains a separate reviewed gate. No manual SQLite or Keychain reset is required; restart the rebuilt app to load current code.

## Verification

Commands:

```sh
node scripts/cargo.mjs test --locked -p stillharbour-core --lib --features fixture-probes,copy-workflow,backup-workflow catalog_metadata::
node scripts/cargo.mjs test --locked -p stillharbour-core --lib catalog::
npm test -- src/metadata src/library/BatchToolbar.test.tsx
npm run desktop:test:map
```

The native map command builds/stages its own instrumented executable and now also exercises metadata organization. It scans only its constructor-owned generated EXIF JPEG/companion fixture and accepts no personal-library path. The combined GPS and separate metadata/export scenarios now execute on both target OSes; actual per-platform receipts in M5 validation determine acceptance, not portable SQL tests. Production frontend rebuilding must follow instrumented testing before release checks.

Actual local evidence: eleven metadata core cases pass in 0.84s, including stale updates, history rollback, consistent independently openable snapshots, offline video membership, exact selections, rescans, hostile saved-filter data and hierarchy bounds. The initial integer conversion compile errors, strict Clippy nested-if warning and frozen-fixture FK cleanup error were corrected; no preservation assertion was removed.

The combined macOS native run passes in 7.3s (`.context/m5-metadata-native-minimum.log`), after the rebuilt-toolbar run passed in 5.2s. UI-created album/search/category through the selected-photo Add to album review, descendant query, rating/reject filters and Unicode caption persistence use actual native IPC. The final independent manifest verifies all nine generated image/companion entries unchanged and original-mutation capability false. Minimum-width dialog fit is checked at 660×560. Screenshots `catalog-organization.png`, `catalog-minimum.png` and `catalog-notes.png` under `.context/m5-native` were inspected. An earlier run stopped on the map-return control; waiting for the displayed map and counted location control passes without changing application behavior. Main-WebView resource timing retains its limited scope; no worker/OS interception is claimed.

All 455 frontend tests in 69 files pass, as do strict types/lint, generated bindings, Rust formatting, default-core and workspace/desktop all-target Clippy with warnings denied. Catalog tests pass all 27 cases in 25.46s, including 42 actual migration-exit boundaries through schemas 2–15 and identity-bound startup for 13–15. Production frontend build/resource checks pass; the lazy map chunk warning remains visible. The implementation log records the full-workspace outcome separately. The schema-16 full workspace attempt records 472 core passes / 10 failures / 15 ignored in 284.59s (`.context/m5-workspace-v16.log`). Its ten failing names match the previous schema-15 run: native metadata/replacement fixtures reject host macOS extended attributes/resource forks. The unchanged-main native-metadata reproduction remains retained. The workspace command stops at core failure, so later integration targets are not claimed. The final eleven-case focused metadata run additionally includes the expiry regression added during the full run. No preservation policy or assertion was weakened.

## Remaining gate

Portable catalog export, conflict-safe XMP export preserving unknown namespaces/companions, tag rename/merge/undo and full provenance import overrides remain open. So do packaged clean-machine format/platform checks, Windows native metadata acceptance, full keyboard/screen-reader review, M4 preservation certification and M6 release work. No cloud import, editor or mobile feature is added.

### Drafts and organization review polish — 2026-11-02

Caption/reject drafts are scoped to the open catalog UI session, bounded to 240 unsaved photos and retained through browsing/Info/viewer changes. They are not persisted and do not survive app closure or catalog switching; the editor explicitly says to save first. Retained drafts keep their original expected version, so a conflicting external catalog update still fails closed. A successful explicit reload discards the draft; failed reload does not. A late save only removes the exact submitted draft.

Organization keeps its title/Close and section shortcuts visible while the form scrolls. Saved-search replacement requires a saved-versus-proposed filter review with a frozen query and explicit confirmation. Human-readable library/album names are used where available; unavailable references remain explicit. No originals, sidecars or backend approval rules change.
