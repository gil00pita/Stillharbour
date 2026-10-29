# Stillharbour - Primary-source research register

Checked: 2026-10-20. Official documentation and project-owned repositories were preferred. This is a documented feature/capability review, not hands-on benchmarking, user research interviews, an exhaustive market survey, or a software-license opinion. Versions and packaging availability must be checked again when implementation starts.

Source IDs are referenced in docs/CODEX_SPEC.md and RESEARCH.md. URLs are written as literal code so they remain easy to reuse outside this package. Product statements below summarize the narrow evidence used; proposed features and engineering decisions remain our recommendations.

| ID | Source and specific evidence | URL |
| --- | --- | --- |
| R01 | Rust project: native performance, memory efficiency, ownership and safety rationale | `https://rust-lang.org/` |
| R02 | Tauri process model: Rust core, platform WebViews, process separation and security boundaries | `https://v2.tauri.app/concept/process-model/` |
| R03 | Tauri capabilities: permission scopes, custom-command defaults, and limitations | `https://v2.tauri.app/security/capabilities/` |
| R04 | digiKam: collections, metadata, map/table/timeline views, similarity search and reference-image selection | `https://www.digikam.org/about/features/` and `https://docs.digikam.org/en/left_sidebar/similarity_view.html` |
| R05 | Immich: timeline/map, external libraries, stacks, search, and duplicate-review behavior | `https://immich.app/features` and `https://docs.immich.app/features/duplicates-utility/` |
| R06 | PhotoPrism: RAW/JPEG, Live Photo and sidecar stacking | `https://docs.photoprism.app/user-guide/organize/stacks/` |
| R07 | Photo Mechanic: ingest/metadata/rename workflow; engineering note on disk read/write bottlenecks | `https://home.camerabits.com/tour-photo-mechanic/` and `https://home.camerabits.com/whats-new-in-photo-mechanic/` |
| R08 | Mylio: external-drive/Vault protection and the warning that synchronized deletion also reaches Vaults | `https://manual.mylio.com/24.3/en/topic/backup-and-protect` and `https://manual.mylio.com/24.3/en/topic/add-an-external-drive` |
| R09 | SpaceSniffer: treemap exploration, size/type/date filtering, text report export | `https://www.uderzo.it/main_products/space_sniffer/index.html` |
| R10 | restic: repository preparation, password dependency, local/SFTP and rclone backend integration | `https://restic.readthedocs.io/en/stable/030_preparing_a_new_repo.html` |
| R11 | rclone SFTP: credentials and host-key verification configuration | `https://rclone.org/sftp/` |
| R12 | rclone SMB: direct SMB transport and UNC-path alternative on Windows | `https://rclone.org/smb/` |
| R13 | rclone FTP: FTP, implicit/explicit FTPS, passive-mode and transport limitations | `https://rclone.org/ftp/` and `https://github.com/rclone/rclone/blob/master/docs/content/ftp.md` |
| R14 | SQLite WAL: concurrency and network-filesystem restrictions; live WAL state | `https://sqlite.org/wal.html` |
| R15 | SQLite backup API: consistent live-database snapshots | `https://sqlite.org/backup.html` |
| R16 | BLAKE3 official implementations: cryptographic hashing, SIMD/parallel implementation, legacy-hash comparison caveats | `https://github.com/BLAKE3-team/BLAKE3` |
| R17 | restic verification: snapshot listing and distinction between structural check, read-data, and sampled read-data | `https://restic.readthedocs.io/en/stable/045_working_with_repos.html` |
| R18 | ExifTool application documentation: read/write modes, supported metadata, command-line/persistent protocols | `https://exiftool.org/exiftool_pod.html` |
| R19 | image crate: native Rust decoding and common-format operations | `https://docs.rs/image/latest/image/` |
| R20 | LibRaw: RAW data, decoding metadata, embedded previews, and licensing choices | `https://www.libraw.org/about` |
| R21 | libvips: demand-driven image processing, formats and LGPL licensing notice | `https://www.libvips.org/` |
| R22 | Rust filesystem Metadata: creation/birth and modification timestamps, unsupported fields | `https://doc.rust-lang.org/std/fs/struct.Metadata.html` |
| R23 | Rust Unix metadata extensions: distinct filesystem timestamp fields | `https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html` |
| R24 | Microsoft: naming/path conventions, reserved names, case and length caveats | `https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file` |
| R25 | MapLibre: built-in point clustering and custom HTML cluster markers | `https://maplibre.org/maplibre-gl-js/docs/examples/create-and-style-clusters/` and `https://maplibre.org/maplibre-gl-js/docs/examples/display-html-clusters-with-custom-properties/` |
| R26 | OSM Foundation: standard raster tile attribution, caching, privacy and no bulk/offline prefetch policy | `https://operations.osmfoundation.org/policies/tiles/` |
| R27 | Tauri testing: current WebdriverIO embedded-driver route and direct tauri-driver platform limitations | `https://v2.tauri.app/develop/tests/webdriver/` |
| R28 | ExifTool ID3: supported audio/media container types; relevance differs from ordinary photo metadata | `https://exiftool.org/TagNames/ID3.html` |
| R29 | Adobe: catalog/XMP metadata interchange and additional ACR companions | `https://helpx.adobe.com/lightroom-classic/desktop/organize-photos-in-lightroom-classic/create-xmp-acr-files.html` and `https://helpx.adobe.com/ae_en/lightroom-classic/desktop/organize-photos-in-lightroom-classic/advanced-metadata-actions.html` |
| R30 | Tauri macOS signing and notarization | `https://v2.tauri.app/distribute/sign/macos/` |
| R31 | Tauri Windows signing | `https://v2.tauri.app/distribute/sign/windows/` |
| R32 | dupeGuru: filename/content modes, picture similarity, reference-directory protections | `https://dupeguru.voltaicideas.net/` |
| R33 | Czkawka official repository: duplicates, similar images, broken/big files; reusable Rust core and component licenses | `https://github.com/qarmin/czkawka` |
| R34 | OpenAI Codex workflow guidance: AGENTS.md and optional phased plan/build files | `https://developers.openai.com/cookbook/examples/codex/iterating-development-workflows-with-codex` |
| R35 | PhotoPrism duplicate documentation: exact-file indexing/import behavior versus related-file stacks | `https://docs.photoprism.app/user-guide/library/duplicates/` |
| R36 | Immich storage templates: metadata variables, collision suffixing, template migration | `https://docs.immich.app/administration/storage-template/` |
| R37 | rclone check: remote hash availability, size-only checking, and download comparison | `https://rclone.org/commands/rclone_check/` |

