# ADR-0068: Reopen an owned generated organization scope

Date: 2026-11-01
Status: experimental implementation; validation pending; M2 remains open

Add an explicit generated-only controller action to retire its live session and reopen retained state. The renderer supplies no path, plan, approval or recovery record. The backend retains only the constructor-owned temporary-directory capability, its native identity, plan identifiers and a digest of the exclusive retained metadata record. It drops the live graph, catalog actor, worker activity and every approval before reconstruction. A fresh read must match the retained digest before decoding or journal inspection.

Share the bounded, deny-unknown-fields decoder with the existing real child-exit harness. Validate canonical forward/reverse graphs, exact five-member constructor mappings, complete retained object/byte/essential-metadata evidence, source/version/probe identities and the actual catalog object/UUID/member mapping. Current positions come from native inspection and the journal; no completed move is automatically replayed. New approvals are empty. This is fresh actor/session reconstruction inside the native application, not a claim that this UI action restarts the operating-system process or provides independent recovery.

Retained metadata creation is exclusive and read back. An existing differing/partial record is preserved and blocks reopening. Failed retention keeps the live session and its files, with old approvals revoked. A failed reopen keeps the owned directory capability for manual review or an explicit retry. Shared temporary-directory ownership prevents dropping a retired session or recovery handle from cleaning up a still-live reconstructed scope. Only the final constructor-owned capability may remove these disposable fixtures.

The controller admits one active action and refuses reopening while a native worker is running. Catalog readers use a short independent lock; status retains an atomic constructor-scope flag. A background reader that delays the catalog's lifetime-lock release may make reopening fail safely; it never authorizes a second writer. The UI clears tickets and acknowledgments and requires fresh approval after reconstruction. The native generated scenario reopens its stopped one-step state before continuation and undo.

No catalog schema/dependency change, arbitrary-library adoption, production mutation authority, independent recovery or physical power-loss claim is introduced. General application-process restart discovery and the remaining M2 acceptance gates remain separate. Run the shared real-exit/corruption, ownership/failure/controller, full/separate-graph and native checks; record actual outcomes in the implementation log before another authorized checkpoint.
