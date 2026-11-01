# ADR-0051 — Explicit discovery of completed import outputs

Date: 2026-11-01. Status: M2 continuation; Windows native indexing passes 41263692858; corrected macOS fixture teardown and full native suite pass 41265781031.

## Decision

Offer an explicit cataloging action for a completed mixed-copy import, including from retained receipt history. Reauthorize its original parent through the native picker. Backend IDs and retained copy scope determine the output child; the renderer cannot supply a child path or root identity. Require the exact committed successful job, valid import receipt, original namespace identity/volume, complete retained preparation inventory, and every published object's recorded identity, revision, length and single-link state.

Retain the completed publication revision in the in-memory journal history when decoding its existing completion event. This changes no stored journal format or schema. The indexing check reads native metadata; it is not a new full-byte verification or digest baseline. Changed versions and unknown namespace entries refuse registration through this receipt, even when files have the same length/restored modification time. A historically successful receipt remains historical and does not authorize adopting a different version. A later continuation that verifies a changed version does not silently replace the original publication-version evidence for indexing.

Hold the operation gate and source/parent read scopes through admission to the existing scan adapter. Register a new output root as Referenced. Preserve an exact existing root's identity, mode and filter; return its recent actual scan without creating another root or scan. Thumbnail preparation is a different job class and cannot stand in for that scan. An existing registration with no scan in bounded recent history is reported explicitly. Existing registration/scan error behavior retains any catalog row rather than claiming rollback; a retry can inspect that registration.

The ordinary scan owns discovery progress, cancellation and per-entry errors. It never writes originals or output files. Catalog registration is separate from successful scan completion, byte verification and independent backup. Newly observed later additions are ordinary discovery entries and are not added to the immutable import receipt. No checksum baseline is invented.

Keep all current overlap protections. In particular, registering this output as a root makes its ancestor destination unavailable to future organization drafts under the current conservative policy. Do not quietly relax that rule to claim a complete destination-library importer. Full folder-import UX, target-root policy, multi-root/cross-volume/case-cycle/undo and remaining M2 gates precede M3.

## Validation

Generated workflow fixtures exercise registration under managed/referenced source grants, stable root and scan reuse, refused failed imports, same-size/restored-mtime output changes and unknown entries with asserted test-run boundaries. The initial retry test detects thumbnail-job confusion; filter the job class and rerun. Component tests cover picker-bound IDs, actual scan progress/cancellation, foreign/managed/thumbnail responses and cancellation only of newly started late scans. Add packaged native output discovery, query and idempotent retry with unchanged source/keeper/output audits. Actual outcomes follow in the implementation log. No schema, dependency, default execution feature or original mutation permission change.
