# M5 local map increment

Owner continuation, 2026-11-05: [offline place search](PLACE_SEARCH.md) adds Places and GPS-based city/country filters from bundled GeoNames/Natural Earth data. No online provider, reverse geocoding or preservation gate changes. Earlier map-only scope/evidence below remains historical; current feature/platform results belong in the implementation log.

Current gate, 2026-11-03: [M5 closeout](M5_CLOSEOUT.md) accepts the implemented M5 development scope against final PR #16's passing Windows/macOS CI and inspected native map/export receipts. Earlier incomplete/pending statements below retain their checkpoint scope. Clean-machine, broader format/accessibility and production original-file gates remain explicit.

Current evidence, 2026-11-03: Windows Foundation at `a2b3d10` passes the native M5 scenarios; fresh local map/navigation and metadata/export tests pass with nine unchanged source/companion entries each. The owner passes the bounded actual VoiceOver map, heading and Catalog-dialog review. [M5 validation](M5_VALIDATION.md) records exact scope and remaining PR-wide checks; the older open statements below retain their historical scope.

The owner requested continuing M5 on 2026-11-02. [ADR-0081](decisions/ADR-0081-offline-geographic-browsing.md) records the first geographic slice. M4 preservation certification and production mutation restrictions remain unchanged; M5 as a whole is not complete.

## Implemented scope

The Map sidebar opens an offline world overview with catalog-derived thumbnail clusters, folder/date/filename filters and a paged photo strip. Selecting a cluster restricts the strip to its exact viewport/grid membership. Show in library carries those filters and the representative selection; Return to map retains map context. Photos can use the existing native viewer and catalog annotation commands. Coordinates are read-only observations, with no manual geotagging or embedded metadata write.

“No usable location” includes missing, invalid and unextracted GPS. Both numeric zero coordinates are valid. Offline roots keep retained observations; map presence never proves source accessibility or integrity. Existing scans generally lack GPS because the prior ExifTool allowlist omitted it. Explicitly rescanning a generated/authorized root can acquire GPS; this increment performs no automatic rescan. The legacy Windows basic adapter did not extract GPS; ADR-0087 adds bounded EXIF/GPS in its supervised worker, with legacy observations refreshed on explicit rescan. Fresh native Windows validation remains required.

Backend clusters are bounded to 256, using a world-aligned angular grid with adaptive coarsening. The strip requests 80 summaries per keyset page. R-tree candidate bounds are checked against exact doubles; antimeridian and identical-location cases are covered. Root/date/tag and ordinary library filters share one SQL predicate path. Pending/stale responses do not replace another scope's results; incompatible map/page revisions require retry. Mercator cannot show the poles, so polar markers are clamped to its edge and polar photos remain in the edge lists.

## Provider, network and rights

