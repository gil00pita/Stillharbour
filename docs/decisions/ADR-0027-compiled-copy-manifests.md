# ADR-0027 — Reconcile explicit companion decisions into retained copy manifests

Date: 2026-10-31. Status: planning implementation; no execution authority.

## Decision

Compile exactly one explicitly selected ADR-0026 companion decision per selected photo in a saved draft. Do not choose the newest decision automatically. New confirmations become the user's current choice; historical decisions require an explicit Use this decision action. Changing choices visibly invalidates the displayed compiled review until compiled again.

Validate each retained decision against its digest, full reviewed inventory, source/draft identity, acknowledgement and pure target mappings. Reobserve each live direct folder and require the exact retained inventory fingerprint and selected native identities/revisions. Check the current draft/catalog fingerprint before retention. These sequential observations are not an atomic snapshot or an execution approval; all selected files need fresh execution preconditions later.

The pure compiler combines entries only when root/native locator/object/revision/byte evidence and exact target agree. Retain every bundle membership. A source mapped to different targets, different sources mapped to one target, possible ASCII case aliases and file/directory conflicts remain explicit blocking issues. Never choose a winner or allocate new suffixes after review. Case checks are conservative warnings and do not establish future native Unicode/case equivalence. Hard-linked locators remain separate paths; they are not independent recovery evidence.

Bound contributions to 240 photos times 65 files, aggregate retained companion evidence to 8 MiB, serialized manifest to 8 MiB and distinct compiled records to 64 per draft. Full proposed output bytes use u128 arithmetic and decimal strings; identical merged mappings count once. Ambiguous mappings remain separate proposed outputs until resolved. UI pages 64 files and shows bounded conflict summaries; the complete conflicts remain in the retained record.

Append copy_manifest_compiled records to the existing journal, leaving schema 3, draft digests and persisted identity unchanged. Bind policy, exact decision IDs/fingerprints, original destination observation, private native locators and public rows into the manifest digest. A scope key makes fresh unchanged retries idempotent. Opening a retained manifest verifies its draft/identity/digest and recomputes reconciliation; it does not revalidate live files or grant execution authority.

## Next gate

The manifest remains executionAvailable=false and copying stays unavailable. Destination observations still describe the original draft; they are not complete companion-target checks. Native future-name semantics, complete destination validation, capacity/fidelity/content evidence, approval binding, centralized execution/recovery, import and reverse-plan undo remain required. The future executor must independently load and validate exact manifest IDs, reauthorize roots, revalidate current source/destination evidence and bind approval to the resulting ready plan. Imported history cannot replay approval.

Tests cover overlapping mappings, changed source evidence, cross-bundle conflicts, hostile targets, decimal precision, explicit decision count/identity, live inventory invalidation, idempotent retries, retained reopening, literal UI rendering, pagination, errors and late responses. Native tests use generated media and retain independent source/destination receipts. Actual results belong in the implementation log.
