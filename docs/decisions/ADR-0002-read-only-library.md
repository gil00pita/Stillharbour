# ADR-0002: Read-only library, catalog identity and bounded previews

Date: 2026-10-27. Status: implemented locally on macOS Apple Silicon; platform and distribution gates remain open.

The owner authorised M1 after the M0 review. This milestone saves and reads selected folders, persists catalog annotations and provides chronological browsing. It adds no executor for original-file mutations. `require_original_mutation` continues to reject those operations regardless of root mode.

## Catalog and source authority

Extend the existing bounded SQLite actor instead of adding an ORM, second connection owner or renderer SQL permission. Schema 2 adds roots, file inventory, per-directory coverage, scan jobs/issues, annotations, frozen selection members and a preview-cache index. Transactional inventory triggers maintain root media counts, avoiding a full root recount for each visible preview request. Before migrating an existing schema-1 catalog, retain a SQLite-consistent, independently validated snapshot using the existing no-replace publication primitive. Preserve the catalog UUID. A newer/foreign schema is rejected.

Only the native picker can issue a root-selection ID. Rust retains its native path and directory identity; display strings never become execution paths. Root registration validates identity again, obtains the macOS volume UUID and rejects component overlap, identical directory identities and ancestor-identity overlap through case aliases (realpath alone preserves caller spelling on APFS). The initial implementation defaulted roots to referenced/read only. The owner’s 2026-10-27 scan-settings revision changes the new-folder default to Managed with a Read only switch; existing modes remain unchanged. Managed mode still records a future grant without enabling mutations. See DECISIONS.md for that approved default change. Relinking requires an explicit native selection and matching complete content samples; roots without samples require the original identity.

macOS traversal opens descendants relative to directory handles with no-follow semantics. Reopen reads through a freshly validated root, volume UUID and each native path component. Store native path bytes losslessly and keep lossy strings for display/search only. Links and special files are skipped. Cloud dataless and hidden flags are checked from directory-entry metadata before opening payloads. Application data is excluded by native directory identity as well as path; alternate case spelling cannot bypass that boundary. Protected packages are excluded. The Windows identity/traversal adapter remains unavailable: the interface must not advertise a successful Windows scan.

## Inventory and workers

Enumerate incrementally with depth 64, commit batches of at most 32 records and expose early results. One scan and at most two media jobs are admitted. A canceled scan retains partial records and never infers absence. Missing requires completed enumeration of the specific parent directory; incomplete, excluded, offline and inaccessible scopes retain history. Interrupted scans reconcile through a new read-only scan, never replay a file mutation. One-link, unique native-file/birth evidence can preserve an asset ID after an external rename; ambiguous hard links remain distinct locations.

Use the pinned ExifTool 13.55 Perl code as a bundled resource. Invoke explicit `/usr/bin/perl`, private module path, fixed config-disabled/read-only arguments, cleared environment and owned working directory. Supply bounded bytes over stdin rather than original filenames. JSONQ preserves numeric-looking metadata such as subseconds. Capture-local dates keep raw values and separate offsets; unknown offsets never become UTC. Full-content BLAKE3 samples are relink evidence, not an integrity baseline or proof of damage.

Use the app executable itself in an early `--media-worker-stdin` mode, before starting Tauri, for isolated JPEG/PNG/WebP decoding. No separate decoder sidecar is required. Both worker paths have 32 MiB input limits, 12-second supervision, bounded output and cancellation. Image dimensions/allocation are bounded; EXIF orientation is applied; ICC inputs are explicitly unsupported pending a tested color pipeline. Worker failures retain typed public errors. These restrictions are visible capability limits, not universal media support.

## Browsing and cache

Use indexed `(sort key, stable asset ID)` keysets, query-bound cursors and pages of at most 240 assets. Year navigation issues a direct indexed seek; it does not load preceding pages. Freeze Select all results membership in SQLite, including unloaded rows, before catalog-only batch annotations. The frontend keeps individual selections bounded at 500 IDs and page-history cursors bounded at 512. Dates and raw metadata remain plain text.

Adopt pinned `@tanstack/react-virtual` 3.14.23 (MIT) for visible rows. Gallery rows use bounded aspect ratios; grid cells use a shared square geometry; text list renders no image elements and submits no nonempty thumbnail batches. Preserve query, page, selection and first-visible anchor when switching view modes. Map, albums beyond tags, and file-operation utilities stay unavailable with an explanation.

Preview keys include asset ID, observed source revision, edge and processing policy. A bounded queue supersedes stale generations, shares the two-worker media budget and validates the source revision before/after work. Preview generations synchronise with Rust so renderer reloads do not depend on wall-clock time. Cache publication never replaces an unexpected file; existing bytes must match the generated PNG. Eviction removes only positively indexed cache keys, with a 256 MiB budget. The custom `thumbnail` protocol serves only known cached PNGs through validated IDs, never originals, filesystem paths or renderer-supplied executable locations. Cached browsing remains available offline.

TanStack positioning requires numeric inline styles. CSP permits inline **styles**, with scripts restricted to self and no `unsafe-eval`. React escapes filenames/metadata, navigation rejects external origins, and production has no shell/filesystem/SQL permission. Only two app commands are allowed. Debug-only native fixtures and WDIO instrumentation are compiled out of release builds and denied in the normal capability.

## Evidence and open gates

See [M1 validation](../M1_VALIDATION.md) for actual commands, performance and scenario coverage. System Perl on macOS 27.0.1 is tested locally; clean-machine execution, minimum macOS 13, Intel and Windows are unverified. Real removable-volume disconnects, cloud-provider placeholders, screen-reader review and signed/notarized release remain gates. M2 original-file work requires separate authorisation and its preservation/recovery prerequisites.

### Owner setup refinements, 2026-10-27

The sidebar reads the app version from the existing Rust-owned foundation status (additive IPC field), avoiding a second version literal or new renderer permission. The photo discovery preset is defined once in Rust, exported with bindings and reused for filename recognition; the UI shows dotted extension tags and submits lowercase undotted values. Native defaults for general callers remain compatible. Removing setup include/date controls clears those hidden constraints on save, without a catalog schema change. A natural-height Advanced form delegates scrolling to the existing main pane. Decoder capabilities are unchanged.
