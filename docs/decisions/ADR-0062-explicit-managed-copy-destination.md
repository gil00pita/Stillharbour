# ADR-0062: Explicit managed copy destination

Date: 2026-11-01
Status: implemented and locally validated; target CI pending; M2 remains open

## Decision

Keep the existing external-folder destination policy as the default, with bidirectional overlap rejection against every indexed root. Add an explicit destinationRootId policy for a different existing managed library root. The native picker must select that exact root object; child/ancestor paths, selected source roots, referenced/unavailable roots, app-data, protected packages, other overlapping roots and unresolved original recovery remain blocked. No picker path or root ID alone grants execution.

Bind the optional destination ID into immutable draft content and destination fingerprints. Omit absent fields so old external-folder serialization/digests remain unchanged. Catalog schema remains 7; do not reinterpret the base-library flag or existing browsing-default preference as a preferred import destination. Persistent preferred-root selection and multiple source-root execution remain separate unfinished M2 work.

Recheck root mode, availability, native identity/volume and original recovery at plan insertion and copy scope/boundary validation. Retained directory handles and source-preserving no-replace publication remain required. Native read scopes may coexist with ordinary scanning; incompatible writes are still excluded by the shared registry. New output stays in the separately reviewed owned namespace and existing destination-only files survive.

Completed import indexing into this destination uses the existing root/filter rather than registering an overlapping child root. Join an active ordinary scan or explicitly request one. Return scanStarted separately from alreadyRegistered so late UI disposal cancels only the scan this request started. Root/asset identity remains stable; another explicit request may rescan after completion. Default external outputs retain referenced-root registration.

The UI exposes the policy, disables source/referenced/unavailable choices and clears picker/review/draft results when it changes. Generated native automation creates a separate disposable managed destination, checks the actual UI, then removes only its empty catalog registration through a fixture-only adapter while preserving its neighbor. The adapter checks the constructor prefix, exact native/path/volume/catalog binding, non-base managed status, zero assets and no active jobs under a shared write lease and database transaction. It never deletes cache rows/files or invokes the production cleanup adapter; the Windows cache-removal gate stays unchanged. This test-only adapter accepts no paths, root IDs or supplied scope.

## Validation

Exercise explicit/default distinction, exact root identity, source/child/unknown-root rejection, revoked target grants, truthful stale drafts, separate preparation/copy approvals, full destination verification and source/neighbor retention. Extend mixed-import/indexing and late-response scan ownership tests. Preserve external-folder legacy identity and existing fault/recovery tests. Run complete/separate-feature checks, rebuilt native Windows/macOS smoke and production exclusions; record actual results before claiming this partial M2 coverage. Real-library original mutation/quarantine and full M2/M3 acceptance remain gated.
