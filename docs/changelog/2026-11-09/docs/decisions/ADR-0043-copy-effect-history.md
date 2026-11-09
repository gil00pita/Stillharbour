> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0043 — Bind organization output history to its source and policy effect

Date: 2026-10-31. Status: experimental copy-workflow integration; full checks and local Windows native validation pass, macOS CI pending.

## Decision

A new draft UUID does not authorize duplicate output for the same source/template/target mapping. Compute a versioned BLAKE3 effect identity from the catalog/root identity, exact source and destination native objects/volumes, actual template definition, non-lossy source locations and parent objects, file objects/revisions/sizes, relative targets and bundle memberships. Exclude draft/manifest UUIDs, approval/session IDs, output namespace UUIDs, timestamps, template reference version and unrelated catalog revision. This is mapping identity, never a substitute for fresh content comparison or permission.

Schema 6 adds bounded effect-group/candidate indexes to the immutable operation journal after a retained consistent schema-5 snapshot. Atomically bind a new namespace anchor, canonical effect definition and candidate in one catalog transaction. A competing candidate refuses allocation even if the renderer bypasses history lookup. Reopening an untouched review for the same owner/manifest reuses its token; prepared or previously queued operations require retained-history review.

Reconcile legacy visible organization namespaces into the index without altering their scopes, files or approvals. Bound reconciliation and matching to 64 candidates; ambiguity, corrupt anchors and overflow block new output. Preserve multiple historical candidates for explicit review. Do not automatically adopt any existing file or folder.

Expose an ID-only history lookup behind the same experimental feature/private picker boundary. The application shows retained folders/jobs and offers existing read-only inspection with fresh native selection and complete byte comparison. Completed output therefore becomes a fresh-verified no-copy result. Incomplete copy can enter the separately approved ADR-0042 continuation flow. An untouched older draft can be explicitly reopened through its retained manifest. Preparation without a ready stream remains a separate recovery gate.

## Validation and limits

Focused regressions pass same-draft token reuse, cross-draft refusal despite unrelated catalog revision, changed-template separation, legacy indexing without filesystem effects, and actual SQLite candidate denial with full reservation rollback. The actual completed application regression creates a new equivalent draft, refuses a new output review and freshly verifies the original output through a new controller without journal growth. Schema-5 snapshot/collision rollback and the twelve migration process-exit cases pass. Component checks exercise existing-history inspection and foreign-scope refusal. Full/native checks follow; no whole M2/M3 acceptance, import duplicate-content detection, production copying or original mutation enablement is implied.

Complete checks pass 264 frontend and 178 applicable Windows Rust cases, strict/generated IPC, lint/format, denied-warning Clippy and build. Actual Windows UI passes four applicable cases with sixteen macOS skips, including an equivalent draft, fresh existing-output inspection and unchanged independent output/source/neighbor audits. The first new native helper selector assumed only one draft; explicit list selection corrects that test error. Production assets are restored and release exclusions pass. The expanded macOS scenario requires this checkpoint’s own CI.
