# Stillharbour - Codex starter prompt

All repository paths in this document are relative to the repository root.

Implement Stillharbour, the desktop photo organizer described in this repository.

First read AGENTS.md, docs/CODEX_SPEC.md, docs/PRESERVATION_CONTRACT.md,
docs/BACKUP_SYNC_ADDENDUM.md, docs/PRESERVATION_TESTS.md, docs/DECISIONS.md,
docs/ACCEPTANCE_TESTS.md, tasks/M0_FOUNDATION.md, and any existing code
or implementation log. Read docs/RESEARCH.md and docs/SOURCES.md for
rationale where necessary. Use the two images under docs/reference/
as visual inspiration only, not application assets.

Inspect the repository before changing it. Preserve existing useful
code and instructions. Summarize the architecture, M0 scope, safety
invariants, and any actual environment limitations, then implement
M0 only. Do not build every roadmap phase in one pass.

Default stack: Tauri 2, Rust core, React with strict TypeScript, Vite,
and local SQLite. Verify current APIs and pin compatible versions.
Keep the domain/planner separate from filesystem and Tauri adapters.
Use generated fixtures and temporary directories for all tests.
Never scan or mutate my real photo folders as a development shortcut.

M0 must produce a real native shell and typed Rust IPC, foundational
catalog/job/media/filesystem adapters, a documented helper-packaging
plan, a feasibility check for no-replace operations and recoverable
copies, and actual test evidence. Follow the detailed M0 task file.

Do not fake native functionality with browser mocks in production.
Do not silently replace the required architecture with a hosted service.
Do not enable rename, move, cleanup, sidecar writes, or source deletion
before the shared plan/executor/recovery subsystem is implemented and
its later safety gates pass. Do not bypass environment/tool approvals
to make a test run.

Record material decisions as ADRs and actual results in
docs/IMPLEMENTATION_LOG.md. Distinguish commands run, tests passed,
tests failed, and checks not possible in this environment. Never claim
Windows/macOS, codec, backup-transport or signing support from an
untested assumption.

At the end of M0, report files changed, run commands and results,
remaining blockers, and the next milestone. Stop for review before M1.

The v1.1 owner priority is preservation above all else. Do not enable real
source mutation without exact-version independent recovery evidence. The
future Backup & Sync workflow defaults to safe one-way copy, not deletion
mirroring. This does not expand this prompt's M0-only authorization.

## Specification v1.2 note

Read docs/LIBRARY_FEATURES_ADDENDUM.md for future feature constraints; this starter still authorizes M0 only. Do not implement N1-N5 just because they are documented. Check the bundled SQLite patch requirements and keep catalog/edit/import data contracts extensible without scaffolding fake providers.

## Selected name - specification 1.2.1

Use **Stillharbour** exactly as spelled. Read docs/PRODUCT_IDENTITY.md for the descriptor, unchanged supplied logo and catalog filename convention. Keep existing persisted identifiers and user data intact; this naming revision does not authorize migrations or expand the milestone approved by this prompt.
