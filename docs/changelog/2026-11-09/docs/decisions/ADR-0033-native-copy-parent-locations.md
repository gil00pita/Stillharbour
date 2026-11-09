> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0033 — Native relative copy locations and parent identity proofs

Date: 2026-10-31. Status: single-root nested executor development under disposable fixture capability; production copying remains unavailable.

## Decision

Extend private copy files with optional versioned source/destination parent locations. A source carries non-lossy native relative path bytes and every intermediate directory's native object identity. A destination carries its portable relative parent and the corresponding object identities. Default/omitted fields retain the old flat plan serialization/digests. No catalog schema, IPC type, dependency, persisted UUID or application identity changes.

Validate source paths as 1–96 normal relative components, at most 8,192 encoded native bytes, with matching parent counts and the known location policy. Reject absolute/prefix/parent/current-directory escapes and protected photo-library bundles. Target parents use the existing bounded portable-relative policy and reject protected packages. Capture observes existing directories only; it creates nothing and grants no authority.

Open each component through its held parent with no-follow native adapters and require each captured native object identity. Check entry kinds and cloud-placeholder status before reading bytes. Native source locators drive execution when present; display strings never supply their path. Catalog binding compares exact locator bytes against the private compiled manifest and reconstructs each complete portable target from its parent plus leaf. It still requires exactly one root and every manifest mapping; multi-root/batch authority is not implied.

The Windows source adapter keeps its strict reader sharing. Owned target parents use the existing owned_directory_child adapter, which permits directory writers while refusing delete sharing and rechecks exact membership. A generic source-directory handle prevented nested publication with sharing violation; do not weaken source handles or silently drop membership checks to resolve that failure. macOS uses the existing no-follow held-parent primitives, with sequential identity checks and explicit dataless rejection.

The engine uses the resolved destination parent for stage/create/verify/publish/readback. Reopen source parents/members and revalidate destination membership before publication and item completion; the final plan check reopens each target parent again. Recovery does the same before inspecting complete/partial objects. Changed/unknown parent objects fail, rather than adopting a replacement directory. Existing no-replace, full-byte comparison, link/revision checks, byte pacing, root leases and durable journal order remain intact. These are sequential observations, not an atomic namespace snapshot or universal exclusion of external macOS directory renames. Sources stay intact even when output work stops.

## Validation and limits

The catalog-native diagnostic additionally copies a generated JPEG/companion from a nested source folder into a nested output folder, reopens its thirteen-event stream, checks complete published bytes and rejects replay. Catalog inventory is fixture-seeded, as previously disclosed; no production scan/copy flow is claimed.

Regressions replace source and output parent directories before execution, preserving the old generated content and a new destination-only occupant; no copy event may be appended. A separate generic-engine fixture copies a source with invalid UTF-16 on Windows or a literal-backslash/Unicode name on macOS, with a deliberately nonexistent display filename, into nested output and recovers by actual bytes. The tested macOS filesystem rejects invalid UTF-8 creation with EILSEQ; the test explicitly verifies that refusal and the codec round-trip. Other Unix uses its stated invalid-UTF-8 fixture. This proves the private locator path, not gallery/confirmation support for such names. Test escape/protected/unknown-policy/missing-parent evidence refusal. Platform outcomes and actual checks belong in the log.

Production preparation/ready-plan/approval/job integration remains pending, including adopting only explicitly reviewed owned empty stages, complete capacity/fidelity and the required import/idempotency/undo flows. Native future-name preparation remains a separate fixture; this extension does not reserve names or create arbitrary real directories. All real original-file mutations and M4 recovery gates remain closed. M2/M3 are unfinished.
