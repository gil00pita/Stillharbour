# Stillharbour - Library features and cloud-import addendum

Specification v1.2 | 2026-10-26 | Proposed behavior, not implemented functionality

Read PRESERVATION_CONTRACT.md first. This extension never relaxes that contract. The owner requested integrity reports, recently added browsing, a basic non-destructive mobile editor, large iCloud/Google Photos imports, database-file management, and photo/video tags. All six are included below. Desktop remains Windows/macOS first; a mobile editor is a separate, explicitly gated follow-on, not an implied existing mobile application.

References [LF01] through [LF12] resolve in LIBRARY_FEATURES_SOURCES.md. Proposed product rules are distinguished from documented provider behavior. See LIBRARY_FEATURES_DATA_MODEL.md for entities and IPC, and LIBRARY_FEATURES_TESTS.md for required, unexecuted scenarios.

## 1. Priority and placement

| Capability | Product placement | Delivery |
| --- | --- | --- |
| Integrity report | Utilities > Integrity, plus affected-item badges | Core safeguard; N1 extends M1/M3 |
| Recently added | Photos > Recently added; shared gallery/list/map filters | N1, after persistent asset identity |
| Save/open catalogs | File > New/Open/Save catalog copy; Settings > Catalog | Core durability; N2 extends M1/M5 |
| Photo/video tags | Details inspector, batch selection toolbar, Tags view | N2 extends M5 |
| Basic editor | Preview > Basic editor toggle | N3 desktop; N5 future mobile |
| Large-library imports | Import > Folder / Apple Photos & iCloud / Google Photos | N4, after verified-copy and journal gates |

Do not duplicate existing modules when integrating the N tasks. They are bounded extension work slices, not a replacement for M0-M6. Optional importer/editor/mobile work does not bypass the original release gates. Integrity reporting, catalog durability, and metadata provenance strengthen existing safety features.

## 2. Integrity report

### 2.1 Inventory and content are separate dimensions

Show report scope, root identity, scan completion/coverage, exclusions, bytes read, verification method, and timestamps. A report does not repair files. Use separate inventory, content, media-decode, and backup states; an offline file can also have a historical mismatch finding.

| Inventory state | Meaning | Permitted next step |
| --- | --- | --- |
| Untracked | An eligible file was observed inside an authorized, scanned root but has no accepted catalog file entry | Preview adding to catalog; preserve bytes and path |
| Present | Known entry observed at the expected identity/path | Inspect or verify |
| Missing | A known entry is absent after successful access and authoritative enumeration of the relevant online path | Locate/relink, compare a recovery copy, or preview restore |
| Root offline | Expected volume/share unavailable | Reconnect; never mass-mark missing |
| Inaccessible / incomplete | Permission, I/O, placeholder, or interrupted enumeration prevents a conclusion | Explain and rerun affected scope |
| Excluded / out of scope | Explicit policy omitted the entry | Show the reason separately; never call it verified |

A normal auto-index scan can accept eligible new entries, while an audit scan deliberately holds untracked observations for review. Neither mode writes into originals. Full-protection audits inventory hidden/unrecognized regular files and companions, independently of gallery visibility. Root or path failures must preserve earlier inventory; empty error results are not successful listings.

| Content state | Meaning |
| --- | --- |
| No baseline | No complete accepted digest exists for this revision |
| Verified against baseline | A complete current read matched the specified historical digest |
| Checksum mismatch | Complete bytes differ from the accepted baseline |
| Changed during reading | Concurrent change was detected; no stable comparison conclusion |
| Read failed / not checked | No valid current evidence; show specific reason |

Checksum mismatch means changed bytes, not proof of damage. Never silently accept a new hash, label a changed image corrupt, or overwrite a backup with the suspect version. Decode health remains a separate check. Establishing a first baseline proves what was read then, not that an image was always healthy or camera-original.

### 2.2 Baseline and review rules

