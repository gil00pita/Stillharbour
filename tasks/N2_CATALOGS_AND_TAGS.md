# Stillharbour - N2 - Catalog files and photo/video tags

Prerequisites: M1 catalog/database actor, validated snapshot/collision-safe publication, jobs safe-stop/recovery; M5 annotation model may be extended here rather than duplicated. Source-writing metadata adapters remain disabled.

Implement addendum sections 8-9, extending existing catalog.open/export and tag commands. New/Open/Recent/Save catalog copy, local writable catalog policy, untrusted file validation, transactional migrations with retained prior snapshot, root reauthorization, no auto-replay/schedules, catalog-only photo/video tags, undo/history and import provenance. Existing sidecar export may be integrated only if protected-output gates pass; otherwise show it unavailable with a reason, not a fake export.

Verify the runtime SQLite includes the documented WAL-reset fix and current supported security fixes. Test actual bundled version; do not rely on the development machine's CLI version.

Required scenarios: LF-029 through LF-044, LF-061, LF-062 plus applicable AC/PS requirements. Gate: safe catalog round-trip including offline media, tags/recipes/baselines/import provenance retained, no original byte changes, no unsafe local/network file open. Separate native fixtures are needed for Windows/macOS location/lock behavior. Stop after evidence.
