# Stillharbour - Logical model and command extensions

v1.2 | Proposed contracts, not a migration script or implemented API

Extend the existing catalog entities instead of creating a second catalog. Review actual repository schema before SQL changes. Preserve schema versions and migration snapshots. The backend owns identity, authorization and all writes.

## Entities and invariants

| Entity / extension | Minimum fields | Invariants |
| --- | --- | --- |
| catalog_manifest | catalog_uuid, lineage_uuid, schema_version, application_marker, created_at, snapshot_id, snapshot_at, activation_id | Source identity/provenance retained; new machine/restored branch gets fresh activation; no imported auto-execute authority |
| assets additions | first_added_at_utc, added_sequence, added_at_provenance, initial_batch_id | Never replace with EXIF or provider upload date; preserve on known rescan/relink |
| import_sessions | id, provider, input_kind, scope_description, completeness_state, state, settings_digest, timestamps | Unknown whole-cloud coverage remains unknown; no provider-deletion capability |
| import_inputs | id, session_id, local_source_identity, archive_digest, declared_part_index/count, parse_version, state | Hash full source part; unknown part count stays null; source unchanged |
| import_items | id, session_id, input_id, member_id, provider_item_id_if_exposed, resource_role, source_revision, staged_identity, destination_id, acquired_digest, fidelity, state | Do not use a URL/path alone as permanent identity; never collapse independent file entries by digest |
| imported_metadata | id, import_item_id, raw_artifact_reference, parser_version, observed_fields, associations, confidence, conflicts | Raw artifact bytes separately preserved/backed up; ambiguity not auto-resolved |
| metadata_artifacts | id, retained_path, digest, length, source_identity, retention_state | Immutable artifact store, no source auto-delete; catalog snapshot alone may not contain large raw artifacts |
| scan_runs | id, scope_digest, root_identities, started/finished, coverage, errors | Enumeration failure does not imply absence |
| scan_observations | scan_id, observed_entry_identity, path, eligibility, associated_file_id | Audit-only observations can be untracked without auto-enrolment |
| integrity_baselines | id, file_id, content_revision, algorithm, digest, length, established_at, origin, previous_id | Append-only evidence history; acknowledging mismatch never overwrites previous row |
| integrity_findings | id, run_id, file_id_or_observation, inventory_state, content_state, baseline_id, observed_evidence_id, resolution_id | Independent inventory/content/decode/backup dimensions; limited scope always visible |
| finding_resolutions | id, finding_id, action, reason, actor, time, new_baseline_id_if_any | Acknowledgement is not verification or recovery proof |
| edit_recipes | id, asset_id, source_file_revision, recipe_schema, geometry, color_policy, renderer_version | Validated finite normalized crop; geometry order specified; never stored as executable code |
| edit_revisions | id, recipe_id, parent_revision_id, recipe_digest, created_at, active_flag | Immutable history; optimistic concurrency; no silent last-writer-wins |
| tag definitions/assignments | existing tag IDs plus parent_id, original_label, comparison_key, source, revision | Preserve display text; uniqueness policy explicit; no media deletion |
| tag overrides | asset_id, tag_id_or_source_key, intent(add/remove), base_observation_revision, created_at | Removed imported keywords do not reappear on rescan; original provenance retained |
| catalog_snapshots | id, schema_version, checkpoint_at, path, checksum, validation_result | Snapshot refers to catalog consistency, not synchronized photos/repository state |

Raw metadata artifacts and original media are not necessarily embedded in a catalog snapshot. Backup must include their immutable storage; opening a catalog with missing artifacts reports this rather than claiming complete metadata recovery. Portable metadata export can package these artifacts separately after review. The app's own versioned metadata/recipe records remain in the database snapshot.

## Import state machine

```text
observed -> planned -> awaiting_source -> acquiring -> staged
         -> verifying -> verified -> publishing -> published -> catalogued
```

Interrupted, needs_attention, unsupported, canceled and paused are explicit states with a resume checkpoint. Publication and catalog commit are not a single filesystem transaction. Recovery examines staged/final identity, content and journal before replay. Reauthorization, retries and catalog reopening cannot mark items verified without evidence.

Session state aggregates every item and supplied archive. The accepted plan includes target/version, fidelity warning acknowledgements, filter scope and unresolved omissions. A modified source, target or resolution invalidates the affected plan.

## Editor geometry (recipe schema 1)

```text
decode original pixels
  -> apply EXIF orientation once
  -> rotate clockwise by 0 / 90 / 180 / 270 degrees
  -> mirror horizontal / vertical in rotated coordinates
  -> crop normalized rectangle in transformed full-image coordinates
  -> encode to a new derivative under explicit output/color/metadata policy
```

Mirrors are booleans and rotation is an enum; do not allow arbitrary shader/code expressions. Clamp only interactive gestures; reject invalid persisted recipe values rather than silently altering an imported recipe. Output pixel bounds use a documented deterministic rounding policy and must include at least one pixel. Existing recipes reference the expected oriented basis dimensions; changed source orientation/digest invalidates the basis.

## Logical IPC families

```text
integrity.audit_preview / integrity.audit_start / integrity.report
integrity.finding_compare / integrity.resolution_preview / integrity.resolve
integrity.report_export_preview
assets.query (sort = first_added; added_range; import_session)
catalog.create / catalog.open_preview / catalog.open / catalog.recent
catalog.snapshot_preview / catalog.snapshot / catalog.validate
catalog.relink_preview / catalog.relink
edits.get / edits.preview / edits.save_revision / edits.revert
edits.export_preview / edits.export
imports.providers / imports.discover / imports.preview
imports.start / imports.resume / imports.report
metadata.association_preview / metadata.association_resolve
tags.query / tags.batch_preview / tags.apply / tags.merge_preview
metadata.sidecar_export_preview / metadata.sidecar_export
```

Reuse existing jobs, operations, root permissions and safe file publication rather than implementing independent executors. Commands use catalog activation, stable IDs, expected record revision and approved plan digest. Secret-bearing provider calls run only in the backend with validated endpoints.

Catalog-only transactions such as saving a tag do not need a full filesystem-operation wizard; enforce a narrow validated transaction, revision checking, undo/history and durable catalog storage. Operations writing a sidecar/derivative or publishing an import require the file-operation safety workflow. Do not misinterpret 'non-destructive' as permission to overwrite any existing non-original output.

## Security and consistency notes

Catalog import is not SQL script execution or permission import. Disable privileged custom SQL functions, unapproved ATTACH operations, extension loading and automatic jobs. Use restricted read validation before approved migrations. No schema-triggered filesystem access.

Clock times preserve uncertainty; stable sequence is not evidence of capture order. Catalog restore/import invalidates prior approvals, mutable queued jobs and stale current-availability claims. A checksum match of a restored database is not proof that referenced media still exists.