Use complete versioned BLAKE3-256 or SHA-256 evidence with byte length, algorithm, observed revision, verification time and provenance. Retain historical baselines in the catalog and catalog backups. Cached size/time equality is not a fresh verification. Changing hash algorithms creates new evidence; it does not compare incompatible digests or erase history.

Allowed resolutions: Recheck; Locate/relink; Compare with verified backup; Restore to a new folder; and Accept as intentional change. The last action is a reviewed catalog-only resolution that creates a new baseline revision, keeps old evidence and records the reason. It does not create recovery evidence, declare the old backup corrupt, or authorize replacement/cleanup. Filesystem relinks require identity/content evidence, not just matching names or drive letters.

Show root, relative path, size, last-seen time, baseline/observed digest and algorithm, finding, verification coverage, known backup versions and recommended action. Do not hide errors after acknowledgement. Bulk actions operate on a reviewed frozen set and cannot skip per-item preconditions.

Export reports as JSON and optionally UTF-8 CSV. Report export is an explicit new-file operation outside scanned originals by default. Support redacted paths/GPS; neutralize spreadsheet-formula prefixes in CSV. Do not automatically open links or executable content from filenames. Never log private paths or hashes to telemetry by default.

### 2.3 Background behavior

Use existing cancellable jobs and I/O limits for inventory, hashing and read-back. Automatic verification schedules are opt-in and describe desktop sleep/offline behavior. A partial run reports the verified subset and remaining scope. The app may update its local catalog/cache during a read-only audit; it must not deliberately change source bytes, names or metadata. OS-maintained access times are a separate limitation, not a false zero-filesystem-change promise.

## 3. Recently added

Use the label Recently added rather than Uploaded for local-first libraries. Sort by the first committed addition of an asset to this catalog, descending, with a stable sequence/ID tie-breaker and keyset pagination. Store this independently from capture date, filesystem dates, provider upload date and last-seen time.

For a copy import, commit addition only after publication and copy verification succeed. For a referenced library, first cataloguing can precede full hashing; display integrity evidence separately. An item can be visible as Importing before publication, but must not look fully imported or protected.

Preserve first-added time across rescans, rename/relink, companion discovery, catalog restore and ordinary reimport of the same recognized source object/revision. Add later import activity to history. Without a reliable provider identity or unambiguous match, do not silently merge assets by filename or global content hash; independent copies remain independently represented. New distinct files are genuinely new entries even if visually similar.

Initial indexing is explicitly labeled Initial library scan. Migrated catalogs without historical first-added evidence use unknown/estimated dates with provenance, not fabricated upload times. If a system clock moves backwards, preserve UTC observations plus monotonic catalog sequence for stable ordering and explain clock anomalies.

Provide Added today, Last 7 days, Last 30 days, date range, import batch, provider and root filters. Calendar windows use the selected display timezone without rewriting saved instants. Gallery and text list retain the same query/selection. Opening a 2010 photo imported today shows it in Recently added today and in the 2010 capture-date timeline.

## 4. Basic non-destructive editor

### 4.1 Interface and scope

Add a Basic editor toggle inside image preview. The first editor includes crop (free and common aspect ratios), 90-degree rotation, horizontal/vertical mirror, before/after, undo/redo and Revert to original. Free-angle straighten, exposure/color edits, video editing and RAW development are not part of this increment.

Save edits stores a recipe revision in the catalog. It never rewrites the original image, changes EXIF orientation in place or runs lossless-JPEG rewriting against the source. Export edited copy renders a separate derivative through the safe-copy executor, with a reviewed filename/format/metadata policy and no-overwrite publication. Backup of the original remains byte-identical; catalog backup includes recipes and revisions, and any exported derivatives are separately inventoried.

### 4.2 Recipe semantics

A recipe references an asset ID and an exact source-component revision/digest, not a mutable pathname alone. Retain immutable recipe history with parent revision, operation order, coordinate-system version and renderer/color-policy version. Save uses optimistic concurrency: stale editors must resolve conflicts, not silently overwrite another revision.

