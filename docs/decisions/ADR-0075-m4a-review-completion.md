# ADR-0075: Complete read-only M4A review and retain observation history

Date: 2026-11-01

Status: implemented and natively validated on Windows/macOS at 90acf91; actual acceptance evidence is recorded in M4A_VALIDATION.md. Owner review and M4B authorization remain separate.

Use catalog schema 12 for immutable comparison observations. The existing retained-snapshot transaction migrates schema 11 without changing catalog/root IDs or profile format 1. Store at most 32 observations, each bounded to 16 MiB and 10,000 destination records. Retain existing observations when full and disclose that the new observation was not saved. Quick/incomplete/failed/canceled runs never replace a complete observation. Validate imported records defensively and never deserialize approvals or executable paths from history.

A scope key binds native source/destination paths, device/object identities, stable volume tokens and catalog/root namespace. Directory link counts do not define that identity. Complete freshly revalidated destination byte reads produce BLAKE3 context; direct current streaming comparison remains the equality authority. A changed historical destination digest requires review, never an automatic winner, replacement, damage diagnosis or recovery claim. The immutable review-plan descriptor binds the session digest, catalog/root/run IDs, policy 3 and row count; execution remains false and no transfer command accepts it.

Explicit profile rebinding requires fresh native destination selection and review of previous/current scopes. Append a new immutable profile with a supersedes ID; retain the previous binding and require a new comparison. No automatic default-root retargeting or inherited approval exists.

Preview requests carry only run/entry/side IDs. Backend retained native snapshots, current root authorization, read leases and fresh revision checks authorize reads. A single preview slot reads at most 32 MiB through shared paced I/O and delegates to the existing supervised byte-input decoder at 512 px. Validate the root and snapshot again before returning bounded PNG bytes. No renderer path or executable is accepted. Undecodable, missing, changed, revoked and oversized inputs retain text observations. Preview decoding is not fidelity or integrity evidence.

Associate potential same-stem RAW/JPEG/motion/sidecar names within a native directory, retaining each entry and its independent evidence. Display at most 32 associations per entry and 2 MiB total association text; do not infer bundle completeness or redundancy. Include all eligible files independently of the gallery. Native existing-name collisions and platform component rules block unsafe/unsupported review paths; prospective publication-specific name preparation remains an M4B requirement because compare never creates a test name in either tree.

M4A acceptance covers this read-only surface and its failure fixtures on Windows/macOS. M4B copy/pacing/publication/recovery, M4C retained replacement and M4D helpers/protocol round trips remain separate gates. No real-library mutation is enabled.
