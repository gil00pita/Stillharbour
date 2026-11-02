# ADR-0080: Verified catalog restore activates after restart

Status: accepted for the bounded M4 catalog restore action; native acceptance
results remain separately recorded.

## Decision

Catalog activation is separate from publishing a snapshot into a new output
folder. A completed, fully compared whole-snapshot restore job supplies the
opaque source ID. A fresh authenticated repository read establishes the exact
manifest and catalog digest before shutdown. Renderer paths, catalog history
alone and imported approval records cannot authorize activation.

The first implementation requires the same catalog UUID, current schema and
platform. It compares bounded `sqlite_schema` tuples with the schema generated
by compiled migrations before querying catalog tables or opening a candidate
writable. Unknown or redefined tables, triggers, views and virtual tables are
rejected; imported SQL is never executed as migration code. Older versions and
cross-platform relinking remain visibly unsupported until their conversions
have native acceptance coverage.

Every restored root needs a fresh native folder selection matching its saved
filesystem identity and volume. A moved root may receive a confirmed new native
path; a missing or unrelated folder cannot acquire authority. Any current
catalog edit, source change, expired review or changed mapping invalidates the
exact approval. Catalog records are selected as a whole; newer local edits are
not silently merged or discarded.

The native shell validates approval before stopping work. It closes ordinary
RPC admission, stops and drains owned jobs and watchers, holds operation/cache
leases, then freezes the catalog actor against writes. Preparation creates an
independently validated consistent current-catalog backup and a separate new
candidate under `catalog-restores/<transaction UUID>/`. It rewrites only the
confirmed root paths using compiled SQL, verifies the result and records native
identities and full hashes. The active database is never overwritten.

The native appdata lifetime lock and OS-backed generation authority seal the
pending descriptor. On the next launch, selection validates the exact descriptor,
candidate, retained backup, schema and root mappings before opening the selected
catalog. A stale, tampered, ambiguous or incomplete handoff fails closed and
retains all catalogs. It must not fall back to creating an empty catalog. Native
runtime approvals are recreated; restored schedule rows lack the new catalog
instance's SHS2 consent.

Preparation is irreversible for the current session: failure after the barrier
requires closing and reopening the retained original. A successful preparation
reports that restart is required; it does not claim the active catalog has
already changed. This is a single-catalog recovery action, not the broader N2
catalog-management interface.

## SQLite basis

The actor's [`query_only` setting](https://www.sqlite.org/pragma.html#pragma_query_only)
blocks ordinary data/schema changes on that connection; it is not a universal
filesystem read-only guarantee. Imported validation additionally opens with
[`SQLITE_OPEN_READONLY`](https://www.sqlite.org/c3ref/open.html), uses only fixed
queries, and checks the documented
[`sqlite_schema` object definitions](https://www.sqlite.org/schematab.html)
against compiled application schema. The candidate is opened writable only
after that comparison, and compiled relink statements never execute imported
schema text as code.

Owned candidate and descriptor files are flushed through native writable
handles. macOS additionally flushes the containing transaction directory.
Windows does not claim an untested POSIX-style directory flush. A handoff whose
descriptor or candidate is missing after an abrupt interruption fails closed;
the unchanged original catalog remains retained. Process-exit tests are not a
claim of universal hardware power-loss durability.
