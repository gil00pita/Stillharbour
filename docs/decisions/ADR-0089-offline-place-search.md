# ADR-0089 — Offline GPS city and country search

Date: 2026-11-05  
Status: owner-authorized; actual validation in the implementation log

The owner explicitly chooses offline GPS-based city/country search. Reuse the
catalog's validated GPS/spatial tables. A Places picker returns at most twenty
labelled suggestions from pinned GeoNames cities15000 and Natural Earth data;
same-name cities remain separate choices with region/country labels. Cities
match within 25 km of the listed centre, not municipal boundaries. Countries and
territories use approximate source polygons, retaining holes and separate islands.
This adds no online service, automatic reverse geocoding or source metadata edit.

Pure lookup/geometry stays separate from the catalog adapter. Conservative R-tree
candidates precede exact distance/polygon tests through one deterministic,
DIRECTONLY SQLite function on owned connections. The backend validates a known,
versioned place ID; renderer labels/bounds cannot redefine membership. Shared SQL
governs pages, counts, buckets, neighbors, maps and frozen selections, intersecting
other filters. Missing/invalid/unextracted GPS cannot match. Offline observations
remain indexed without proving accessibility or protection.

AssetQuery adds optional placeId, omitted when absent. Schema 17, catalog/root
UUIDs, migration fingerprints and root authority remain intact. Existing queries
keep their meaning; new saved searches retain the place ID. Older binaries can
reject newer query fields rather than silently drop their scope. Unknown/removed
IDs fail explicitly. Future dataset/policy changes require compatibility review.

Reuse locked flate2 1.1.10 as a direct dependency for compressed assets and enable
rusqlite 0.41.0's functions feature; no package versions or native helpers change.
Decode each file once, bounded to 32 MB, with at most 40,000 cities and 300 country
records. Initialization runs on the existing blocking request path. No full
library coordinate array reaches the renderer. Large-catalog country performance
remains unmeasured; this does not close P4/M6.

The shared Chakra v3 picker owns its popover recipe. Add only its missing spacing,
size/font/stacking tokens to the existing defaultBaseConfig system and regenerate
types. Preserve loading, no-match, failure/retry, stale-response disposal,
choose/clear/cancel, focus and coverage/source disclosures.

Committed compressed data builds without downloads. Exact provenance, hashes and
notices live in assets/places. GeoNames daily URLs are mutable: regeneration needs
the retained pinned inputs or a reviewed update, never an automatic pin change.
Natural Earth uses the existing map-source commit. No network capability,
credentials, original-file authority or release gate changes.

See [place search](../PLACE_SEARCH.md), [GeoNames documentation](https://download.geonames.org/export/dump/readme.txt),
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) and
[Natural Earth terms](https://www.naturalearthdata.com/about/terms-of-use/).
