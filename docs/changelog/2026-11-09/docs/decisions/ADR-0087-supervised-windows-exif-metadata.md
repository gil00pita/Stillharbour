> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0087: Supervised Windows EXIF metadata

Date: 2026-11-02. Status: implemented; fresh native Windows evidence required.

## Context

At PR head 0d97e14, both transport jobs pass and all Windows core/preservation tests pass. The newly required Windows native M5 scenario fails with zero located photos: its existing `windows-basic-v1` adapter explicitly returns dimensions without EXIF/GPS. This is an implementation gap, not permission to invent fixture coordinates. Mac M5 map/export scenarios pass in Foundation CI at this head; unrelated default desktop failures expose stale schema expectations, absent screenshot folders and embedded-driver focus order.

## Decision

Use pinned [kamadak-exif 0.6.1](https://docs.rs/kamadak-exif/0.6.1/exif/) (BSD-2-Clause), with its single dependency mutate_once 0.1.2 (BSD-2-Clause), inside the existing supervised media subprocess. The new closed `--metadata-worker-stdin-v1` route accepts only bounded stdin bytes, never a supplied path, executable or shell command. Parent supervision retains the existing twelve-second deadline, cancellation, decoder memory monitoring, cleared environment and one-MiB stdout cap. Input remains at most 32 MiB and preview/decode bounds remain unchanged.

The worker produces `windows-exif-v2` observations for baseline-decodable media: oriented dimensions, bounded make/model/lens strings, observed original/digitized dates and offset, orientation and GPS. It reads EXIF strictly; malformed input is an explicit failure rather than partial success. Three positive rationals plus a recognized N/S/E/W reference are required for each signed coordinate; zero denominators and unsupported references fail. Missing references never imply a positive sign or zero. Out-of-range observed degrees remain invalid observations for the existing geography gate to reject. Text fields are bounded to 4096 bytes. Unknown EXIF fields are not copied into telemetry or logs.

On an explicit rescan, rows carrying the older dimensions-only policy receive a fresh authorized read instead of retaining a false appearance of metadata completeness. Revision-bound policy detection is read-only; ordinary successful metadata publication follows the existing catalog adapter. No catalog schema, UUID, root authorization or preservation-manifest policy changes. macOS continues to use the existing pinned ExifTool path.

## Scope and evidence

This does not add RAW/HEIC decoding, embedded-XMP/IPTC keyword completeness, full-media validation or broader format support on Windows. Unsupported formats retain their documented limitations. Sidecar XMP export/unknown-field preservation remains the separate reviewed adapter. Originals are never rewritten.

The generated GPS regression fails before implementation and passes afterward, including source audits, reference signs, zero coordinates, missing/invalid GPS and malformed/input bounds. A separate legacy-policy regression verifies revision binding and unchanged observations. Actual child CLI/native map checks and fresh Windows Foundation results must be recorded in M5 validation; a cross-platform parser unit test is not native platform certification.

Follow-up: both the desktop executable and standalone stillharbour-media-worker binary dispatch the closed metadata protocol. A real subprocess regression first failed with exit code 2 at the missing standalone route, then passes for PNG dimensions, generated signed GPS, extra-argument rejection and source preservation. Legacy-policy inspection now bounds retained/imported raw JSON to one MiB before parsing; the oversized-observation case failed before this guard and passes afterward without rewriting observations.
