# Stillharbour - N1 - Integrity report and Recently added

Prerequisites: persistent M1 catalog/asset identity, root authorization, scanner, typed errors and job budget; baseline primitives from M3 or a bounded read-only extraction of those primitives. No source mutation required.

Read addendum sections 2-3 and data model. Extend existing Integrity and Photos modules, not a second backend. Implement audit inventory observations, orthogonal statuses, baseline history, reviewable findings, complete/partial coverage, safe report export, first-added semantics and indexed keyset browsing. Basic triage is allowed; restore goes through existing protected operations rather than an ad hoc repair button.

Required scenarios: LF-001 through LF-017, LF-060 and applicable AC/PS tests. Use generated fixtures; no real library/provider access. Validate list view does not fetch thumbnails and date changes do not rewrite metadata. Actual timings need measurement, not targets labeled passed.

Gate: source-preserving audit and recent sorting work end-to-end, no offline-to-missing false claims, no baseline erasure, tests/evidence recorded. Do not start N2-N5 without approval.