Canonical geometry is explicit: decode and honor the stored EXIF orientation once; rotate in 90-degree clockwise steps; mirror in the rotated coordinate space; then crop in normalized coordinates of that rotated/mirrored full image. Crop values must be finite and inside bounds. Record the basis dimensions and compute a minimum one-pixel output. Transform the crop correctly when users change rotation; do not rely on incidental UI operation order.

Never use preview-resolution pixels as an original-quality export. Handle alpha and color profiles deliberately. If only an embedded RAW preview or cloud thumbnail exists, permit a clearly labeled preview edit but require the original/tested decoder for full-resolution export; otherwise disable that export with a reason. Preserve RAW/JPEG/Live Photo bundles; editing a still does not implicitly rewrite the video or another component.

If source bytes change, retain the old recipe and flag revision mismatch. Rebase requires an explicit reviewed decision. No auto-rebase from a same-named file. Edited views must not enter exact-duplicate hashing; hashes are over stored source bytes. Caches include source revision, recipe digest, output size and renderer version.

### 4.3 Mobile follow-on

The user explicitly requested editing directly on mobile. Keep the recipe contract platform-neutral and plan N5 for touch editing on iOS/Android after native storage, permissions, codecs and device tests. Desktop first does not mean mobile is silently dropped, nor does a responsive page establish mobile support.

Start mobile as a local import/edit/export workflow with the same source-preserving rules. Sharing catalog files is not a concurrent database synchronization protocol. Desktop/mobile library sync, pairing, remote access and account infrastructure require a separate ADR and approval; do not invent a server or let phones write to a shared SQLite file.

## 5. Large-library import engine (shared)

Workflow: Choose provider/export > Discover scope and versions > Inspect fidelity and omissions > Preview destination > Copy/download to staging > Verify > Publish > Reconcile report. Default destination is the base library, with user-reviewed organization rules. Sources and provider libraries remain untouched.

Create a durable import session and per-item ledger with source identity, archive identity, resource role, metadata provenance, transfer state, verified local digest, target and error. Handle pause/cancel/restart at item boundaries; support in-item range resume only when the adapter can prove unchanged content and correct range behavior. Otherwise restart just that item into new staging. Never equate an HTTP success code or archive CRC with full source-identity proof.

Reuse global read/write/network pacing and bounded queues. Count staging, history and decode work against appropriate owned-I/O budgets. Disclose when provider/OS download behavior is outside the application's speed control. Resume never means unlimited retries; quota/permission/expired session/disk-full/identity errors have relevant remedies.

Preserve exported metadata and unsupported files as provenance/artifacts, even if they are not displayed as media. Keep originals, provider-edited renditions and motion/RAW companions separately. Preserve available albums, descriptions, favorites, dates, locations and tags with provenance; unsupported or absent fields are reported, not fabricated. Names and timestamps alone cannot establish identity. Cross-archive duplicates retain every logical membership; physical reuse needs fresh equality and a manifest decision, never deletion of source archives.

Define separate import claims:
- Local copy verified: read-back matches the acquired source/export bytes.
- Provider-export fidelity: originals/renditions, metadata and components obtained through the selected route.
- Scope completeness: supplied archive parts and accessible selected resources are accounted for.
- Whole-cloud completeness: only possible with defensible provider enumeration/export evidence; otherwise explicitly unknown.

A verified download cannot prove equality to a camera original that the provider did not expose. Local archive hashes establish transport/storage baselines, not upstream historical authenticity.

Archive processing is sandboxed and bounded. Reject traversal, absolute paths, drive/UNC paths, alternate-stream names, symlink/hard-link escapes and target aliases; handle case/Unicode collisions without overwrite. Bound expanded bytes, entry count, nesting, compression ratio and parser memory. Never execute archive scripts/configs or fetch URLs from metadata. Unsupported/encrypted/corrupt archives remain untouched and produce actionable diagnostics. Identical member names in different parts use unique session identities; never extract every part over the same path indiscriminately.

Completion reports count all supplied entries, verified files, reused verified bytes, retained variants, unmatched metadata, unsupported resources, missing parts and failed/skipped items. Unknown expected counts remain unknown. Report Import complete for supplied scope rather than All cloud photos backed up unless the stronger claim is established. Never delete or offer automatic cloud cleanup after import.

