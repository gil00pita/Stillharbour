# Offline place search

Owner-approved scope, 2026-11-05: GPS-based city/country search working offline.
[ADR-0089](decisions/ADR-0089-offline-place-search.md) records the architecture and
compatibility decisions. M6 and original-file preservation gates remain intact.

Choose **Places** beside Photos/Map search, enter at least two characters, then
choose the city/country with the correct region/country label. At most twenty
suggestions appear; add a region or country to narrow broad matches. Names and
alternate names are case/accent insensitive. Cities match **within 25 km of the
listed centre**, not municipal boundaries. Countries/territories match approximate
source polygons rather than bounding boxes, including separate islands and holes.
Coastlines, borders, disputed-area treatment and GPS itself can be inaccurate.

The snapshot contains 34,204 city records and 258 country/territory geometries.
Small towns, streets, addresses and every local name are not covered. Only valid
indexed GPS can match; no location is inferred from a filename, tag or preview.
Offline roots retain observations without proving original availability. Older
observations may need an explicitly authorized rescan; browsing never starts one.

Other filters, including date text, still apply. Shared native membership governs
pages, counts, buckets, navigation, map clusters and Select all results. Selecting
a place focuses the map; manual viewport filtering remains separate. **Clear
place filter** preserves the other filters and current viewport. Closing without
choosing changes nothing. Saved searches retain known versioned IDs; unavailable
IDs produce an error instead of silently broadening the query.

Lookup/membership run locally using compressed data embedded in the native app.
No geocoding account, runtime download, network permission or source rewrite is
added. Credits, modifications, license links and coverage appear in the picker
and [NOTICE](../assets/places/NOTICE.md). Sources: [GeoNames](https://download.geonames.org/export/dump/readme.txt),
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/),
[Natural Earth](https://www.naturalearthdata.com/about/terms-of-use/).

[The manifest](../assets/places/manifest.json) pins source and derived hashes.
Normal builds consume committed derived files. GeoNames URLs rotate daily and
cannot be assumed to reproduce the retained snapshot. Regenerate only from exact
retained inputs with `python3 scripts/prepare-place-data.py --sources .context/place-sources`;
the developer script verifies input pins, reads one fixed ZIP member, and has no
downloader or archive-tree extraction. Review the derived/manifest diff rather
than changing hashes to hide a failed source check.

Focused checks: `node scripts/cargo.mjs test --locked -p stillharbour-core --lib library::places::tests`
and the PlaceSearch/Map component suites. Follow with standard full/native checks
and restore production assets before `npm run release:check`, which verifies data
and notice pins. Generated native EXIF fixtures supply London, zero, opposite
hemisphere, antimeridian, invalid and missing GPS, with independent source audits.
Only actual results in the implementation log establish platform evidence; no
Windows, large-catalog performance or trusted-release acceptance is inferred.
