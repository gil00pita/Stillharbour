> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0063: Catalog-scoped import destination preference

Date: 2026-11-01. Status: implemented; local full/native checks pass. Target CI follows the checkpoint.

Store the preferred import root in SQLite schema 8, as a singleton nullable foreign key to roots with ON DELETE SET NULL. Return the actual catalog UUID with the choice. Validate that UUID on every save, including clearing, and require a managed root on both initial validation and the committing transaction. An offline managed root may remain a preference; it does not authorize copying. No catalog UUID, root identity, global preference format or source file changes.

The Settings section has its own save action, separate from global defaults, browsing defaults and folder grants. An earlier prototype used the global preferences writer; its focused Windows test correctly hit that writer's existing native durability gate. Keep that gate unchanged and store the catalog-specific choice with the catalog. This also carries the hint through consistent catalog snapshots.

Only a new organization destination form uses the matching catalog's hint. Missing, offline, read-only or selected-source roots remain explicitly blocked until reviewed or changed. Do not silently substitute an external folder. Changing the hint does not retarget immutable plans or bump their observation revision. Native picker identity, fresh root validation, preview and separate operation approval remain required.

Migration uses the existing retained consistent snapshot and transaction. Extend actual child-exit coverage through version 7 (eighteen boundaries); test independently opened prior-version snapshots, preserved identities/history, collision rollback, failed preference writes, foreign catalogs, cleared deleted-root references and unchanged generated neighbor bytes. Native UI tests save/clear through real IPC without changing global defaults or grants. Full validation results belong in the implementation log.

The diagnostic binary reports SQLite 3.54.1, source ID 2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76. Checked primary [release notes](https://sqlite.org/releaselog/3_54_1.html), [WAL documentation](https://sqlite.org/wal.html) and [advisory table](https://sqlite.org/cves.html) on 2026-11-01. This version includes the documented WAL-reset fix and is newer than the listed 3.53.2 FTS5 fixes; this is version-specific evidence, not a blanket security claim. No dependency or lockfile change.
