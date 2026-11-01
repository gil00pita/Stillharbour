# ADR-0030 — On-demand copy capacity estimate and fidelity scope

Date: 2026-10-31. Status: read-only M2 review; approval and production copying remain unavailable.

## Decision

Add organization_copy_assessment with retained plan/manifest IDs only. Load and validate the immutable records, recompile exactly their chosen companion decisions through fresh inventory observations, and require the same manifest identity/digest. Repeat destination identity/volume/overlap/target review against the saved destination fingerprint. Query allocation granularity, available bytes and total bytes from one fs2 0.4.3 statvfs response. Rebind the selected destination and independently open its resolved directory chain afterward; reject catalog revision changes during review. Windows short-name picker spellings follow the existing bind-then-resolve procedure rather than weakening exact native child checks.

No source content, probe file, reservation, imported approval, executable path or arbitrary renderer destination is involved. This report does not modify the saved draft/manifest or grant execution authority. Existing immutable digests and catalog schema remain unchanged. Fresh observations remain sequential and can become stale; execution needs fresh validation and separate approval.

## Explicit estimate

The versioned retained-source-copy-estimate-v1 policy bounds 1–15,600 files and 1,024 output directories including the pending output root. Validate paths and exact manifest byte totals. Sum logical lengths with u128 and round each file separately to the observed allocation unit. Empty files contribute zero data allocation. Staging/publication rename the same output objects, so this policy counts their data once and retains source storage. It assumes neither sparse/compressed layout preservation nor reflink savings.

Show each allowance: 64 KiB per output file/directory for filesystem metadata, 32 MiB for journal/planning work and headroom equal to the greater of 64 MiB or ten percent of rounded data. Estimated peak additional space is rounded data plus these allowances. They are declared planning estimates, not filesystem upper bounds or guarantees. Available space already excludes occupied retained stages, but later writers, quotas and filesystem behavior may change it. Report sufficient_for_estimate or insufficient_for_estimate; insufficient observations block future copying. No space is reserved. Future executor integration must recheck capacity, including application-journal volume capacity, retained versions and any different operation policy, rather than treating this review as a ready-plan gate passing.

All byte quantities cross IPC as decimal strings. Invalid/unknown allocation units and inconsistent space/scope fail with typed errors. Large totals never pass through JavaScript Number.

## Fidelity and interface

Report file bytes as verification required and confirmed companions as selected separate files. Explicitly mark timestamps, permissions/ownership, extended attributes/resource forks, alternate data streams and sparse/compression layout as not preserved or verified by the current copy engine. This does not assert those metadata are absent. Retained originals are not independent verified backup. No source removal follows from a byte-only copy report.

The compiled-manifest view offers Check copy space and fidelity on demand, with busy, retryable error, insufficient-space, exact breakdown and fidelity table states. Reject a foreign plan/manifest/digest response. Changing the draft/companion scope removes the report and late unmounted responses are ignored. The report never enables a copy button.

## Validation and remaining gate

Pure regressions cover per-file rounding, zero-length files, exact large totals, allowance breakdown, the precise low-space boundary, bad paths/sums and unsupported allocation units. Native library fixtures cover actual stats, immutable identity binding, changed companions and occupied destinations. Component regressions cover ID-only requests, no automatic work, precision, focus, foreign reports, errors/retry, insufficient space and disposal. The existing Windows desktop scenario records windows-copy-assessment.json plus a screenshot within its independently preserved generated source/destination scope. Full check and actual platform results belong in the log.

No dependency version, binary, schema or persisted identity changes. Reviewed pinned fs2 source supplies the existing wrapper. Primary native declarations: [Microsoft cluster units](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdiskfreespacew) and [Apple filesystem statistics](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/statfs.2.html). M2 approval, production executor/catalog/job/locks, import, undo and native recovery gates remain open. M3 follows M2 acceptance.
