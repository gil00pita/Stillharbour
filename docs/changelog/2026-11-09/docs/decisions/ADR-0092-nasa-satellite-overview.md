> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0092: Optional NASA satellite overview

Date: 2026-11-05. Status: implemented; executed checks and platform limits belong in IMPLEMENTATION_LOG.md.

## Decision and owner scope

The owner requests an icon-only whole-world control and Map/Satellite view choices, then explicitly selects **NASA satellite overview — no API key; limited detail, internet required**. Retain bundled Natural Earth as the default Map view and add NASA GIBS Blue Marble as an optional online raster layer in the same MapLibre instance. This supersedes ADR-0081's network-free-only provider decision solely for the explicit Satellite choice. It does not add a detailed street/aerial service, account, credentials or an offline imagery download feature.

## Provider, rights and accuracy

Use the public NASA GIBS EPSG:3857 WMTS layer BlueMarble_ShadedRelief_Bathymetry with GoogleMapsCompatible_Level8, 256-pixel JPEG tiles and zoom levels 0–8. The fixed identifier, URL prefix, matrix, resolution bound and review date live in assets/maps/satellite-provider.json. The provider's current capabilities document confirms this static, non-temporal layer and tile template. It is a composite world overview, not live imagery or street-level detail; zooming beyond the source's level 8 magnifies existing imagery. Both the choice tooltip and active-view notice disclose limited detail and internet use.

NASA promotes open use of GIBS and requests acknowledgment. Visible credit identifies NASA GIBS and Blue Marble; the source attribution also carries that credit. Natural Earth attribution returns in Map view. Relevant primary sources:

- [NASA GIBS use guidance and acknowledgments](https://nasa-gibs.github.io/gibs-api-docs/).
- [NASA WMTS access and time dimensions](https://nasa-gibs.github.io/gibs-api-docs/access-basics/).
- [NASA projection/resolution tables](https://nasa-gibs.github.io/gibs-api-docs/map-library-usage/).
- [Live EPSG:3857 capabilities](https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/1.0.0/WMTSCapabilities.xml).
- [MapLibre raster source API](https://maplibre.org/maplibre-gl-js/docs/API/classes/RasterTileSource/).

The review receives HTTP 200 image/jpeg for the level-zero tile with a 256×256 decoded image, wildcard CORS and a provider cache lifetime of three days. This confirms endpoint behavior on the review date; it is not an availability guarantee.

## Privacy and resource limits

No satellite source exists until the person selects Satellite. This choice sends tile coordinates for viewed map areas and the normal connection IP to NASA, never photo bytes, filenames, catalog/root IDs, EXIF records or credentials. The map disclosure and choice tooltip explain that viewed areas are requested from NASA. Selection is session-only; a remounted map starts offline again. Offline place search remains bundled and unchanged.

The request transformer accepts only the compiled HTTPS prefix and numeric z/y/x tile coordinates within the provider matrix; reject other URLs, query strings and requests while Map is selected. Cross-origin credentials are excluded through same-origin mode and referrers are suppressed. Use fetch no-store for imagery, with no persistent application tile cache, bulk downloader or prefetch. MapLibre limits concurrent image requests to four and its inactive tile cache to 64; active viewport tiles, browser/GPU memory and external service throughput are separate limits. Returning to Map removes the raster layer and source, aborts owned in-flight source requests and revokes new NASA requests.

Production and development CSP allow only the exact NASA tile URL prefix in image/connect directives. Remote scripts, styles, glyphs, sprites, TileJSON, geocoding and arbitrary hosts remain blocked. Release preflight pins the complete CSP policy and rejects broad or extra origins. This grants no remote Tauri command capability or filesystem access.

## Behavior and failure handling

Switching layers preserves the camera, markers, photo selection and click-loaded filmstrip. A named 36px globe IconButton replaces the visible Whole world text, retaining tooltip, keyboard focus and reset behavior. The Map/Satellite buttons expose pressed state and remain reachable at the minimum desktop width.

A satellite source error removes that source and returns to the bundled Map view with an explicit connection/retry message; photos remain available. Retrying requires another explicit Satellite click. Graphics failure remains a separate photo-list fallback. No automatic photo fetch, original-file operation, schema, identity or dependency change follows.

## Validation boundary

Component tests cover named icon/reset behavior, opt-in requests, layer removal, camera preservation, failure/retry and strict URL boundaries. Node tests cover exact production/development CSP. Generated native acceptance retains existing map/source audits and adds real NASA tile retrieval, minimum-width controls, identical canvas/marker nodes, no extra photo-page load, no requests in Map mode and an explicitly simulated network failure followed by a real retry. Record actual native/network results and limitations; main-WebView interception does not establish OS/worker traffic completeness. Existing preservation, clean-machine, signing and unexecuted target-platform gates remain open.
