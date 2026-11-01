# ADR-0040 — Inspect retained copy history without session authority

Date: 2026-10-31. Status: implemented behind copy-workflow; Windows fixture validation passes, remaining target-platform/milestone gates open.

## Decision

Add read-only inspection keyed by an existing operation job and fresh private native destination selection. Reopen existing bounded copy/namespace anchors and ordered events. Require exact job action/owner/review/digest/file count and registered source-root identity. Missing, duplicate, foreign, oversized or invalid history blocks inspection. Opening history cannot insert an anchor, create a stream, restore a session, approve or resume work. Active jobs cannot enter this path. Prepared jobs inspect their existing ready stream; incomplete preparation with no ready plan returns an explicit limitation and retains all objects/facts.

Referenced registered roots may be read for inspection. Managed grants remain required for planning, approvals, preparation and copying. Both modes retain structural source/manifest/native-parent validation. Check live native identities, volume tokens, disjoint containment and shared read leases. Open the exact output namespace relative to the held destination and compare its frozen identity. Refuse another chosen destination even if a caller supplies matching-looking identity fields. This adds no imported-catalog authorization or relink authority.

Freshly inspect each selected file using paced complete-byte comparison and native object/membership checks. Preserve structured per-file errors and needs-attention states. Do not replace inaccessible files with empty data, call mismatches proven damage, rewrite completion history or adopt occupants. Whole-root/history/admission failures remain request errors. Inspection performs no media or journal writes.

Recent organization Jobs expose fresh destination selection, bounded file paging, exact mappings and states/errors. Responses must match job/root/stream/file count; ignore unmounted responses. Unavailable builds do not open the picker. This path survives loss of live session authority and grants no resume permission.

Session and retained inspection share the workflow's single admission permit with copying. Conflicting work returns Busy before byte inspection; native scope leases remain an independent safeguard. Byte inspection remains synchronous on the backend blocking worker; cancellable background inspection jobs are not added by this decision.

## Validation and next gate

Generated application regression covers a fresh empty controller, referenced-root inspection, wrong-destination refusal, changed-output preservation and unchanged journal count. Component cases cover native selection, structured errors, unavailable builds, foreign responses and unmount. Windows native extension discards live session authority before inspection through Jobs. Actual outcomes belong in IMPLEMENTATION_LOG.md.

Approved resume, idempotent retry, incomplete preparation reconciliation, multi-root/import/cross-volume/undo and remaining M2 acceptance remain open before M3. Default copying and original mutation gates remain unchanged.
