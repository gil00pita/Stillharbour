# M5 readiness review

Final development gate, 2026-11-03: [M5 closeout](M5_CLOSEOUT.md) records acceptance of the implemented offline map, catalog metadata and reviewed exports into new files. The deterministic export correction passes native Windows/macOS CI for the exact merged application tree. Unverified formats, release acceptance and M4 production authority retain their separate gates. The readiness review below is historical; stop before M6.

Current final implementation: see [M5 validation](M5_VALIDATION.md) and [ADR-0084](decisions/ADR-0084-reviewed-metadata-exports-and-tag-history.md). The owner subsequently requested “Finish M5”; schema-17 tag history/overrides and reviewed catalog/XMP exports are implemented. Remaining acceptance restrictions are explicit there. The readiness review below remains historical and does not revoke later authorization.

Current continuation: the owner subsequently requested “continue with m5”. Read-only M5 development is now authorized under [ADR-0081](decisions/ADR-0081-offline-geographic-browsing.md); [M5_MAP.md](M5_MAP.md) records the first increment. The later [M5_METADATA.md](M5_METADATA.md) continuation adds schema-16 catalog metadata organization while keeping export and whole-milestone gates open. The review below remains historical evidence of unclosed M4 preservation gates, which this authorization does not waive.

Review date: 2026-11-02. Request: check progress and start M5 if the preceding work meets its gates. **M5 implementation has not started under this request: M4 integration passes CI, but its preservation acceptance remains open.** The previously approved macOS format-preview slice is retained.

## Repository and CI evidence

- The workspace was clean at `7a8174096ee26b0bf01db8ca436410d6d2da440c`, matching fetched `origin/main`. The current branch is unchanged.
- [M4 PR #14](https://github.com/gil00pita/Stillharbour/pull/14) merged on 2026-11-02. Its final head was `bd28314ef7706213309c8403bec306ad11e32107`.
- Both Windows and macOS jobs passed in final-head [Foundation checks 41427552015](https://github.com/gil00pita/Stillharbour/actions/runs/41427552015) and [transport checks 41427552080](https://github.com/gil00pita/Stillharbour/actions/runs/41427552080), verified through GitHub's API. The earlier timeout-fix runs `41426372917` and `41426372967` also passed.
- The final PR head and merged workspace have identical Git tree `daf078bb354d1386dd0ea419573d3997ee0e9895`. The passing final-head checks therefore cover the merged tree. The additional [post-merge Foundation run 41434439502](https://github.com/gil00pita/Stillharbour/actions/runs/41434439502) was still in progress when checked; no pass is claimed for it.
- This review checked run/job status, source and recorded acceptance evidence. It did not download or independently re-audit the latest native receipts, rerun native tests, or operate on a personal library.

## Milestone position

| Milestone | Current position                                                                                                                                                                                      |
| --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M0–M1     | Foundation and read-only library implemented; platform, packaged-format and release claims retain their documented limits.                                                                            |
| M2–M3     | Authorized fixture/read-only gates accepted; production original-file mutation and duplicate quarantine remain disabled. See [M3 validation](M3_VALIDATION.md).                                       |
| M4        | Comparison, source-retaining copy, retained history, snapshots and catalog recovery integrated, with passing both-platform CI. Full preservation certification and production execution remain gated. |
| M5        | Earlier bounded macOS format-preview work exists. Maps, remaining metadata interoperability and complete format acceptance are pending.                                                               |
| M6        | Release hardening, signing and clean-machine distribution remain later gates.                                                                                                                         |

## Why the M5 handoff is not clear

[M4 validation](M4_VALIDATION.md), the [application acceptance matrix](M4_APPLICATION_ACCEPTANCE.md) and [M4 task](../tasks/M4_BACKUP_RESTORE.md) explicitly retain the stop before M5. A merge or passing CI does not complete acceptance scenarios outside the workflows.

| Remaining M4 requirement                 | Evidence needed                                                                                                                                                                                                                                            |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Strict retained OLD-version recovery     | Actual provider admission and fresh restore/comparison of exact bytes and essential metadata with proven independent recovery. Windows audit-security fidelity is currently uncertified; synthetic proofs cannot replace native evidence (PS-009/023/024). |
| Retained-provider failures and conflicts | Actual denied history writes, quota exhaustion, corrupted retained history, rollback/path conflicts and repeated history-name collisions through the application provider (PS-010/021/038).                                                                |
| Complete preservation matrix             | Close the other partial/open rows with applicable native evidence, including path/alias races, restore-publication phases and changed trust/secret-handling checks. Keep unresolved repository locks and unsupported capabilities explicit.                |

Source inspection agrees with these gates: `src-tauri/Cargo.toml` has no default workflow features; snapshot execution in `library/snapshots.rs` requires the backup feature and verified helpers; readable-copy commands in `library/sync_workflow.rs` reject execution without their feature. The Map navigation control remains disabled. These are intentional current boundaries, not evidence of milestone completion.

The next gate is M4 preservation closeout against all forty PS scenarios and the applicable AC cases. Unsupported targets must remain unavailable. Any reduction of the required milestone scope needs an explicit owner decision; this review changes no requirements or production gate.

## Prepared M5 scope after handoff

1. Record the production tile-provider or self-hosted/offline data decision, attribution, privacy, cache limits and offline rights. Implement bounded local spatial queries and viewport clustering with valid-zero/missing GPS, identical-coordinate and antimeridian fixtures; connect filters, selection, filmstrip and library navigation (AC-074–076).
2. Complete catalog-only albums, hierarchical tags, annotations, saved searches, provenance and portable export using existing catalog behavior. Explicit XMP export needs unknown-data preservation, external-conflict review and the preservation executor boundaries; referenced originals stay read-only (AC-083).
3. Extend the existing [format capability evidence](FORMAT_SUPPORT.md), separating previews from full validation and marking untested targets unverified. Include keyboard/screen-reader map alternatives (AC-079), clean-machine packaged capability checks (AC-077) and M5-F01–F06. AC-083 is the XMP-conflict test, not the clean-machine format test.

No provider dependency, schema migration, network service, metadata writer, editing module, cloud import or M6 work is added by this review.
