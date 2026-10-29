# Stillharbour - Market review and product recommendations

As of 2026-10-20. Evidence is limited to official product documentation and project repositories. Source IDs resolve in SOURCES.md. We have not installed and benchmarked these products as part of this review.

## 1. Positioning

Proposed promise: "Your photos stay in your folders. Organize them with a preview, find redundant copies with evidence, and keep recoverable backups."

The best opportunity is not to assemble the longest feature checklist. It is to combine an approachable timeline/map browser with safe, transparent filesystem operations. The user should always understand the difference between a photo, a physical file location, a related-file stack, and a backup copy.

## 2. Relevant products and lessons

| Product/reference | Documented strength | Proposed lesson for Photo Library |
| --- | --- | --- |
| digiKam [R04] | Rich metadata, multiple collection views, similarity search and configurable reference-image choices | Include folder-aware keeper rules, list/gallery/map views, ratings/tags, and searchable metadata; simplify the presentation |
| Immich [R05, R36] | Timeline/map, stacks, external libraries, storage templates, smart search and duplicate review | Use a photo-first chronological experience, shared filters, photo stacks, and a review workspace; do not require its server-style architecture |
| PhotoPrism [R06, R35] | Related-file stacks and distinct exact-duplicate handling | Model RAW/JPEG, Live Photo components and sidecars as relationships; do not destroy unique companions while deduplicating |
| Mylio [R08] | External-drive/Vault workflows and library protection visibility | Keep unavailable-root states understandable and surface protection status; add real retained snapshots rather than conflating sync with backup |
| Photo Mechanic [R07] | Ingest, fast previews, metadata variables, batch renaming and multi-destination workflow | Make import incremental and transparent; prioritize useful previews, keyboard culling and dependable metadata handling |
| dupeGuru [R32] | Filename/content and picture comparison with reference-directory concepts | Offer protected reference folders and clear matching modes, not just an unexplained similarity score |
| Czkawka [R33] | Duplicate, similar-image, broken-file and large-file tools with a Rust core | Consolidate practical diagnostics under Utilities; evaluate reusable code only after component-level license and fit review |
| SpaceSniffer [R09] | Treemap-based space exploration with filters and reports | Combine a zoomable treemap with an accessible table and honest size/reclaimability metrics |
| Adobe metadata workflow [R29] | Catalog metadata, XMP interchange and companion editing data | Preserve sidecar relationships and unknown metadata; make external metadata writes explicit and conflict-aware |

These are patterns to learn from, not instructions to copy competitor source, branding, layouts pixel-for-pixel, or proprietary sample images. The strongest proposed distinction is a reviewed transaction plan shared by organizer, import, dedupe, quarantine, and restore.

## 3. Additions worth prioritizing

### First release: safety and interoperability

**Confirmed related-file bundles.** RAW/JPEG pairs, Live Photo still/motion components, and XMP/ACR/JSON companions need first-class relationships. An organizer that renames only the visible JPEG can damage a workflow even when the JPEG itself survives.

**Recoverable actions and clear evidence.** Add durable operation history, quarantine, undo-as-a-new-plan, source-change detection, and preferred/protected keeper folders. Explain every suggested removal in human-readable language.

**Backup coverage and real restoration.** Show whether each current file version is included in a snapshot, whether data was actually read back, and when a restore was tested. Include the catalog, not just image bytes. Mylio's documented deletion-propagation warning makes the distinction between redundancy and historical recovery particularly important. [R08]

**Metadata that can leave the app.** Favorites, ratings, tags, virtual albums, saved searches, and catalog export are useful early. XMP writes should be explicit, with unknown fields preserved. Avoid a new opaque organization format as the only place the user's work exists.

**Offline-root continuity.** Keep cached browsing when a disk is unplugged. Separate "not connected" from "deleted" and make relinking robust to changing drive letters.

**Integrity history.** Separate unsupported decoding, decode failure, and a changed checksum baseline. Add repeatable verification rather than a misleading single "healthy" badge.

### After the core is reliable

**Perceptual duplicate review and burst culling.** Similarity search can find recompressed or resized versions, but it creates false-positive and quality-selection problems. Start with explainable scores, manual review and stacks, not automatic removal.

**Optional local semantic search and people grouping.** These features are visible in products such as Immich and digiKam, but require model packaging, privacy decisions and resource budgets. Keep them opt-in and independent of destructive decisions. [R04, R05]

**Export privacy profiles.** A later share/export action can remove GPS or private tags from a new copy while leaving the original intact. This is more useful than a global metadata-stripping switch.

**Smart organization assistance.** Saved searches such as "not backed up," "missing capture time," "missing location," "suspected corrupt," and "over 100 MB" can be useful before introducing an LLM or a complex search language.

**GPX geotagging and date correction.** These can help older libraries, but should start as reversible catalog overrides and require explicit metadata export.

### Deliberately defer

A full Lightroom-style develop module, cloud accounts/sharing, mobile auto-upload, collaborative catalogs, and two-way device synchronization would each expand the product substantially. They are not prerequisites for a strong Windows/macOS folder-based organizer.

## 4. Why Rust, but not all-Rust

Recommend Rust for discovery, hashing, comparison, filesystem operations, database coordination, and job control. It supports efficient native code and memory/thread-safety benefits. That does not establish that a Rust application will outrun every C++, Go, C#, or Python application with optimized native libraries. [R01]

For this workload, unnecessary reads, decode cost, network latency, cache design, database queries, and frontend virtualization can dominate. Photo Mechanic's published engineering notes discuss disk I/O as a principal bottleneck in its workload. Measure the real library before optimizing language-level details. [R07]

Recommend Tauri plus React/TypeScript rather than requiring a pure-Rust UI. It separates the Rust core from a WebView interface and suits the proposed interactive map and gallery. The tradeoff is testing and maintaining two implementation languages plus platform WebView behavior. [R02]

An Electron, Qt, Flutter, or Avalonia implementation could be viable for a team with deeper expertise there. This recommendation is a project-fit judgment, not a universal framework benchmark. Do not replace a capable team's existing stack solely on a generic "fastest language" claim.

## 5. Build versus integrate

Build the catalog model, the photo-focused UX, the template planner, the safety policy, the operation journal, keeper decisions, and verification reporting. These are the product-specific behaviors.

Integrate established metadata/decoder and backup components behind narrow adapters. ExifTool, image, LibRaw, and potentially libvips address different media tasks; none should be presumed to cover every requested format automatically. [R18-R21]

Use restic for encrypted snapshots and rclone for the required transports. restic documents its rclone backend; rclone documents SMB, SFTP and FTP/FTPS. The app remains responsible for credential storage, strict host verification, complete scope reporting, restore UX and integration tests. [R10-R13]

This choice makes backups more than a file-copy button, but it means a repository is not an ordinary mirrored folder and recovery needs the repository key. Offer a readable-copy/export feature separately rather than disguise one as the other.

## 6. Unsettled decisions with explicit gates

The owner has selected Stillharbour as the product name. Public brand clearance, minimum macOS version, Intel Mac release support, codec distribution path, production map provider, licensing/commercial model and background-agent behavior require implementation or owner decisions. Defaults and gates are in DECISIONS.md. The core architecture does not need to wait for public brand clearance or an AI provider.

No delivery-time or cost estimate is supplied because team experience, real library size, codec scope and release distribution are not known. Milestones are ordered by risk and dependencies, not by invented calendar estimates.
