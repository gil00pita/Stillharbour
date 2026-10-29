# Live library updates

Owner request: 2026-10-29. Keep Gallery and Folders current when files are added, changed, moved or removed outside Stillharbour, and retain a way to inspect changes. This is a read-only M1 extension after P2; it does not start P3 or enable original-file operations.

## Behavior

The desktop backend starts monitoring registered roots independently of the visible page. Native notifications invalidate observations; the existing scanner validates actual contents. Debounce for 750ms of quiet, with a five-second maximum burst delay. Periodic eligibility restarts after an observed terminal scan, so a long scan alone does not cause a continuous rescan loop. Relinking rebinds the watcher to the new native path/identity. One scan runs at a time and shares the existing two-media-worker admission. Events arriving during a scan remain pending for another reconciliation. Do not accumulate filenames/event queues: each root has an atomic signal and one pending reconciliation state.

Reconcile on startup, reconnect and native events. Up to 64 roots receive native watchers. Healthy watchers also receive a safety reconciliation every five minutes; unavailable/failed watchers and additional roots use 30-second periodic checks. These intervals are eligibility times: an active scan, slow filesystem, permissions or large library can delay completion. Network filesystems can omit notifications. There is no promise of an instantaneous or atomic filesystem view.

Ignore read/access events and events confined to the app-owned data/cache directory to avoid feedback loops. A native overflow/rescan flag still requests reconciliation. Event paths never authorize traversal. Reopen and validate root identity/volume through the existing native containment rules, including a final root check before inferring absence. A replaced root requires relinking. An offline/inaccessible root retains its last known inventory and cached browsing.

Only complete per-directory enumeration can mark an item missing. Failed, canceled, excluded and inaccessible scopes retain observations. Confirmed missing files and directory branches leave normal Gallery/Folders results, without deleting their catalog observations or originals. Gallery's explicit missing-item filter remains available. A removed folder already open shows its missing state and keeps Up/Back navigation available.

Both views refresh from catalog revision changes through the existing status polling. Gallery refresh includes new photos ahead of its previous first item; at the beginning it stays at the beginning, while older bounded windows keep stable anchors. Folder pages refresh in place and recover with one bounded first-page request if a removed last entry strands an old cursor.

## Change history

Open the header Jobs dropdown, then Library changes. It shows Added, Moved, Updated, Removed and Reappeared observations with the library label, observed time and previous/current paths. Monitoring status distinguishes native watching, periodic checks and unavailable roots. History loads only when opened, in 25-item pages with Older/Latest controls and explicit loading/error/retry states. Native reads accept at most 100 items and reject oversized, malformed or newer record formats explicitly.

The first successful/partial completed scan establishes a baseline; subsequent observed differences are recorded. History covers observed states, not every transient filesystem event between scans. An Updated observation means the stat/revision evidence changed, not that corruption or content equality was proved. Same-root moves preserve asset IDs/notes only with the existing unambiguous native-file/birth evidence. Cross-root moves, hard-link ambiguity and unsupported identities are not merged by name or content hash. Folder path changes appear through moved files and added/removed folder observations; empty folders do not gain invented stable identities.

Versioned records commit with inventory changes in the existing job_events table, under the reserved library-change:<root UUID> namespace. Schema 2 and catalog identity remain unchanged; there is no catalog migration or history side file. Native path locators retain lossless bytes alongside escaped display strings, and are never filesystem execution inputs. The journal is local catalog data containing filenames/paths, not telemetry, recovery evidence or an integrity baseline. No history is uploaded or automatically expired. Explicit library forgetting retains this history, including previous paths; the removal review discloses that distinction. Clearing thumbnails leaves history intact.

## Dependencies and safety

