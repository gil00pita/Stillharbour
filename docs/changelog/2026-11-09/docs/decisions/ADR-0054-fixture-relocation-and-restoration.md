> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0054: Same-volume fixture relocation and restoration

Date: 2026-11-01
Status: closed generated infrastructure; M2 remains open

## Decision

Keep existing copy publication behind the unchanged `publish_new` signature. Its private implementation accepts separate held source/destination directories. A new fixture-gated `relocate_new` validates exact source membership, single-link object identity, equal native volume tokens/devices and destination absence before using native no-replace publication. Windows addresses the held source handle; macOS uses bounded leaves under held directories and flushes both parents when distinct. Post-publication identity checks remain required. They do not establish immunity from uncooperative external source-name races on macOS.

`executor/rename_fixture/quarantine.rs` has only a constructor-owned generated scope. It moves the generated Case.JPG/Case.xmp pair to a separate pinned directory on the same volume and restores their original names through a new immutable reverse record and separate fixture approval. All five original objects are directly byte-compared against the pinned local versions; unselected originals, private probes and exact source/quarantine memberships remain checked. No replacement, permanent deletion or expiry is available.

The fixture retains bounded plan payloads and ordered intent/completion facts in its private FULL-synchronous SQLite journal. Foreign keys, unique phases and immutable-history triggers reject rewriting/deletion. Failed/interrupted operations preserve actual positions and refuse blind replay. A newer occupant of an original name blocks restoration before a reverse execution plan is recorded.

The continuation slice uses strict `generated-relocation-v2` private records, including the complete five-object revision scope. A separate `ContinueRelocationFixture` action binds fresh complete-byte/essential-metadata checks, exact directory/volume/object/version identities, actual positions and validated phase history. It may record a previously moved object's missing completion after verification, then move remaining members; completed phases are never replayed. Observation records are immutable and capped at 64 per plan, and never serialize approval. A denied observation write stops before any new move. Interrupted continuation itself requires another fresh approval. These v2 records are disposable-fixture metadata, not a catalog schema migration or an imported mutation grant.

The existing strict virtual-volume runner additionally attempts relocation of a separate newly generated probe object across its actual distinct volumes. The primitive must reject it without moving the probe or creating a target. This does not open an indexed source member for relocation and is not a cross-volume move implementation.

## Limits and gate

This is infrastructure for M2, not duplicate detection or M3 quarantine authorization. There is no keeper/last-copy decision here. The retained versions share development storage and do not establish independent recovery or complete filesystem-metadata fidelity. Default real-library mutations remain disabled. Actual Windows/macOS continuation/failure checks and original-organization/undo catalog/job/UI integration still precede M2 acceptance and M3.

Primary primitive references are recorded in [SOURCES.md](../SOURCES.md). Run focused `relocation_fixture` cases, the existing rename/copy regressions, full/separate-feature checks, rebuilt desktop IPC assertions and strict mounted-volume CI. Record actual outcomes in the implementation log; compiling the refusal probe is not a mounted-volume pass.
