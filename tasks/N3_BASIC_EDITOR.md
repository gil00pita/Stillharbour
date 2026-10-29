# Stillharbour - N3 - Recipe-based basic editor (desktop)

Prerequisites: M1 image preview with tested orientation/color behavior, N2 catalog durability, safe derivative export. Recipe storage is catalog-only; filesystem export uses existing protected operations.

Implement addendum section 4 on Windows/macOS: preview toggle, crop, 90-degree rotate, mirror both axes, before/after, undo/redo, revert and Save edits. Store immutable source-revision-bound recipes with explicit geometry. Export edited copy is a new separate file. Do not rewrite EXIF orientation, re-encode an original or treat a preview decode as full-quality RAW export.

Required scenarios: LF-018 through LF-028 plus relevant AC/PS tests. Validate all eight EXIF orientations, crop/rotation/mirror ordering, color/alpha policy, cached thumbnails, stale edits and conflicting output. Inspect generated render fixtures and unchanged original hashes.

Gate: tested recipe geometry, durable edits, unchanged originals and collision-safe export on claimed formats. Mobile code/shared schema considerations do not establish mobile support. Stop before N5.