The production data path is bundled Natural Earth 1:110m land, pinned to upstream commit `ca96624a56bd078437bca8184e78163e5039ad19`. [Natural Earth's terms](https://www.naturalearthdata.com/about/terms-of-use/) permit public-domain redistribution and offline use. The original data/license and hashes are in `assets/maps`; no reference screenshot is shipped as map data. Attribution remains visible even though the license does not require credit.

There is no online provider, tile cache, prefetch or runtime data download. World overview geometry is compiled into the lazy basemap module; the MapLibre module worker is bundled locally with one worker. No glyphs, external sprites, style URL or reverse geocoder are configured. The map request transformer rejects resource URLs, and the existing restricted production CSP remains intact. The overview deliberately has no street detail. Adding online detail requires a new reviewed provider decision.

Native resource-timing observations cover the main WebView, not independently intercepted worker/OS traffic. Do not describe a static source/CSP inspection or browser test as a complete native network trace. Actual native outcomes, screenshots and the precise evidence boundary are recorded below when run.

## Catalog and validation

Schema 14 introduced the spatial tables; schema 15 adds strict numeric-text extraction for the pinned ExifTool JSONQ output. Startup verifies the exact compiled historical layout before migrating a bound catalog. It retains a consistent pre-migration snapshot, backfills usable observations and maintains the spatial index through metadata updates/deletions. Migration name collisions roll back; no existing table is adopted or overwritten. UUIDs, roots and annotations remain unchanged. The map checkpoint required schema 15; the later [metadata/export continuation](M5_VALIDATION.md) advances current activation to schema 17; old-schema restore activation remains unavailable until a separate reviewed migration path exists.

Core tests: `node scripts/cargo.mjs test --locked -p stillharbour-core --lib --features fixture-probes,copy-workflow,backup-workflow geography::`. Migration: `node scripts/cargo.mjs test --locked -p stillharbour-core --lib geographic_migration_tests`, plus existing catalog migration/process-exit tests. UI: `npm test -- src/map/MapScreen.test.tsx`. Native: `npm run desktop:test:map` builds and stages an isolated instrumented executable beside the Cargo debug resources, then runs wdio.map.conf.mjs. The isolated name avoids replacement by a concurrent normal development watcher. This uses constructor-owned generated JPEGs with real EXIF GPS and an independently hashed companion; it accepts no personal-library path. The native GPS scenario is macOS-only while Windows extraction remains unavailable. Run the documented full checks and rebuild the production frontend before `release:check`.

Initial observed checks: six geographic core tests pass; nine map component tests and the 444-test frontend suite pass. Strict frontend types/lint and the production frontend build pass. Native/migration/complete workspace checks are still being finalized; no pass is claimed until recorded. The first fixture-enabled compile failed on an ambiguous integer denominator in generated EXIF construction; it was corrected with an explicit `u32`. The initial graphics-fallback component case needed to await the independent photo request; its corrected test passes without changing application behavior. Preserve these failures in the implementation log.

## Added dependency inventory

MapLibre GL JS 6.16.0 is pinned in package.json and package-lock.json. The 23 newly added package entries declare the following versions/licenses (inventory only, not completed distribution legal review):

| Package                           | Version   | Declared license  |
| --------------------------------- | --------- | ----------------- |
| maplibre-gl                       | 6.16.0    | BSD-3-Clause      |
| @mapbox/jsonlint-lines-primitives | 2.0.3     | MIT               |
| @mapbox/point-geometry            | 1.1.0     | ISC               |
| @mapbox/tiny-sdf                  | 2.3.0     | BSD-2-Clause      |
| @mapbox/unitbezier                | 1.0.0     | BSD-2-Clause      |
| @mapbox/vector-tile               | 3.0.0     | BSD-3-Clause      |
| @maplibre/geojson-vt              | 6.3.0     | ISC               |
| @maplibre/maplibre-gl-style-spec  | 26.5.1    | ISC               |
| @maplibre/mlt                     | 1.3.0     | MIT OR Apache-2.0 |
| @maplibre/vt-pbf                  | 4.3.3     | MIT               |
| @types/geojson                    | 7946.0.16 | MIT               |
| earcut                            | 3.2.4     | ISC               |
| gl-matrix                         | 3.4.4     | MIT               |
| json-stringify-pretty-compact     | 4.0.0     | MIT               |
| kdbush                            | 4.1.0     | ISC               |
| minimist                          | 1.2.8     | MIT               |
| murmurhash-js                     | 1.0.0     | MIT               |
| pbf                               | 5.1.2     | BSD-3-Clause      |
| potpack                           | 2.1.0     | ISC               |
| protocol-buffers-schema           | 3.6.1     | MIT               |
| quickselect                       | 3.0.0     | ISC               |
| resolve-protobuf-schema           | 2.1.0     | MIT               |
| tinyqueue                         | 3.0.0     | ISC               |

`npm audit --omit=dev --json` reports zero vulnerabilities for the installed production graph (`.context/m5-npm-production-audit.json`). Install reported 33 advisories including development dependencies; no forced dependency change was applied. No native binary or codec was added. The lazy basemap chunk is approximately 1.22 MB before gzip and its worker approximately 519 kB; Vite's existing chunk-size warning remains visible.

## Final local map checkpoint

The actual macOS desktop round trip passes in 3.4 seconds (`.context/m5-native-zoom-success.log`). It renders the bundled geometry/worker, verifies six usable and two unavailable GPS observations from generated EXIF, zero/identical points, signed hemispheres and antimeridian queries, uses the visible zoom control, selects a thumbnail cluster, carries its scope/selection into the library, returns to the map and opens the no-location list. The independent final manifest verifies all nine generated originals/companion entries unchanged; original mutation capability remains false. Screenshots `overview.png`, `after-marker.png` and `minimum.png`, plus `map.json`, are under `.context/m5-native`; overview/minimum images were inspected.

Main-WebView resource timing records no external HTTP(S) resources. This is scoped native resource evidence, not independent interception of worker/OS traffic. Only the closed generated fixture was scanned; the owner's development catalog was inspected read-only for schema/version diagnosis, without querying a photo library or resetting native authority. The map checkpoint covered the known schema-14-to-15 upgrade; the later metadata increment advances the current catalog to schema 16. Restart the rebuilt app to load current code; direct edits to SQLite or Keychain are unnecessary.

Ten geographic/helper cases pass in 1.71 seconds, including strict JSONQ validation and backfill from the original schema-14 layout. All 27 catalog cases pass in 5.86 seconds, with 39 actual migration-exit boundaries through schemas 2–14; schemas 13/14 use the identity-bound startup path. These preserve the old consistent catalog at snapshot, pre-commit and post-commit boundaries and reject unfamiliar schema before imported data queries. Strict workspace/desktop all-target Clippy, default-core Clippy, Rust formatting, IPC bindings, frontend type/lint/build and all 444 component tests in 67 files pass their recorded local checks. Production resource checks verify the map hashes, pinned helpers and exclusion of test routes. The full workspace's host-metadata failures are retained below, not called passing.

Native failures uncovered and corrected an initialization/resize `moveend` that revoked a chosen list after map return. Only actual camera interaction now changes map scope; minimum-size reflow preserves list choice. The new zoom test initially assumed that zooming must increase marker count, which is false when the smaller viewport contains only identical coordinates; it now asserts a smaller nonempty matching photo scope and enabled zoom-out control.

The final schema-15 full workspace rerun records **462 passed / 10 failed / 15 ignored** in 272.71 seconds (`.context/m5-workspace-final-v15.log`), retaining the same ten unsupported macOS metadata failures. The complete command stops at those core failures, so later workspace integration targets are not inferred to have run. A final component regression keeps the map photo viewer open across catalog/annotation refreshes without changing its geographic filter scope; the final frontend suite is **445 tests in 67 files**, with strict type/lint/build and production checks passing again. Earlier counts above retain their checkpoint scope.

## Remaining milestone gates

Follow-up verification: nine geographic core/helper tests pass, including a direct pinned ExifTool check of zero and southern/western EXIF references. Bound startup/migration tests and all 27 catalog tests pass, including actual migration exits. The full workspace attempt reached 457 passes, 10 failures and 15 ignored cases: generated M4 metadata/replacement fixtures reject macOS extended attributes/resource forks. The unchanged origin/main metadata test independently reproduces that rejection on this host. No preservation gate or assertion was weakened.

The initial native GPS run exposed the pinned helper's JSONQ numeric strings; strict schema-15 parsing and a direct helper regression cover them. A same-version schema-14 view change temporarily blocked the running development catalog; a separate schema-15 migration and exact historical bound-startup checks fix that case without resetting Keychain/catalog authority. Native test attempts also exposed concurrent relinking of the shared debug binary and an overly broad strict filmstrip selector. The map harness now stages a separately named executable within the Cargo resource layout and narrows the selector. Copying it outside the Cargo resource layout initially caused Tauri's resource resolver to fail; the corrected sibling staging is exercised by the passing run.

Native map/platform and network evidence, keyboard/screen-reader review, M5 metadata interoperability (including conflict-safe XMP export), and full per-format clean-machine capability evidence remain required. This is the first M5 implementation slice, not an all-M5 certificate or an M4 production unlock. Stop before M6.

The subsequent [catalog metadata increment](M5_METADATA.md) extends the same native harness and advances the current schema to 16. Earlier counts and schema-15 evidence above remain historical checkpoint results.

## Final M5 continuation

[M5 validation](M5_VALIDATION.md) records the schema-17 export/tag implementation and final local evidence. Fresh final graphics acceptance is unverified on the currently occluded host: requestAnimationFrame is suspended while the local module worker responds. Separate native metadata acceptance uses the real photo-list alternative. Production scheduling and worker remain unchanged. No external service or CSP exception is introduced. Earlier passing runs retain their original build scope; the final native result is recorded separately.
