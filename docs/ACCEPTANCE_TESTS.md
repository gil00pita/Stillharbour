# Verify the implemented application

This page maps current behavior to its existing verification entry points. It is
not a record of fresh test execution. Run the checks appropriate to a change and
record actual outcomes in [IMPLEMENTATION_LOG.md](IMPLEMENTATION_LOG.md).

## Behavior and test entry points

| Area                                | Preserve and verify                                                                                                                                   | Existing entry points                                                                                                                                                                                   |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Library registration                | Native identities, nonoverlap, filter validation, source preservation, preparation and explicit job controls                                          | [Foundation native suite](../tests/desktop/foundation.e2e.mjs), [core library](../crates/core/src/library)                                                                                              |
| Browsing and dates                  | Bounded feeds, 6 px virtual/rendered gutters, anchor restoration, chronology, offline states and image-free Text list                                 | [AssetBrowser](../src/components/AssetBrowser/AssetBrowser.test.tsx), [timeline native suite](../tests/desktop/timeline-navigation.e2e.mjs), Foundation                                                 |
| Appearance and controls             | System default/saved appearance, portal surfaces, shared sizing, embedded amount/unit selectors, visible icons and both window widths                 | [Appearance](../tests/desktop/appearance.e2e.mjs), [controls](../tests/desktop/control-consistency.e2e.mjs), [file sizes](../tests/desktop/file-size-fields.e2e.mjs)                                    |
| Display and filters                 | Independent drafts, Apply/Cancel/Escape, focus return, query/selection retention and compact slider access                                            | [Display panel](../src/components/DisplayPanel/DisplayPanel.test.tsx), [display native suite](../tests/desktop/display-menu.e2e.mjs), [FilterPanel](../src/components/FilterPanel/FilterPanel.test.tsx) |
| Selection and favourites            | Exact rest/reveal states, keyboard reachability, confirmed catalog saves, errors, busy guards, aligned nonoverlapping controls and no original writes | [AssetTile](../src/components/AssetTile/AssetTile.test.tsx), [page chrome](../tests/desktop/page-chrome.e2e.mjs), [appearance](../tests/desktop/appearance.e2e.mjs)                                     |
| Albums and tags                     | Empty/explicit picker mode, atomic creation, persistent catalog membership, duplicate-name errors and retained pending saves                          | [AlbumEditor](../src/components/AlbumEditor/AlbumEditor.test.tsx), [Albums native suite](../tests/desktop/albums.e2e.mjs)                                                                               |
| Viewer                              | Guarded original/preview routes, image limits, bounded retry, controls, filtered navigation and return focus/scroll                                   | [PhotoViewer](../src/components/PhotoViewer/PhotoViewer.test.tsx), [viewer controls](../tests/desktop/viewer-controls.e2e.mjs), [format suite](../tests/formats/native.e2e.mjs)                         |
| Map and search                      | Indexed GPS/zero values, bundled places, bounded clusters, explicit photo loading/handoff, real marker geometry and explicit-only NASA requests       | [MapScreen](../src/components/MapScreen/MapScreen.test.tsx), [map suite](../tests/desktop/map.e2e.mjs), [map handoff](../tests/desktop/map-photos.e2e.mjs)                                              |
| Catalog metadata exports            | Source/sidecar retention, new-output publication, exact selection/version review, conflicts and retained tag history                                  | [Metadata native suite](../tests/desktop/metadata-export.e2e.mjs), [core export adapter](../crates/core/src/library/metadata_export)                                                                    |
| Utilities and Jobs                  | Actual job IDs/states, bounded results/history, partial/offline coverage, baseline acknowledgement and explicit failed-run retry                      | [InspectionScreen](../src/components/InspectionScreen/InspectionScreen.test.tsx), [utility native suite](../tests/desktop/utilities-scans.e2e.mjs), [Jobs](../tests/desktop/jobs-page.e2e.mjs)          |
| Structure and selected-photo plans  | Paged whole-catalog review, examples labelled illustrative, metadata/missing-date rules, presets, revalidation, simulation and closed execution gates | [Structure native suite](../tests/desktop/structure-formatter.e2e.mjs), [OrganizationDestination](../src/components/OrganizationDestination/OrganizationDestination.test.tsx), Foundation               |
| Recovery and experimental workflows | Generated-scope authority, durable records, byte/version/neighbor survival and separate process/transport evidence                                    | [Catalog suite](../wdio.catalog.conf.mjs), [native CI](../.github/workflows/check.yml), [transport CI](../.github/workflows/m4-transports.yml)                                                          |
| Production exclusion                | Empty/missing inventory refusal, default feature graph, reviewed helper resources and no fixture/test server in production                            | [Release policy regressions](../scripts/tests/release-policy.test.mjs), [production preflight](../scripts/release-check.mjs)                                                                            |

## Scope of a pass

- Windows page-chrome selection checks deliberately establish native cursor hover and
  then require exact hidden controls after moving both cursor and focus away.
  Synthetic DOM pointer events alone do not establish CSS hover coverage.
  The [readiness regressions](../tests/desktop/photo-controls-ready.test.mjs)
  cover partial discovery/previews from invalid generated media and reject missing,
  unfinished, failed or mismatched preparation jobs. A settled partial fixture is
  not a successful integrity check.
- Component/browser cases do not establish native OS behavior. Instrumented native
  cases do not certify installed/signed packages or authorize real-library mutations.
- Generated source manifests and native fixture audits must remain unchanged.
  Do not run discovery, mutation, backup or restore tests on a personal library.
- Exact/normalized/property duplicate evidence, checksum changes, preview readiness,
  source accessibility and recoverable backup versions are different claims.
- Report failed attempts, skipped cases, missing platform coverage and reused
  evidence explicitly. A pending or canceled CI job is not a pass.
- Keep the default execution gates closed until the applicable full preservation,
  native recovery and release obligations pass. See [Release status](RELEASE_STATUS.md).

## Preserved acceptance obligations

The original [AC backlog](changelog/2026-11-09/docs/ACCEPTANCE_TESTS.md),
[PS failure catalog](changelog/2026-11-09/docs/PRESERVATION_TESTS.md), extension
tests and milestone evidence remain in the [archive](changelog/README.md).
Their relocation does not remove required tests, turn an unimplemented plan into
a feature or mark a release gate complete.
