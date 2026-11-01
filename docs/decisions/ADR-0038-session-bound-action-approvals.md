# ADR-0038 — One-use session approvals for exact native actions

Date: 2026-10-31. Status: constructor-owned workflow integration; no production approval IPC or job flow yet.

## Decision

Keep action approvals in bounded backend memory, separate from persisted catalog plans and journals. A review binds the explicit preparation/copy action, immutable namespace/copy stream ID, digest of its complete retained binding and ephemeral native destination-selection ID. Backend-generated random tickets authorize only that exact review. Never deserialize approval entries or one-use proofs from a catalog, imported file or renderer payload.

Allow at most 64 unused tickets and expire them after ten minutes measured by monotonic Instant. Purge expired entries before admission; fail closed on poisoned state, backwards time, duplicate IDs or a full table. Remove a recognized ticket atomically on its first attempt, including wrong scope/selection/expiry attempts. Missing, replayed and previous-session tickets fail. The returned proof is neither cloneable nor serializable; action authorization consumes it and rechecks its scope and expiry immediately before entering the native workflow. This is admission expiry, not a ten-minute limit on an already admitted job.

Preparation/copy wrappers independently recompute the expected review from their own retained evidence. The copy wrapper supplies its own exact plan and journal to the engine, accepting only backend native I/O/scope inputs rather than a caller-selected replacement plan. Actual managed-root permission and binding checks run after approval. A compact native-root/grant check accompanies each engine scope boundary, including immediately before publication, so revoking a managed grant cannot be bypassed with a cached ticket. Preserve staged bytes if access is revoked during copying.

The generated native catalog diagnostic issues fixture session approvals before real preparation/copy and proves foreign-selection rejection before any output/fact, plus preparation/copy ticket replay refusal. Its executionApproval=false remains correct: these are backend fixture grants, not an actual user review/approval flow. No raw ticket or proof is written into receipts/catalogs. Persisted facts do not recreate authority.

## Validation and remaining gate

Test changed action/ID/digest/selection, exact expiry, backwards time, single-attempt rejection, restart loss, 64-entry admission/expiry release and concurrent double consumption. A valid proof for another scope/action must create no preparation object or fact. Revoke the managed grant after approval at admission and after staged verification: require zero events or four retained events respectively, no target publication, fresh staged-byte inspection and unchanged sources/neighbors. Existing copy/preparation/recovery tests remain required.

Production native-selection/session ownership, explicit review/approval IPC/UI, durable job/audit states, controls, capacity checks, idempotency/resume, import, undo and remaining M2 native acceptance remain open before M3. Original-file mutations still require the independent exact-version recovery gate. This change adds no production write endpoint or recovery-provider claim.
