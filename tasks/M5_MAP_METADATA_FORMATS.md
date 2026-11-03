# Stillharbour - M5 - Map, metadata completeness and format gates

## Objective

Add geographic exploration that works with the same local catalog and complete the photo-management interoperability requirements.

**Current gate, 2026-11-03: complete for the implemented offline map, catalog metadata and reviewed exports into new files.** [M5 closeout](../docs/M5_CLOSEOUT.md) verifies final PR #16's Windows/macOS Foundation and transport passes against the exact merged application tree, native preservation/output receipts and the acceptance mapping. Unverified packaged-format/platform combinations retain explicit labels under the gate below. Clean-machine release acceptance, broader accessibility and M4 production original-file authority remain separately gated. Stop before M6. The dated entries below retain their historical scope.

Readiness review, 2026-11-02: the owner requested a progress check and conditional M5 start. M4 PR #14 is merged and its final Windows/macOS Foundation and transport checks pass, but M4 preservation acceptance and production execution remain gated. [M5_READINESS.md](../docs/M5_READINESS.md) records the evidence, remaining blockers and prepared M5 scope. No new M5 implementation starts under this request; earlier explicitly approved format slices below remain intact.

Subsequent owner continuation, 2026-11-02: “continue with m5” explicitly authorizes M5 development. The first increment is read-only geographic browsing under [ADR-0081](../docs/decisions/ADR-0081-offline-geographic-browsing.md), with bundled offline Natural Earth geometry, catalog schema 15 with preserved schema-13/14 migration snapshots and bounded local spatial queries. [M5_MAP.md](../docs/M5_MAP.md) records behavior, commands, actual checks and remaining work. This supersedes the earlier development pause; it does not certify M4 or enable production file mutations. Remaining metadata interoperability and format/native gates stay part of M5. Stop before M6.


Further owner continuation, 2026-11-02: [ADR-0082](../docs/decisions/ADR-0082-catalog-metadata-organization.md) and [M5_METADATA.md](../docs/M5_METADATA.md) implement schema-16 catalog-only captions/rejects with immutable saved history, virtual albums, validated saved searches and explicit category relationships. Existing favorites/ratings/flat tags stay compatible. Portable catalog/XMP export, rename/merge/undo and remaining native/format/accessibility acceptance remain open. This is a reviewed increment, not M5 completion or M4 production authority.

## Prerequisites and references

M1 query contracts and date/GPS provenance; docs/CODEX_SPEC.md sections 7, 8, 15, 16; acceptance AC-074 through AC-076, AC-079 and AC-083.

## Work

Select and document a production map provider or self-hosted/offline data path, including attribution, privacy, cache limits and permitted offline behavior. Do not use a public demo endpoint as the final service.

Implement spatial queries, zoom/viewport clustering, thumbnail markers, click-through filmstrip, date/root filtering and show-in-library. Handle absent versus valid zero GPS, identical coordinates and antimeridian bounds. Keep payloads bounded and retain usability offline without unsupported offline tile downloads.

Complete virtual albums, hierarchical tags, favorites/ratings/rejects/captions, saved searches, metadata provenance and portable catalog export. Explicit XMP export/synchronization must preserve unknown data and detect external conflicts. Referenced roots remain read-only even when catalog annotations change.

Complete and publish format capability gates for packaged HEIC/HEIF, selected common RAW formats, TIFF and baseline formats. Validate orientation, color policy, bounded decode, sidecars, metadata and separate preview/full-validation support. Do not label a RAW supported merely because its extension was indexed.

## Gate

Map and metadata acceptance passes. Document real provider behavior and network privacy evidence. Test every claimed packaged format on clean target machines or mark it unverified. Do not add local AI, mobile/cloud synchronization or an editing module during this milestone. Stop before M6.

## Version 1.2 integration

N2 extends catalog files and photo/video tags, with catalog-only edits and protected sidecar export. N3 basic editing and N4 imports are separate approved extensions, not automatically authorized by M5. Read docs/LIBRARY_FEATURES_ADDENDUM.md.

## Approved partial format slice — 2026-10-28

The owner's HEIC/CR2/common-preview request authorises only read-only format previews. Implemented the bounded macOS native worker adapter, typed RAW hints, negative-policy retry and compatible M1 positive cache reuse; see [format capability/evidence](../docs/FORMAT_SUPPORT.md) and [ADR-0010](../docs/decisions/ADR-0010-native-format-previews.md). Local packaged tests pass the declared generated formats and six camera samples; native generated HEIC gallery/viewer passes independently. M5-F01–F05 have scoped local evidence. M5-F06/AC-083 cross-platform/clean-machine/full-validation gates remain open. Maps, provider selection, metadata export, other M5 work and P2–P4 are not authorised by this slice. Stop at this partial gate.

## Owner-approved rendition follow-on — 2026-10-28

The subsequent large JPEG/PNG/TIFF and separate-thumbnail-task request authorises the bounded P2 slice in [PROGRESSIVE_PREVIEWS.md](../docs/PROGRESSIVE_PREVIEWS.md), with the file-handle/job decision in [ADR-0011](../docs/decisions/ADR-0011-progressive-renditions.md). This extends preview generation only; the remaining M5 format/platform/map/metadata gates and broader P2–P4 work stay open.


