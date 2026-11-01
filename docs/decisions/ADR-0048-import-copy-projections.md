# ADR-0048 — Exact copy projections of retained import decisions

Date: 2026-11-01. Status: private M2 fixture development; complete checks and Windows native fixtures/regression pass; macOS CI pending.

## Decision

Derive the files-to-copy subset from the immutable saved import decision record. Retain original manifest/source IDs, native parent locators, object/version/length evidence, exact targets and connected bundle memberships. Never re-resolve, renumber or mutate the full manifest. An optional backend-only `import_selection` reference binds the saved import ID/digest and the source mode observed for this review. None remains omitted from serialization, preserving existing snapshot/journal hashes and identities.

Retained import decisions must contain an original full source snapshot, never another import projection. Reject recursive references before loading their mapping. Validate a projected snapshot against an exact reconstruction of the saved subset, including order and all native/source/destination fields. Historical read-only inspection remains distinct from current execution grants.

Source-retaining import copies may read a registered referenced source without granting ordinary managed-copy permission. Bind its exact observed mode to the private projected review and recheck it at execution boundaries. Normal snapshots keep their unchanged managed-source requirement. Cache the validated expected mode in the in-memory catalog journal; per-boundary checks remain small current-root identity/volume/mode queries rather than reloading large preparation histories on every byte chunk. Destination write scopes, separate preparation/copy approval and all native no-replace/staging/full-byte/journal rules remain unchanged. This class never writes originals.

Prepared namespace targets derive from the validated source projection. Copy binding reconstruction must match that exact subset; skipped IDs cannot enter it. Effect memberships resolve by manifest ID rather than zipping a shortened source vector against the full manifest's leading rows. Existing complete mappings keep their previous effect identities; equivalent projected mappings remain idempotent across drafts.

## Scope and validation

The projection constructor is test-only at this checkpoint. No renderer route or mixed-import execution button is enabled. Fresh keeper comparisons, import receipt orchestration, capacity/recovery integration and actual mixed-import UI/native acceptance remain subsequent work. A copy projection itself grants no skip authority or import-completed receipt.

Generated native fixtures contain one existing photo/sidecar bundle and one distinct new bundle. Compare/save real decisions, then test exact new-file IDs/targets, rejection of added skipped files/changed targets/foreign or changed references, unchanged legacy serialization, and effect identity equal to a separate new-only mapping with the same bundle membership. Prepare and publish only the two new members through the actual retained-stage/catalog engine under managed and referenced source grants, preserving all four originals and existing destinations. A changed referenced grant must retain the exact empty stages and zero publication history even when the caller callback permits work. Ordinary referenced-source copying must remain refused. Full/native/platform outcomes follow in the implementation log.

No schema/dependency/default feature/original mutation change. M2 remains incomplete and still gates M3.
