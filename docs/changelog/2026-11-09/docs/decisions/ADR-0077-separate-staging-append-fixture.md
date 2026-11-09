> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0077: Separate staging and verify generated append recovery

Date: 2026-11-02

Status: implemented at the generated-fixture boundary; production M4B gate open.

M4B reuses the central copy executor. Add an optional native parent locator for staging, independent of the publication parent. Missing locators preserve the exact legacy serialization/digest and same-parent behavior. Execute, inspect and explicitly approved continuation validate both parent chains. Publication uses the existing same-volume no-replace primitive and full staged/published byte comparison under shared PacedIo. Existing organization stream decoders reject the new staging locator rather than silently granting additional write scope.

A closed no-input lab constructs a referenced library, a real M4A verified comparison, explicit keep-destination decisions and ADR-0076 append projection. It generates video-like opaque bytes, a sidecar, hidden/empty files, a differing source/destination pair and destination-only neighbors. Namespace and native name probes receive durable intent before creation; unknown existing parent directories fail closed. The copy plan and preparation journal live outside source/output trees. Separate action-bound one-use approval proofs authorize preparation, copying and fresh continuation. No input can adopt a real user path or restore an approval from a journal.

Generated interruption coverage includes five preparation gaps, all nine copy boundaries, actual child-process exits at those nine boundaries, full disk, denied journal writes after publication, corrupt stages, late target occupants and stale/wrong-action approvals. Cold recovery reopens SQLite and validates ordered events using the existing catalog-journal decoder, then inspects actual native objects/bytes before a newly approved continuation. Preparation interruption retains uncertain objects and refuses blind replay; general preparation recovery is not implemented by this lab. Destination replacement, production job storage/control/UI, fidelity/capacity acceptance and independent recovery remain separate work.

The instrumented desktop-only `backup_append_fixture` diagnostic accepts no scope and uses a blocking worker. It returns bounded generated-only receipts; production permissions and default features exclude it. The standard desktop suite saves `m4b-append.json` for both OS jobs. This is native engine evidence, not a production transfer UI or proof of remote transports.

The owner subsequently authorized finishing M4B–D, committing, pushing and creating a PR, and selected disposable local protocol servers. That authorization permits continuing the later slices; it does not waive preservation, real OS/transport, recovery or helper-distribution evidence. Stop before M5.
