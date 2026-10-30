# ADR-0020 — catalog-only organization template previews

Date: 2026-10-30. Status: accepted for the first M2 slice, with execution unavailable.

The owner requested the next stage and prioritized organization templates/previews. P4 local evidence is available, while full P4-06 and recovery/platform gates remain open. The preservation contract permits fixture-based M2 development but does not permit real-library mutation without the missing evidence.

Separate a pure Rust template module from a catalog snapshot adapter and the existing typed library IPC. Snapshot at most 240 explicitly selected UUIDs in one SQLite transaction. Bind native bytes/identities, observations, template policy and stable sequence ordering to a deterministic fingerprint, while returning only display text and relative proposals. Reuse the existing gallery selection toolbar and native dialog/focus conventions. Keep output extension case unchanged and reject unsafe names rather than changing them silently.

Do not turn the existing M0 disposable copy spike into a production executor or treat a display preview fingerprint as approval. No destination is authorized, no immutable plan is persisted and no file operation can start. Native destination capabilities, confirmed companions, saved templates, export and persistent planning precede executor enablement. Exact and advisory ASCII case collisions block all affected rows; actual Unicode/filesystem equivalence still requires native destination checks.

No schema/dependency/capability change or packaging/license addition. [M2 preview contract](../M2_ORGANIZATION_PREVIEWS.md) records limits, actual coverage and the next gate. Existing source-preservation/native/recovery acceptance requirements remain intact.

Continuation, 2026-10-30: expose companion details as a separate on-demand, bounded catalog read rather than eagerly expanding every preview row. One UUID selects same-root/directory name candidates, including excluded/missing observations, with a 64-row cap and visible truncation. Its own fingerprint binds returned observations and native identities; it is not a plan/approval token and does not extend the existing preview fingerprint into one. Review remains incomplete, with no confirmed bundle or execution authority. No schema/dependency migration; persistent destination-aware planning remains the next gate. Current Windows frontend/core validation does not establish macOS generated-library integration or Windows scanning acceptance.
