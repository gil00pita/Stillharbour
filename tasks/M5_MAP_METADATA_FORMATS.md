# Stillharbour - M5 - Map, metadata completeness and format gates

## Objective

Add geographic exploration that works with the same local catalog and complete the photo-management interoperability requirements.

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
