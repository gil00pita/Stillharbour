# ADR-0073: Immutable saved comparison profiles

Date: 2026-11-01

Status: implemented increment; target-platform validation recorded separately.

## Decision

Persist comparison bindings in the catalog actor using schema 11. Preserve pre-migration snapshots, UUIDs and all original-operation gates. The bounded table accepts at most 32 immutable profile records, each at most 64 KiB; update/delete guards retain old bindings. Do not store approvals, recovery evidence, passwords, schedules or executable configuration.

Keep native path bytes in Rust, with catalog/root IDs, platform, device/object identities and stable volume tokens. Send display paths and IDs to React. Both new and saved comparisons use the same native containment/read-lease admission. A saved profile always selects its captured root, with no default-root fallback. Changed roots/destinations require a new explicit profile; automatic relink/rebind is absent. Creation rechecks the base binding transactionally. Reopening creates a fresh read-only comparison, never execution authority.

Directory link counts are not persistent object identity and vary on macOS and as children change. Compare directory device/object keys and stable volume tokens; retain revision checks for inventories/files. Do not relax file-byte/revision evidence or mutation gates.

## Consequences

No new dependency or helper is needed. Profiles survive catalog reopening; comparisons/results remain session-only. Persisted records reject foreign catalog/platform/version, malformed native paths and unknown approval fields. The UI keeps profile scope distinct from the current base and retires stale responses. No profile editor/deletion, scheduler, historical baseline, copy or snapshot/restore feature is implied. See [M4 scope](../M4_BACKUP_SYNC.md).
