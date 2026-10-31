# ADR-0025 — Native creation of plan-review export files

Date: 2026-10-31. Status: implemented continuation; native validation outcomes belong in the implementation log.

## Scope

Add Save JSON file / Save CSV file to retained plan review. The user selects an existing native output folder; IPC carries only that picker-session ID, plan UUID and export enum. Rust generates the immutable review text and fixed `stillharbour-plan-UUID.json`/`.csv` filename. No renderer path, filename, text, overwrite flag or executable approval is accepted. Existing selectable JSON/CSV text remains available.

The metadata adapter rejects overlap in either direction with every indexed root and app-data/cache folders, refuses Apple Photos packages, verifies picker identity/volume and independently holds the native directory chain. Unavailable/replaced roots block the overlap review. This action creates a new review document outside the library; it does not copy, rename or delete originals or companions. Stale retained drafts can be exported as review evidence, without refreshing their paths or approval state.

## Publication and recovery

Record `review_export_intent` in the existing append-only plan journal before creating an exclusive UUID `.partial` stage. Evidence includes private native folder bytes and identity, fixed names, intended byte count and BLAKE3 digest. No new schema or immutable plan rewrite is required. Limit generated review text to 16 MiB.

Write and flush the new stage, then reopen/read and compare all bytes. Windows reuses the retained owned-output no-replace publisher from ADR-0023, extending its former SQLite-snapshot-only scope solely to these backend-generated regenerable documents. Only the output leaf uses directory write sharing; ancestors and source roots retain their existing guards. Verify source stage identity/link count before rename and the published file after flush. Generic original-file no-replace/durability guards remain closed.

On macOS, create the stage with exclusive no-follow openat relative to the held directory, use directory-relative renameatx_np with RENAME_EXCL, sync the held directory, then reopen and verify published identity and complete bytes. There is no ordinary rename/overwrite fallback. Unsupported publication or flush semantics fail explicitly.

Success requires byte verification, output-directory revalidation and a completed journal entry. Errors record needs-attention where the catalog is writable; an interrupted/ambiguous publication retains intent. Failed stages are retained, and a completed file whose final journal commit fails is not reported as success. No recovery step deletes/replays/replaces an output automatically. The retained catalog can regenerate the text. This is not original-file executor recovery, independent photo backup or physical power-loss proof.

## Validation

Owned temporary fixtures verify exact JSON/CSV bytes, unchanged destination-only neighbors, no replacement on filename collision, retained failed stages, original-root rejection, native picker authority and literal rendering. Windows integration also verifies source preservation. Instrumented native actions create a separate owned export fixture and return receipts only for that generated directory; no supplied paths are accepted and production excludes the routes. Actual OS chooser interaction remains distinct from the mocked picker return used by native tests.

Primary API sources: [Apple XNU stdio declarations](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/stdio.h), [Apple exclusive-renaming capability](https://developer.apple.com/documentation/foundation/urlresourcekey/volumesupportsexclusiverenamingkey), and the reviewed ADR-0023 Windows publisher. No new dependency, source mutation capability or backup evidence provider is added.
