# Stillharbour - Codex prompt: library feature extension

All repository paths in this document are relative to the repository root.

Extend Stillharbour with the library features below. Inspect the repository before changes. This package is specifications only; do not assume the earlier desktop app, preservation engine or any provider connection has already been built.

Read AGENTS.md, docs/CODEX_SPEC.md, docs/PRESERVATION_CONTRACT.md, docs/LIBRARY_FEATURES_ADDENDUM.md, docs/LIBRARY_FEATURES_DATA_MODEL.md, docs/LIBRARY_FEATURES_TESTS.md, docs/LIBRARY_FEATURES_SOURCES.md, tasks/N1_LIBRARY_HEALTH_AND_RECENT.md, and the existing implementation log.

Implement N1 only, integrated into existing modules. If the M1 catalog/query/scanner/job prerequisites do not exist, stop at a gap report and identify the smallest earlier milestone needed; do not create a mock success path or silently authorize a full app build. A fresh repository should start with docs/CODEX_START_PROMPT.md instead.

N1 provides read-only inventory/content reporting and Recently added. Define untracked, missing, offline, inaccessible, checksum mismatch and no-baseline separately. Preserve baseline history and added-date provenance. Missing status requires successful authoritative enumeration. Never auto-repair, overwrite, delete or silently reset a baseline.

Use fixtures only. Capture manifests of input files before/after auditing, rescanning, export and failure scenarios. Reuse global I/O pacing. Provide component tests and backend tests for N1's defined acceptance scenarios, including partial/offline results and same-size changed files. Export reports only through a reviewed new-file path. Treat inability to run native tests as an unverified gate, not a pass.

Do not implement cloud sign-in, new mobile platforms, embedded metadata writes, automatic schedule creation, source mutations or mass feature scaffolding in this task. Keep N2-N5 as separately approved follow-ons. Record actual changed files, commands/results, remaining gaps and current source-version checks in docs/IMPLEMENTATION_LOG.md. Preserve prior evidence and stop at the N1 gate.

## Selected name - specification 1.2.1

Use **Stillharbour** exactly as spelled. Read docs/PRODUCT_IDENTITY.md for the descriptor, unchanged supplied logo and catalog filename convention. Keep existing persisted identifiers and user data intact; this naming revision does not authorize migrations or expand the milestone approved by this prompt.
