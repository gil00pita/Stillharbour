# ADR-0004 — App defaults and indexed folder Settings

Date: 2026-10-28. Status: accepted for the owner-requested M1 settings slice.

## Context and decision

The owner requested editable defaults and Library location, explicitly clarifying indexed photo folders rather than catalog/cache storage. Keep the existing native picker, root identity checks, IDs, catalog schema and preservation gates. Change location reconnects the same moved folder; a different photo directory is added through the existing explicit registration/scan flow. Root mode and filters remain per-root. A default browsing-root ID is validated against the current catalog and does not move originals or rewrite old roots.

Persist small nonsecret application defaults in a versioned preferences.json under the existing private app-data directory. A bounded typed Rust model owns defaults and validation; the existing foundation command exposes only typed read/save actions. Native selection paths stay in Rust. File I/O executes off the async command thread, serialized by the preference adapter. Missing files receive factory defaults; linked, corrupt, oversized or future-version files are preserved and reported. Staging, file sync, atomic replacement of this app-owned configuration, and parent sync provide the validated macOS write path. Do not use this adapter for originals. Windows saves remain unavailable until native durability validation.

A preferences-only catalog migration or an unversioned ad-hoc table would couple app defaults to catalog compatibility unnecessarily. Browser localStorage would bypass native validation and lifecycle controls. The separate bounded adapter avoids both and adds no dependencies. Future portable catalog/backup workflows must decide explicitly which nonsecret preferences to include; this is not N2 save/open/relocation.

## Interface and validation

Share ScanOptions/scanForm, Switch, TagInput, FileSizeField and BrowserControls. Settings has explicit draft/save/discard/error/retry states; new folders inherit saved defaults, existing roots retain their settings. Gallery controls remain temporary overrides. Tests cover persistence, invalid updates preserving previous bytes, linked/corrupt/future data, native location-picker routing/cancel, registered-root defaults, form submission boundaries, existing-root preservation and generated source manifests. Actual outcomes are recorded in IMPLEMENTATION_LOG.md. No native picker gesture, Windows or original-mutation safety claim follows from component/browser tests.

## Owner presentation refinement — 2026-10-28

Existing-root Scan settings now lives inline in Settings, using RootSetup's embedded presentation and shared ScanOptions. A selector loads one registered root; gallery links target and focus the group. Its separate form explicitly saves/rescans that root or discards its scan draft without submitting the application-default form. Existing mode/scan IPC, native root validation, partial-failure semantics and read-only original policy remain unchanged. Opening Settings does not scan; no architecture or storage migration follows from this refinement.
