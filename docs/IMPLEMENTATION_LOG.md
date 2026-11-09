# Implementation log

Records after the current-documentation cleanup belong here. The complete earlier
history, including failed checks and incomplete gates, is preserved in the
[2026-11-09 archive](changelog/2026-11-09/docs/IMPLEMENTATION_LOG.md).

## 2026-11-09 — Accept settled partial discovery in the page-chrome fixture

**Request:** fix Windows run 42649858582, which timed out in page-chrome setup
before reaching the hover checks. The other 20 desktop specifications passed.

**Cause and scope:** the previous readiness predicate required successful
discovery while allowing partial thumbnails. The generated Windows library's
uploaded status records both jobs as `partial`, each with two invalid-media
errors. Extract the predicate into `photo-controls-ready.mjs`; accept `completed`
or `partial` for discovery and the same online root's exact initial-preview job.
Still reject missing, queued, running, paused, canceled, interrupted, failed or
mismatched preparation. Keep the 45-second setup wait, native cursor fix, exact
control/menu assertions and production/preservation gates. Save a path-free
readiness/root/job receipt on setup success or timeout, using the existing CI
artifact directory. Update developer and acceptance guidance.

**Validation:** the old predicate fails five new regression cases, including
partial discovery. All 15 readiness regressions and nine existing stricter
managed-destination regressions pass with the repository Vitest runner. Replaying
the actual downloaded Windows status returns ready while retaining both partial
states and error counts. A fresh native macOS page-chrome run passes both cases,
both widths and menu geometry, with all 32 generated source files unchanged.
Reuse the instrumented executable from the preceding entry: application,
dependencies and native build inputs are unchanged. Scoped ESLint, JavaScript
syntax, changed-file formatting, documentation links/anchors and whitespace pass.
Artifacts are under ignored `.context/windows-preparation-repair/`.

**Limits and next gate:** no native Windows execution is available locally;
require fresh hosted Windows verification and all four current-head
native/transport checks before merge. Earlier production build/preflight evidence
is reused because no production code, dependencies or build inputs changed.

## 2026-11-09 — Clear native Windows hover before measuring photo controls

**Request:** fix the attached Windows CI failure in the page-chrome selection
specification. The run passed 20 desktop specifications; this specification failed
at the initial resting-opacity check.

**Cause and scope:** the pinned embedded driver's `pointerMove` dispatches DOM
events without moving the OS cursor. Search could have focus while the first
thumbnail remained natively hovered. Use Tauri's native cursor command with
logical window coordinates on Windows, authorized only in the instrumented
`tauri.test.conf.json`. Deliberately establish native hover before asserting the
exact hidden state, and verify hover and DOM focus have actually left. Keep the
existing non-Windows driver setup, exact opacity/geometry assertions and timeouts.
Wait for generated discovery/preparation to settle before measuring tiles;
intentional corrupt fixtures may finish thumbnails as `partial`. Include selection
and menu-position receipts in CI artifacts, and distinguish native hover evidence
from the still-synthetic context-menu event. Update developer/acceptance guidance.
No product behavior, production capability or preservation gate changes.

**Validation:** a browser reproduction fails with driver-style DOM movement and
passes with native pointer movement. An isolated native macOS reproduction of
the old helper fails with the exact CI error: search focused, unchecked tile,
`:hover` true and opacity `1`. Initial universal cursor-warp probes did not
establish WKWebView hover reliably, so native movement is scoped to Windows.
An initial preparation predicate incorrectly required successful thumbnails for
deliberately corrupt fixtures; the final predicate explicitly accepts terminal
`partial` thumbnail outcomes while requiring completed discovery. The final
focused native macOS page-chrome run passes both cases, including both window
widths, menu geometry and the unchanged 32-file generated-source audit.
Instrumented build, 12 release/dependency policy cases, scoped ESLint, script
syntax, formatting, 1,340 local links/images and 60 anchors, production frontend
build and production-exclusion preflight pass. Whitespace checks pass. Artifacts
are under ignored `.context/windows-controls-repair/`; the native executable hash
is `ac9e499b3fc939981dfb82a722aad32e0b3831bc34ff6249c1ae7d949e38ab78`.

**Limits and next gate:** Windows native execution is unavailable on this macOS
host; the Windows-specific cursor/hover path requires fresh hosted CI. Browser
reproduction and the macOS pass are not Windows acceptance. Require all four
current-head native/transport checks before merge; release gates remain open.

## 2026-11-09 — Separate current documentation from previous plans

**Request:** keep documentation for what is built and move previous plans into a
changelog folder.

**Scope:** archive all 200 previous documentation/task files and a copy of the
project README, product and design notes under `docs/changelog/2026-11-09/`, preserving the original
`docs/`/`tasks/` layout, dated evidence, numbered ADRs, requirements and reference
images. Mark archived pages as historical and rebase their repository links.
Replace the main documentation with current capabilities, user/developer guides,
architecture, dependencies, format limits, verification and release status.
Keep the preservation contract's substantive rules. Update README, the agent read
order and replace the historical HTML design prompt with a current-docs pointer;
retain its original text in the snapshot. Historical snapshots are excluded
from automatic formatting; current docs use the normal formatter.

**Decisions:** describe default and experimental builds separately; retain all
unmet preservation/release gates. Remove obsolete roadmap instructions from the
current read order and stale user guidance, including the claim that no online
map provider is configured. Previous reports remain evidence only for their
recorded inputs. No product, permission, dependency or execution gate changes.

**Validation:** repository-wide `npm run format:check` and whitespace pass.
Validate 1,340 local links/images and 60 Markdown anchors across 224 Markdown
files: none are broken. Archive comparison preserves all 203 source-file payloads
(allowing only banners/link rebasing); reference images are byte-identical and
all ten numbered preservation rules are unchanged. Documented npm script names
exist, and default-feature/mutation claims match the code. The HTML is identical
after stripping comments. Application/native suites are not rerun for this
documentation-only change; no new runtime or release evidence is claimed.

**Next gate:** review the documentation diff. Existing application CI and release
requirements remain binding; this cleanup is not new native or release evidence.