## 6. Google Photos

### 6.1 Primary route: Google Takeout archives

Use user-supplied Google Takeout ZIP/TGZ parts or an extracted export folder as the large-library route. Google documents separate JSON for additional photo metadata and multi-archive exports [LF01]. Build a format-detecting, fixture-tested parser; no assumption that every export has one fixed filename/JSON schema.

Build a manifest across every supplied part before final association. Preserve the JSON raw bytes and associate conservatively using available source IDs, relative paths, titles and supporting metadata. Ambiguous or unmatched JSON remains preserved and visible for review; do not guess by stripping filename suffixes. Parse metadata into the catalog without writing it back into originals. Keep conflicting capture time/location values with provenance.

Google now documents scheduled Photos exports for new and updated data when Photos is the only selected product [LF01]. Accept repeated and incremental export batches: add or version changed resources, preserve prior membership/provenance, and never interpret absence in a later batch as deletion. Do not claim automatic retrieval of exports; the initial route starts with user-provided downloads. Provider-export scheduling restrictions still apply.

Validate supplied archive completeness as far as manifests/counts allow; request another part when known missing. Unknown completeness is explicit. Reimport is idempotent for recognized archive members/revisions; new versions never overwrite a local original or an earlier provider version.

### 6.2 Optional route: Google Photos Picker

Use Picker only for explicit user-selected imports, behind a registered/approved OAuth integration. The Library API is restricted to app-created content following the March 2025 changes; it is not a full-library enumeration API for this app [LF02].

Provider limits are material: the documented image download parameter omits location metadata; the video download parameter returns a high-quality transcoded rendition; download URLs expire and require the correct authorization [LF03]. Show these warnings before approval and store rendition/fidelity flags per file. Do not advertise Picker as a bit-exact original archive, a complete album migration or a complete GPS import.

Use the system authorization browser and narrow read scope. Keep credentials in OS-backed storage, never in catalog exports or logs. Expired/revoked sessions preserve verified local work and require supported reauthorization/reselection. Refresh URLs through supported APIs; never persist temporary URLs as permanent asset identity or authorize arbitrary metadata URLs with bearer tokens.

Do not assume the generic Google Data Portability API grants Google Photos access: the scope list checked on 2026-10-26 did not list a Google Photos library scope [LF12]. Recheck official capabilities before a later ADR, without confusing Maps/Street View media scopes with Photos. Service-to-service transfer partnerships are not an established public desktop importer contract.

## 7. Apple Photos and iCloud

### 7.1 Original-preserving export workflow

Guide macOS users to download originals and export unmodified originals, optionally with IPTC/XMP. Apple documents separate still/video exports for Live Photos and XMP metadata export [LF04, LF05]. Import that export into a separate ordinary library directory through the shared engine. Original and edited renditions, when both supplied, remain separate variants. Do not crawl/write undocumented internals of a .photoslibrary package as a substitute for an official adapter.

On Windows, support copying locally hydrated iCloud for Windows files into a separate library folder after checking actual readable content, not placeholder appearance. The route's original-versus-rendered fidelity must be verified/labeled; a local readable file alone is not proof of unmodified original. Pinning an iCloud item is not independent backup: Apple notes deletion can propagate to pinned files and recommends a copy elsewhere [LF06]. Never rename, organize, deduplicate or quarantine inside the provider's managed folder. Flag a proposed primary-library root inside a cloud-sync tree and prevent treating it as an independent backup.

Browser exports are a guided fallback, not a scalable unattended full-library API. Apple documents a selection limit of 1,000 items at a time for iCloud.com [LF04]. Preserve available metadata, pairs and supplied export manifests; report missing components instead of silently importing only the still image.

### 7.2 Later native macOS adapter

Investigate PhotoKit with explicit user authorization: PHAssetResourceManager exposes asset resources including original and edited variants [LF07]. Access is to the authorized local Photos library with OS-mediated cloud resource retrieval, not a universal cross-platform iCloud HTTP library API.