[ADR-0013](decisions/ADR-0013-live-library-reconciliation.md) records the trade-offs. Add exact notify 8.2.1 and its locked platform adapters; do not upgrade existing packages. The [notify documentation](https://docs.rs/notify/8.2.1/notify/) describes native backend/network limitations; its [configuration](https://docs.rs/notify/8.2.1/notify/struct.Config.html) and [rescan hints](https://docs.rs/notify/8.2.1/notify/struct.Event.html) inform the integration. Windows traversal remains unavailable pending its existing native safety gate, regardless of notification-library platform coverage.

Preserve originals, sidecars, source identities, offline distinctions, scan settings, existing cache bounds, frozen selection semantics and preservation gates. This feature does not rename, remove, clean up, auto-convert or write metadata into a source. Change history does not authorize undo or restoring a file. No new mutation permission, filesystem renderer API, decoder, state framework or resource scheduler is introduced.

## Executable validation

```sh
npm run check
npm run desktop:test:build
npm run desktop:test
# Run the live flow in its own app session when diagnosing combined-driver stalls:
npm run desktop:test -- --spec tests/desktop/live-library.e2e.mjs
npm run desktop:build
npm run release:check
```

The native test creates its shared `.context/live-library` parent on demand before exclusively creating a UUID run directory. A fresh CI checkout requires no preexisting local evidence. Ownership markers, independent manifests and restoration checks remain mandatory; recursive parent creation does not permit reusing an existing UUID run.

Focused native backend tests:

```sh
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes change_journal
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes native_notifications
```

[Backend fixtures](../crates/core/tests/library_flow.rs) simulate external changes only inside generated/owned roots. Independent Python manifests surround read-only reconciliation phases; restored files, paths and native identities match their original fixture inventory. The test’s external rename/restore changes parent-directory mtimes; retain those raw differences explicitly instead of claiming the whole tree metadata is unchanged. Exact full manifests still match around each read-only phase. [Native UI validation](../tests/desktop/live-library.e2e.mjs) checks real Gallery/Folders refresh without a manual scan and a real change-history dropdown. Its temporary external moves are reversed in finally; source manifests and screenshots remain under .context. Test-only monitor controls are in the existing desktop-test diagnostic command and excluded from production.

Follow the isolated-test-binary procedure in [DEVELOPMENT.md](DEVELOPMENT.md) if a live watcher shares target/debug. Never scan or modify a real user library as a development fixture. Actual check outcomes and unavailable environments are recorded in IMPLEMENTATION_LOG.md; a browser component test does not establish native watcher support.

For JavaScript-driven source checks, pass --lossless-stat to [manifest.py](../scripts/performance/manifest.py). It encodes 64-bit size/device/inode/mtime fields as decimal strings, preserving values through JSON.parse. The default legacy output remains unchanged for existing probes. Native Rust/Python comparisons do not pass through JavaScript number conversion. Older Node-parsed traces have rounded timestamp precision; do not call those nanosecond-exact metadata evidence. The [round-trip regression](../scripts/performance/test_manifest.py) exercises an owned fixture and the actual Node JSON consumer.

## Recorded outcome — 2026-10-29

| Check                         | Actual outcome                                                                                                                                                                                                                                                                                                                                                                                              |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Full repository check         | Passed: 157 frontend / 73 Rust tests (51 unit, 20 library, 2 media), formatting, strict types, lint, generated bindings, denied-warning Clippy and production frontend build.                                                                                                                                                                                                                               |
| Final diagnostic edits        | Final strict types/lint and both Python diagnostic tests pass. Lossless-stat transport has a real Node round-trip regression.                                                                                                                                                                                                                                                                               |
| Existing native desktop cases | The first run passes all 14 cases in 38.9s with its independent source inventory unchanged.                                                                                                                                                                                                                                                                                                                 |
| Final isolated live UI        | Passed in 16.7s on native macOS WKWebView; run 5ebd93f6-0b9c-4964-b0f3-c9e82f75a7bf. Gallery/Folders react to external moves/removal without manual Rescan, and history is visible at 1180×780 and 660×560. Read-only phase manifests match exactly with decimal-string stat transport; restored original files/paths/identities match, and two externally changed parent-directory mtimes remain recorded. |
| Combined native reruns        | Failed/interrupted: retained driver/window-rectangle stalls in existing cases before live monitoring starts. These are not passing evidence. The original new-spec fixture-reuse failure and overly broad final tree-mtime assertion are corrected separately. No physical-input or release presented-frame claim follows.                                                                                  |
| Release package               | Unsigned macOS .app builds 34.40MiB. Production test-route/capability/dependency exclusion and helper checksums pass. Read-only packaged SQLite remains 3.54.1 with the same source ID and userCatalogOpened=false; no catalog migration.                                                                                                                                                                   |
| Dependencies                  | Exactly seven additive locked notification packages; no removed/upgraded existing version/checksum. npm lock unchanged.                                                                                                                                                                                                                                                                                     |
| React Doctor                  | Final scoped scan 73/100, 11 warnings. Ten are incumbent; the additional feed callback finding is reviewed as an imperative viewport-anchor notification that writes refs, not parent-owned live state. No rule suppression is added. Native/component anchor regressions remain enabled.                                                                                                                   |

Final isolated native test binary SHA256 26f6ba714bb9f48be50ba4d3928d686016693c06c2cf60a18219e6f892e252f7. Packaged release executable SHA256 9cd7009ab20adee0089e5961d77afb2b8eb42d692c504e4e1fa2213b694ab66f; SQLite provenance run c041fe2d-7eb1-4405-a9d0-594080a82e50. Evidence is retained privately under .context/live-library, with native screenshots in .context/screenshots/live-library-changes-*.png. These are generated fixtures and native debug functional evidence, not benchmarks of a real user library.

Windows native traversal, real removable-volume/NAS/provider behavior, minimum OS/clean-machine/signing, physical input, production frame timing/full process memory and large full-media event-storm latency remain untested. Periodic and storm policy logic is unit-tested; native local notifications and offline retention are integration-tested on this Mac. No complete cross-platform/release performance pass or P3 implementation is claimed. Changes remain local for review; no commit/push.

Final verification passes 247 owned text/local-link checks and git diff --check. Of 82 retained manifest pairs, 79 match exactly at their recorded encoding; three external-move restoration pairs preserve files/paths/identities with only explicitly recorded parent-directory mtime differences. The final live read-only phases use lossless decimal-string stats and match exactly. Fresh inventories of all three retained P2 rendition trees also match. The npm lock is unchanged; Cargo contains only the seven documented additive packages with existing versions/checksums retained. All changes are saved locally; no commit or push.
