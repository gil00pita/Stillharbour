# Stillharbour - M0 - Foundation and feasibility

## Objective

Establish a runnable native shell and validate the decisions that could invalidate the architecture. This is not permission to mutate any real photo library.

## Read

AGENTS.md; docs/CODEX_SPEC.md sections 1-5, 17-22; docs/DECISIONS.md; docs/ACCEPTANCE_TESTS.md; existing repository code and instructions.

## Work

Inspect rather than overwrite any existing application. Establish the Rust workspace, frontend package, strict types, formatter/linter/test setup and a reproducible package manager lockfile. Use a minimal native window and native directory-picker round trip to prove IPC.

Create an architecture boundary for pure domain/planning code, a local catalog adapter, platform filesystem adapter, metadata/media-worker adapter, job scheduler, backup adapter and Tauri IPC layer. Do not create dozens of empty crates; begin with clear modules and extract crates when justified.

Spike SQLite migrations and a consistent backup. Spike a generated image's safe metadata extraction, thumbnail, orientation/color policy and bounded decoder execution. Establish the packaged path for ExifTool, restic/rclone and the intended RAW/HEIC providers on each release architecture. Record unavailable native environments or codecs as unverified, not supported.

On disposable paths, identify and test native no-replace publication, file identity, hard-link detection and path/volume capability queries. Demonstrate that the proposed operation journal can record intent and recover a interrupted non-destructive copy. Do not enable actual organization yet.

Check current dependency APIs, advisories, binary checksums and license notices. Select a viable macOS minimum and document the release matrix. Prototype desktop WebdriverIO test instrumentation and ensure a production build excludes it. Make a small MapLibre/virtualization spike if needed; never depend on public demo tiles as production infrastructure.

Probe a local disposable restic repository and the rclone integration boundary. Plan real protocol-server tests for M4; do not claim remote protocols are implemented because the executables run.

## Deliverables and gate

A runnable development shell, generated-fixture smoke tests, docs/decisions/ADR-0001-stack.md, dependency/helper inventory, documented development/test commands and an evidence entry in docs/IMPLEMENTATION_LOG.md. Relevant acceptance: AC-001, AC-013 through AC-015, a feasibility subset of AC-027/AC-037, and the testing/distribution plan for AC-077 through AC-080.

Report exactly which native environments and checks ran. Safety-critical unresolved primitives block future mutation enablement. Stop here; M1 is the next separate task.
