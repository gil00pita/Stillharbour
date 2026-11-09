> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0080: Verified catalog restore activates after restart

Status: accepted for the bounded M4 catalog restore action; native acceptance
results remain separately recorded.

## Decision

Catalog activation is separate from publishing a snapshot into a new output
folder. A completed, fully compared whole-snapshot restore job supplies the
snapshot source ID. A fresh authenticated repository read establishes its exact
manifest and catalog digest before shutdown. A second source kind selects only
the current OS-active transaction's sealed local pre-restore catalog. Renderer
paths, catalog history alone and imported approval records cannot authorize
either source.

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

## Latest retained local catalog

The capability hint exposes only the latest OS-active transaction ID. A native
factory verifies that exact selector/generation, unchanged descriptor bytes and
seal, current catalog identity, and complete frozen pre-restore catalog bytes,
schema and revision. Its non-deserializable proof retains native handles and
is checked again during mapping, approval, preflight and preparation. Roots come
from that pre-restore catalog, which can contain newer roots absent from the
previously selected snapshot. The user confirms each root afresh.

The retained source does not require repository credentials or packaged helpers;
the native workflow/platform gate still applies. UI labels it as local catalog
recovery, with no snapshot or media-backup claim. It produces a new forward
activation transaction and independently backs up the current catalog again.
It never renames an old catalog into the live path, scans arbitrary history or
accepts an imported path. Advancing the OS selector retires the old proof and
latest-source ID. This preserves edits from both sides of a return.

New prepared descriptors use version 2 and explicit source provenance. Snapshot
sources require a restore-job UUID and concrete snapshot ID, with no retained
source fields. Retained sources require the current transaction UUID and OS
generation and forbid job/snapshot IDs. Existing version-1 descriptors parse
only as snapshot provenance; their raw bytes, seal and native identities remain
unchanged. No catalog schema or UUID migration follows from this protocol
evolution. Imported UUIDs, numeric identities/revisions and platform strings
have SQL byte bounds before allocation in Rust.

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
