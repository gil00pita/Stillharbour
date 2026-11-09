> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0050 — Mixed import review and durable verification receipts

Date: 2026-11-01. Status: M2 continuation; full checks and Windows/macOS native UI/recovery pass in CI run 41262382228.

## Decision

Keep the full confirmed source manifest immutable. Validate the retained import projection, reobserve the original explicit companion decisions and destination scope, then compute capacity from only its exact copied file IDs. Reject empty, duplicate or foreign subset IDs. Preserve the original manifest ID/digest in the assessment; never persist the temporary filtered capacity input as a new manifest.

Projected preparation and copying use the existing separate reviews, one-use approvals, durable jobs and namespace effect identity. Fresh review and continuation reconstruct the same retained import projection and compare its captured source mode. Ordinary copying retains its original managed-source policy. The private review factory checks the selected native destination before binding planning metadata. Equivalent source/template/target effects cannot allocate another namespace.

## Validation and remaining scope

Generated fixtures register existing comparison roots before drafting so destination overlap checks include them. Resolve their Windows test paths before exact component traversal. Exercise real prepare/copy jobs for managed/referenced sources, exact new-file capacity and immutable full manifest, separately approved actions, late inaccessible keepers, preserved originals and refusal to allocate another output namespace.

The feature-gated ID-only application route now binds a saved import plan to a fresh native destination selection. The frontend validates exact new-file IDs, bytes, targets and capacity against the full retained manifest. Preparation/copy remain separate approvals. Closing a mixed review revokes its live approval or requests cancellation; validated late responses are released/canceled. The legacy retained `executionAvailable` flag continues to distinguish the all-existing action class, preserving stored decision hashes; copy capability and the separate mixed route govern the copy class.

After final fresh keeper verification, record bounded `import_copy_verified_v1` evidence before committing job success. Bind job, stream, original manifest, saved decisions, copied IDs, skipped keeper mappings and actual aggregate keeper bytes. A receipt-write failure stops successful completion even if verified outputs exist. Evidence alone is historical: the receipt reader requires the exact job's durable CompletedVerified state. A failed final job write therefore leaves recoverable output and no completed-import receipt. Continuation follows the same rule and needs fresh approval/bytes. Saved import history reads completed receipts from the bounded job list without restoring authority.

Prepared-import reopening returns the full immutable manifest together with the optional saved import plan. Omit the field for ordinary copy responses. Validate the prepared review against that exact subset while retaining the full manifest for decisions/history, then require fresh fidelity acknowledgment and copy approval. Never compare the full import count to a new-only copy job or silently replace the original manifest with its subset.

Focused generated tests also inject receipt and job completion denial, verifying output/source retention and absence of false completion. Component tests reject skipped members in a copy review and foreign receipts. The native desktop scenario sets up explicit generated inventory decisions through actual IPC, then exercises mixed review, approvals, jobs and retained receipts through the UI. Actual results follow in the implementation log; adding the scenario is not a passing result. Folder-import indexing, multi-root/cross-volume/case-cycle/undo and other M2 gates remain open before M3. No schema/dependency/default feature/original mutation grant changes.
