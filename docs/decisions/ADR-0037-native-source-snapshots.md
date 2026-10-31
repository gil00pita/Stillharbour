# ADR-0037 — Plan native sources before preparing future targets

Date: 2026-10-31. Status: constructor-owned integration; production approval/job flow remains open.

## Decision

Separate a native source snapshot from an executable copy plan. The previous namespace fixture borrowed a direct-copy plan whose destination parent proofs required existing target folders. A future folder cannot have a native identity before explicit preparation creates it. Do not fabricate those identities, create folders during preview or treat an unprepared plan as executable.

Build snapshots from actual retained drafts/manifests and registered managed roots. Bind catalog/draft/manifest digests, source/destination native identities/volume tokens, one source-root ID, each exact manifest file ID/relative target and non-lossy source locator with captured source parent identities, native object, revision and size. Open source/destination spellings before independently checking canonical native ancestor chains; reject overlap and acquire shared read leases. Validate against retained records and recheck every source member through its native parents. This performs metadata observation, not content hashing, source rewriting, native name probing or output creation; it is not an atomic snapshot or approval.

Namespace preparation now retains this source snapshot instead of an execution-shaped base plan. Use distinct namespace_preparation_plan_v2/namespace_preparation_fact_v2 phases. Earlier v1 diagnostic facts remain untouched; they were emitted only inside constructor-owned disposable scopes. No user catalog migration or silent execution of older diagnostic references is performed. Direct-copy bindings without namespace references retain their existing encoding/behavior. Project executable files only after native preparation supplies actual target parent and stage identities.

Freeze the output namespace spelling in the v2 anchor before creation. Support the existing diagnostic pending name and the backend-derived visible `Stillharbour organized UUID` name; both must contain the exact canonical frozen UUID. Stage names remain tied to that UUID and target index. The visible namespace is the actual output folder, created explicitly during preparation and populated only by verified individual publications. No whole-folder promotion/rename is required or claimed. A future production ready-plan review must show the full output-folder prefix and exact paths before approval; existing relative draft targets remain unchanged and gain no approval from this fixture.

Reopening existing namespace facts uses the retained snapshot rather than recapturing source identities under an incompatible preparation write lease. New source planning occurs before that write lease. Fresh execution/recovery checks remain mandatory.

## Validation and remaining gate

Create a new retained draft/confirmed manifest targeting nonexistent `future/nested` folders. Native source planning must leave both folders absent and every source/neighbor byte unchanged. Then actual preparation/copy must create their native parents inside the frozen visible output folder and verify two original stage objects through publication. Reject source planning under an active destination writer and after a source revision changes, without creating outputs or reverting the changed fixture bytes. Existing preparation/copy/refusal regressions and native diagnostic suites remain required.

Production namespace proposal/preparation approval, ready-plan/session approval/jobs, multiple roots, runtime capacity checks, import/idempotency/resume, undo and full native M2 gates still precede M3. No production path endpoint, schema/dependency change, original mutation or independent backup capability is enabled.
