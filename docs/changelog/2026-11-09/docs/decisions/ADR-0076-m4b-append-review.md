> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0076: Revalidate append-only reviews before transfer integration

Date: 2026-11-01

Status: accepted for the initial M4B increment; transfer gate open.

The owner requested updating to main and starting M4B after the accepted M4A gate. Main at `eaa3a8a` includes the policy-3/schema-12 comparison implementation. The existing M2 copy executor owns staging, pacing, publication and recovery. M4B needs an exact reviewed scope before connecting to that executor. A read-only comparison digest is not transfer approval.

Add a pure append planner and backend-only `Library::backup_append_review`. Only a complete Verified comparison in the current session is admitted. Reacquire read leases and repeat native root/volume/overlap/recovery validation. Reinventory both scopes, compare every observed object and directory revision against the frozen report, and revalidate all file snapshots. A new dataset, changed membership, replacement object, incomplete listing, writer lease or retired session fails closed. Review creates no directory, name probe, stage, approval or journal entry.

Source-only regular files receive CopyNew; destination-only files receive KeepDestination; verified equal files receive LeaveUnchanged. Every different-content or destination-edited row requires an explicit ID-bound KeepDestination decision. Other categories block the plan. Decisions cannot refer to another report, repeat, or suppress a source-only copy. Uncopied differing source revisions have a separate count, so eventual transfer completion cannot imply whole-source protection. Potential companion strings remain context only: all regular files participate independently of gallery filters, without bundle-fidelity or source-cleanup authority.

The deterministic policy-1 append digest binds the comparison/run, native paths, identities, volumes, revisions, namespace and sorted decisions. Native evidence stays private; output paths are display data. The response is serializable for inspection but not deserializable as authority. It has `execution_available = false`, no IPC handler and no approval/start consumer. Schema 12, existing IPC bindings, profiles and the production UI remain unchanged.

One append review shares the comparison-preview admission gate. It clones at most the bounded current report, retains frozen inventory metadata and performs bounded metadata enumeration. It adds no payload reads or helper process. Existing byte pacing and disclosed limits remain unchanged. Review is not an atomic snapshot or immunity to undetectable external writers; fresh native preparation and full-byte verification remain required for execution.

Next: connect the exact subset to the shared executor; implement fidelity/capability/capacity review, exclusive namespace/name preparation, approval, durable intent/staging/flush/read-back/no-replace publication, cancellation/pause/limits, restart inspection and native failure matrices. Production transfer stays disabled until those gates pass. M4C replacement and M4D snapshot/transport/recovery evidence remain separate. This increment does not complete M4B.