Enumerate authorized assets/resources, request resource data into staging, handle network-needed and limited/denied/revoked access, and preserve pair relationships. Request the minimum platform permission that permits reading; do not imply a read-only OS authorization category exists where the OS names it differently. The adapter must make no Photos write/delete/change calls. No Apple password collection, cookie scraping or undocumented private endpoints.

Native permission/signing, export fidelity, cloud hydration and cancellation tests on real macOS fixtures are mandatory before this adapter is advertised. Missing resources remain visible and unresolved; they never authorize deleting provider content.

## 8. Save and open database/catalog files

### 8.1 User model

Use the term Catalog for the database. Provide New catalog, Open catalog, Recent catalogs, Save catalog copy, Catalog information, Validate catalog and Relink library roots. Auto-save committed catalog changes; Save catalog copy creates a point-in-time portable snapshot. The proposed catalog filename example is Family.stillharbour: a documented, versioned SQLite application file, not a proprietary container for originals.

Display prominently: This catalog contains organization data and file references, not your original photos/videos. It includes tags/albums, added dates, edit recipes, baseline history, import provenance and references to backup/operation evidence. Original files and immutable backup repositories need their own protection. Large thumbnails/previews are separate regenerable caches by default; optional cache bundles are not part of the initial single-file export.

Do not include passwords, OAuth tokens, live download URLs or hardware-bound permission tokens in portable snapshots. Catalogs may contain private paths, GPS and tags; warn before sharing/export. Backup encryption is separate from SQLite format. Do not claim a catalog is encrypted unless an implemented, tested encryption layer exists.

### 8.2 Consistent saving and opening

Create a consistent SQLite snapshot using the Online Backup API into an exclusive new local staging file, validate it, close it, and publish with collision-safe semantics [LF08]. Never use a casual file copy of an active WAL database, overwrite the current catalog from a save dialog, or discard a required WAL to make the file look standalone. Prepare an independently openable snapshot with no dependence on live journal companions. Bound/retry busy snapshots without weakening durability.

Default active catalogs to a local, non-cloud-synchronized directory. Do not allow a live writable catalog on SMB/NFS or a cloud-synced working folder; import a validated local working copy instead. SQLite documents that WAL does not work over network filesystems [LF09]. Export closed snapshots to a chosen backup target using the safe-copy engine. Detect known unsafe folders and disclose detection limits; ask users about provider-managed paths when uncertain rather than claiming universal detection.

Open untrusted catalog files through a restricted validation path: application marker/schema version, SQLite integrity and foreign-key checks, allowed objects/migrations, resource limits, no extension loading, trusted_schema=OFF, no side-effect SQL functions or scripts from the catalog [LF10]. A valid SQLite structure is not authorization to act on its recorded filesystem roots.

Before opening a different catalog, pause jobs at a safe journaled boundary and reconcile unfinished operations. If this cannot be done safely, block switching with a reason; do not strand an active writer. Serialize catalog writes through the database actor. A second app instance uses a coherent read-only mode or declines the open; it cannot steal a lock. Freshly imported catalogs have saved schedules and mutating plans disabled until roots, identities, permissions and current file revisions are revalidated.

Treat opening from another machine, restoring an older snapshot, or opening a duplicate catalog UUID as a separate activation branch. Retain original provenance, assign a new local activation identity, invalidate stale approvals and do not replay historical operations. Missing roots remain offline until explicit mapping; relink changes references, not files. Forking must not let two active catalogs claim the same unreviewed job/schedule authority.

### 8.3 Recovery and migrations

Before migration, create and validate a retained snapshot of the old catalog. Migrate a staged copy or transactionally under a tested strategy, run integrity/foreign-key checks and publish only on success. On crash/disk-full, keep the old working catalog and journal recoverable. Unknown future schemas are not opened writable; present a clear compatible-version requirement.

