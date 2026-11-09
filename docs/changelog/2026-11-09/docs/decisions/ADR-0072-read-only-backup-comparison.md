> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0072: Read-only Backup & Sync comparison

Date: 2026-11-01

Status: accepted for the initial M4A increment; milestone acceptance pending.

Reuse handle-relative inventory primitives, `DirectoryChain`, shared operation read leases and `PacedIo`. Generate IPC from Rust; accept only native selection and run IDs. Keep one bounded worker and one session report per catalog with cooperative cancellation and paged rows. Add no dependencies or catalog migration.

Bind runs to the base root UUID and compare a dedicated catalog/root destination namespace without creating or adopting it. Infer absence only from complete inventories. Quick results remain unverified; equality requires full bytes and fresh native preconditions. Return review proposals and a digest, with every execution/replacement/recovery capability false. Production backup availability stays false; there is no approval or mutation route.

Session reports expire on restart/replacement and grant no durable protection. Profiles, historical evidence, richer inspection and write/recovery adapters require further reviewed work. Read leases coordinate owned processes; revision checks are not atomic tree snapshots. See [scope and limits](../M4_BACKUP_SYNC.md).

Primary API references reviewed: [Rust Read short-read contract](https://doc.rust-lang.org/std/io/trait.Read.html#method.read) and [React effect cleanup](https://react.dev/reference/react/useEffect). Existing pinned native adapters/dependencies are reused.