## Owner-requested UI library adoption — 2026-11-02

The owner separately requested the latest Chakra UI using their supplied boilerplate as an example. [ADR-0083](../docs/decisions/ADR-0083-chakra-ui-design-system.md) records pinned Chakra/Emotion, shared product tokens/controls and preservation of native dialog/canvas behavior. [UI_FRAMEWORK.md](../docs/UI_FRAMEWORK.md) records checks. This presentation/library change does not advance catalog schema or close any remaining M5/M4/M6 gate.

## Owner-requested final implementation — 2026-11-02

“Finish M5” authorizes the remaining bounded milestone implementation. [ADR-0084](../docs/decisions/ADR-0084-reviewed-metadata-exports-and-tag-history.md) adds schema-17 exact tag review/merge/removal/undo, explicit keyword import overrides and new-output portable catalog/XMP export with native authority, retained plans, conflict detection and completion receipts. [M5 validation](../docs/M5_VALIDATION.md) is the current implementation/evidence record; earlier slice statements above remain historical.

Implementation is present; milestone acceptance is **not certified complete** while the recorded platform/manual/preservation checks remain unclosed. Formats without clean target-machine evidence remain explicitly unverified, as required by this gate. No production original-file mutation is enabled. Stop before M6.

## Acceptance repairs — 2026-11-02

The owner requests build/preservation repairs and completion of remaining checks. [M5 validation](../docs/M5_VALIDATION.md) records the resolved ten macOS preservation failures, native foreground graphics/keyboard/focus and reviewed-export checks, the accessible export label, and the separate Windows CI repairs. On 2026-11-03 the owner passes all three actual manual VoiceOver map/heading/Catalog-dialog checks; fresh native map/export audits preserve nine generated source/companion entries each. Windows Foundation and both transport jobs pass at `a2b3d10`; fresh corrected Foundation CI remains pending at `5541d8f`. Broader format/platform limits remain qualified. Original-file production gates remain disabled; stop before M6.

## Windows native metadata completion — 2026-11-02

ADR-0087 fills the dimensions-only Windows adapter gap with real bounded EXIF/GPS in the supervised worker, including reference signs and valid zero coordinates. Legacy rows refresh through authorized read-only rescans. Native M5 tests execute on both target OSes, and the default desktop harness retains its source/focus assertions while updating schema/directory/pointer-order assumptions. M5 validation tracks fresh CI, actual VoiceOver/manual evidence and explicit unsupported-format limits. No M6 or production original-file mutation is enabled.

## Current closeout — 2026-11-03

The owner passes all three bounded actual VoiceOver checks. Fresh native map/navigation and metadata/export tests pass with independent nine-entry unchanged-source audits. The complete local Foundation desktop retry passes 22/22 with three existing platform skips; its earlier two-source timeout and passing isolated retry remain recorded. Production frontend and release checks pass after native testing. Windows Foundation and both transports already pass at `a2b3d10`; formal closeout awaits the fresh corrected Foundation CI at `5541d8f`. Unsupported clean-machine/format/platform combinations remain explicitly unverified under the gate. Stop before M6.

## Resumed evidence review — 2026-11-03

The owner requests “Continue M5” after PR #15 merges. Final PR head `5a56a61` passes Foundation and transport CI on Windows/macOS and has the exact merged Git tree. Downloaded native receipts confirm unchanged generated sources and protected catalog/XMP outputs on both platforms. The earlier `5541d8f` Windows run fails an order-dependent global XMP count; a documentation-only passing rerun does not repair it. This continuation scopes that assertion to the exact reviewed export, adds native BLAKE3 output evidence and seven regressions, and retains the reviewed file list in receipts. [M5 validation](../docs/M5_VALIDATION.md) distinguishes prior CI, fresh checks and remaining qualified platform/format/accessibility gates. No M6 or production original-file authority is added.

Current continuation outcome: deterministic repeated-export native metadata passes with twelve verified artifacts and nine unchanged source/companion entries; frontend 482/482, strict checks, instrumentation Clippy and production build/resource checks pass. The first full workspace records 507 core passes/one catalog-recovery lock failure/fifteen opt-ins; isolated and exact-original-executable recovery retries pass, with no root-cause repair claimed. The final foreground combined native map run passes in 7.0s with every graphics/keyboard/focus and source/output assertion intact; earlier hidden-rendering and section-focus failures remain recorded. The final harness correction has no fresh Windows run. That check and qualified platform/format/accessibility gates remain open. This increment does not certify all M5 acceptance.

The final complete serial `npm run check` passes: 482 frontend tests, 508 core tests/zero failures/fifteen existing opt-ins, two desktop unit cases, all 29 integrations, zero doctests, strict checks and production build. Production resource checks pass afterward. This closes the local validation retry while retaining the earlier lock failure and the remaining Windows/platform acceptance boundaries above. Stop before M6.
