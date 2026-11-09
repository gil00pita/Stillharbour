# Format and platform support

Discovery, preview decoding, original viewing, metadata extraction and integrity
validation are separate capabilities. A recognized extension or cached thumbnail
does not establish full-media validity, original accessibility or universal
camera support. This page summarizes the implemented adapters and bounded evidence;
it does not certify a released package.

## Platform boundaries

| Platform                                                     | Implemented scope and limits                                                                                                                                       |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Apple Silicon macOS                                          | Native folder identity/scanning, metadata helpers, guarded originals and ImageIO/ColorSync preview adapters; bounded generated and selected-camera evidence exists |
| Windows local NTFS                                           | Native folder identity/scanning, baseline JPEG/PNG/WebP previews and supervised bounded EXIF/GPS metadata; no broader filesystem/codec claim                       |
| Intel macOS, network roots, other Windows filesystems, Linux | Not established supported product targets by existing evidence                                                                                                     |

Configured minimum macOS 13 does not prove acceptance on that OS. Development/CI
hosts do not establish clean-machine install, helper availability, signing,
notarization or every codec combination. See [release status](RELEASE_STATUS.md).

## Media paths

| Media                                        | Capability boundary                                                                                                             |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| JPEG, PNG, WebP                              | Rust baseline decoder and bounded previews; admitted originals can be served through the guarded original protocol              |
| HEIC/HEIF                                    | macOS native previews; detected still-image originals on compatible macOS WebKit within the original-route limits               |
| TIFF, GIF, BMP                               | Bounded macOS native preview evidence; no every-frame or multipage validation claim                                             |
| Selected RAW files                           | macOS preview evidence for pinned CR2, CR3, NEF, ARW, DNG and RAF samples; no full-resolution RAW developer or all-camera claim |
| Video extensions, including MP4              | Discovery/cataloging is not video preview or playback support                                                                   |
| Unrecognized, malformed or unsupported media | Originals remain retained; explicit metadata/preview failure states replace invented success                                    |

The direct-original route admits supported static media only within 32 MiB and
32 million pixels, with native root/identity/revision checks and explicit fallback.
Derived renditions are bounded and do not rewrite source bytes. Larger-image
JPEG/PNG/TIFF adapters and the tall-JPEG path have their own policy-specific
read/dimension/time/memory limits; do not substitute an older generic limit for
the running adapter's policy.

Deep integrity decoding is bounded to supported static PNG/JPEG/WebP. It does not
validate complete RAW payloads, every animation frame, all pages, HDR behavior or
every embedded metadata ecosystem. Normalized-pixel duplicate matching has its
declared static PNG8 policy and requires manual review.

## Evidence and reproduction

Pinned camera provenance and checksums live in
[camera-fixtures.json](../scripts/formats/camera-fixtures.json). The
[format runner](../scripts/formats/run.py), [native format suite](../tests/formats/native.e2e.mjs)
and [worker code](../crates/core/src/media) define executable coverage.
Use generated/pinned fixtures, never personal originals.

The [archived format report](changelog/2026-11-09/docs/FORMAT_SUPPORT.md) and
[implementation history](changelog/2026-11-09/docs/IMPLEMENTATION_LOG.md) retain
exact artifact hashes, source manifests, failures and per-camera results.
Those are historical results for their recorded inputs, not new-head passes.