Saving a catalog while a filesystem job is active records a database snapshot, not an atomic snapshot of all photos. Mark captured in-flight operations historical/untrusted on reopen and reconcile against storage before allowing action. A catalog restore does not prove that referenced backups are currently available or unchanged.

Include a catalog snapshot and immutable source metadata/recipe provenance in backup jobs. Keep catalog version history rather than silently replacing the sole good copy. A report may show Catalog saved at ... separately from Photo files protected through ....

Require a supported, patched SQLite runtime and record the actual bundled version. In particular, upstream documents the WAL-reset corruption fix in 3.51.3 and later, with specified older backports [LF09]. Verify inclusion of that fix and current advisories rather than blindly accepting an OS/library default. This is a gate, not a claim that a tested runtime has been selected here.

## 9. Tags for photos and videos

Provide manual tags, hierarchical categories, batch add/remove, autocomplete, rename/merge previews, undo and filters combining tags with dates, roots, backup or integrity status. Photos and videos use the same asset-level tag model even when preview playback/decoding is unavailable. Do not require re-encoding a video just to tag it. Tags apply to an asset and its declared related components without changing stored file bytes; distinct variants/copies are not silently merged globally.

Store default edits in the catalog only. Import EXIF/IPTC/XMP keywords as provenance-aware observations. Distinguish Imported metadata from Your catalog tags. Removing an imported keyword creates a catalog override/tombstone so rescanning does not resurrect it; the original embedded value remains recorded, not overwritten. Preserve Unicode display labels and use a documented normalization policy for comparisons; never conflate different labels silently.

Support explicit Export metadata sidecars, with XMP for interoperable fields and a versioned manifest for app-specific data. XMP sidecars are a documented metadata route [LF11], but different applications/containers do not necessarily interpret every field or video sidecar identically. Publish a tested compatibility matrix rather than promising universal portability. Never encode the app's crop recipe as another editor's schema unless round-trip fidelity is actually implemented/tested.

Sidecar export is a real filesystem write: preview the association, names and changes; avoid basename ambiguity such as IMG_001.jpg/IMG_001.raw sharing one unexpected XMP; preserve existing sidecars and unknown namespaces, use retained-version conflict handling for any approved replacement, and honor read-only roots. Prefer new sidecars in a chosen export directory for initial implementation. Do not overwrite a third-party edit history to add keywords. Future embedding writes need separate explicit approval and the preservation recovery gate; they are not the default tag-save action.

Deleting a tag or album changes catalog organization only. No tag operation deletes media, moves folders or alters backup-retention policy. Tags/recipes/baselines must survive catalog backup/restore even when originals are offline.

## 10. Cross-cutting security, errors and release gates

Assets at risk are original bytes/companions, catalog annotations/recipes/baselines, provider credentials, private location data and backup evidence. Trust boundaries include renderer-to-Rust IPC, provider responses, archives/media decoders, user-supplied catalogs and filesystem/network adapters. The Rust backend enforces permission, revision, containment, budget and publication checks; UI disabling alone is insufficient.

Use structured errors with preserved underlying cause, affected item IDs, safe human explanation, recoverability and relevant actions. Examples: ARCHIVE_PART_MISSING, ARCHIVE_UNSAFE_ENTRY, PROVIDER_RESOURCE_UNAVAILABLE, PROVIDER_AUTH_EXPIRED, CATALOG_SCHEMA_NEWER, CATALOG_IN_USE, CATALOG_UNSAFE_LOCATION, BASELINE_MISMATCH and EDIT_SOURCE_REVISION_CHANGED. No generic override bypasses safety checks. Logs and reports redact secrets and private metadata by default.

Implement parser/resource-limit tests, archive and catalog fuzz fixtures, native permission/placeholder cases, interrupted save/import recovery, deterministic recipe tests, and round-trip catalog recovery. Use generated/disposable data, not a real cloud/photo account, without precise authorization. Keep provider adapters unavailable until authentication, policy and real platform capability gates pass; a mock importer is not a shipped importer.

All requirements here are specifications. No application build, provider authorization, native import, database migration, media export, mobile test or safety test has been performed by producing this package.