## Limits and interpretation

The comparison does not establish which competitor is fastest, cheapest, safest, most popular, or best for every user. No benchmark numbers, pricing, market share, or development-duration estimates are inferred from feature lists. A product's absence from this review is not evidence that it lacks a feature.

Some documentation is continuously generated or updated and can contain stale adjacent text. The specification avoids pinning version numbers from marketing pages. M0 must validate actual dependencies, APIs, binary targets, and licenses.

The reference screenshots were supplied by the user. They are design inputs, not independent evidence of an app's implementation or performance. Do not redistribute them as application sample content.

## P0 performance provenance checks — 2026-10-28

The performance proposal references S01–S17, but their source register was not supplied in this repository. Those historical/proposal citations are not benchmark evidence. P0 used the following primary references and the actual linked runtime; existing R IDs remain unchanged.

| ID | Primary evidence | URL |
| --- | --- | --- |
| SP01 | SQLite WAL-reset fix starts at3.51.3, with documented backports; WAL is local-host storage. | [SQLite WAL](https://sqlite.org/wal.html) |
| SP02 | Current checked release3.54.1 and its exact source ID; 32-bit overflow fix does not affect this arm64 host. | [3.54.1 release](https://sqlite.org/releaselog/3_54_1.html), [release history](https://sqlite.org/changes.html) |
| SP03 | Upstream advisory table; evaluate listed fixed versions and prerequisites, not a blanket vulnerability-free claim. Table observed update2026-08-01. | [SQLite advisories](https://sqlite.org/cves.html) |
| SP04 | Connection status API, checked return codes and non-resetting counters. Pager misses and WAL/database page writes are not physical-device measurements. | [db_status](https://sqlite.org/c3ref/db_status.html), [status definitions](https://sqlite.org/c3ref/c_dbstatus_options.html) |

The installed macOS27 SDK's sys/resource.h rusage_info_v2 and libproc.h proc_pid_rusage declarations supplied the primary ABI definition for the read-only process sampler. P0 records its runtime coverage and omissions. No dependency upgrade, source-format support or security clearance is inferred from this research.

Additional diagnostic API checks: [React Profiler](https://react.dev/reference/react/Profiler) documents production profiling's opt-in build/overhead and actualDuration as render work. The pinned react-dom package exports its profiling createRoot implementation. [WebdriverIO setTimeout](https://webdriver.io/docs/api/browser/setTimeout) covers ordinary script timeouts; inspection of the pinned Tauri service showed its separate30-second DirectEvalClient default. P0 therefore uses a bounded harness promise/poll for fixture setup rather than treating a timeout as an application browsing result.

P1 primary API checks (2026-10-28): [INDEXED BY](https://www.sqlite.org/lang_indexedby.html) is a plan requirement, not an optimizer hint; it is reserved here for the reproduced simple-inventory regression after predicate/ID-only alternatives still sorted. [total_changes64](https://www.sqlite.org/c3ref/total_changes.html) includes this connection's DML/triggers but not other connections; [PRAGMA data_version](https://www.sqlite.org/pragma.html#pragma_data_version) supplies the same-connection external-commit observation. They invalidate derived summaries conservatively; neither replaces authoritative catalog validation or selection membership.

## M5 native preview slice — primary references, 2026-10-28

- [Apple ImageIO](https://developer.apple.com/documentation/imageio) and [image source/thumbnail options](https://developer.apple.com/library/archive/documentation/GraphicsImaging/Conceptual/ImageIOGuide/imageio_source/ikpg_source.html): bounded thumbnail creation, properties, orientation and source options. Installed SDK ImageIO/ColorSync/libproc headers supply the actual linked API/ABI definitions, including primary-image index and profile verification. [FORMAT_SUPPORT.md](FORMAT_SUPPORT.md) records runtime versions; API documentation alone does not prove a camera passes.
- [raw.pixls.us](https://raw.pixls.us/) and its [official repository index](https://raw.pixls.us/json/getrepository.php): selected camera sample provenance and individual CC0 records. Exact URLs/SHA-256/license pins are in [camera-fixtures.json](../scripts/formats/camera-fixtures.json). No private photo fixture is used.
- The locked cc 1.10.0 package manifest declares MIT OR Apache-2.0. This change adds only the owned C bridge build edge to that existing version. System framework usage does not establish codec/patent clearance or clean-machine portability; release review remains required.

## Gallery refinement reference — 2026-10-28

[Antin Harasymiv, Building the Google Photos Web UI, Google Design](https://medium.com/google-design/google-photos-45b714dfbed1) is the owner's primary engineering reference for alternative row-break evaluation and bounded rendering/resize work. Stillharbour reuses its own layout/virtualizer; the article's product-specific timings and frame-rate claims are not Stillharbour measurements. The implemented subset and independent validation are in [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md).

Windows CI portability checks (2026-10-28): [Rust slice::as_chunks](https://doc.rust-lang.org/std/primitive.slice.html#method.as_chunks) defines fixed-size arrays plus a remainder (stable since1.88, below the pinned1.99 toolchain), used to reject incomplete UTF-16 units without truncation. [Microsoft file/path naming rules](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file) exclude quotes and control characters from ordinary Win32 filenames; generated physical Windows tests therefore retain Unicode/percent/leading-hyphen coverage without inventing support for illegal names. Hostile Unix-name tests remain intact.
