# Development

Read [build capabilities](CODEX_SPEC.md), [architecture](ARCHITECTURE.md), the
[preservation contract](PRESERVATION_CONTRACT.md) and [AGENTS.md](../AGENTS.md)
before changing native behavior. Use disposable generated media for tests.

## Setup

CI uses Node.js 24 and Python 3.13. The repository declares Node.js >=22.12.0,
npm 11.16.0 and Rust 1.99.1. Use the pinned npm package manager and Rust toolchain.
Install the native [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
for the target platform; Python 3 is also used by helper/fixture tooling.

```sh
npm ci
node scripts/backup-helpers.mjs --download
node scripts/backup-helpers.mjs --check
npm run desktop:dev
```

The helper script verifies pinned archive/executable checksums. Do not disable
TLS or checksum checks or replace a changed helper without review. Helpers and
download caches are ignored and must be prepared in fresh workspaces. See
[dependencies](DEPENDENCIES.md) for manifests and notices.

`desktop:dev` starts the native app and Vite. It uses `CONDUCTOR_PORT` when set,
otherwise port 1420. Debug catalog data is isolated under `.context/native-data`;
do not automate a running personal-catalog development session. A frontend-only
`npm run dev` cannot provide native catalog or filesystem behavior.

## Commands

| Command                                                      | Purpose                                                                                                      |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| `npm run desktop:dev`                                        | Native development application                                                                               |
| `npm run build`                                              | Strict frontend compilation and production frontend output                                                   |
| `npm run typecheck` / `npm run lint`                         | TypeScript and ESLint                                                                                        |
| `npm test -- <path>`                                         | Focused Vitest cases                                                                                         |
| `npm run bindings:check`                                     | Generated Rust/TypeScript IPC drift                                                                          |
| `npm run check`                                              | Formatting, lint, types, frontend cases, release/dependency policy, bindings, Rust matrix and frontend build |
| `npm run desktop:test:build`                                 | Instrumented debug native app with generated-fixture features                                                |
| `npm run desktop:test`                                       | Default native WebView suite                                                                                 |
| `npm run desktop:test:catalog`                               | Separate three-process catalog activation/recovery suite                                                     |
| `npm run desktop:test:formats`                               | Separate native format/viewer suite                                                                          |
| `npm run desktop:test:map` / `npm run desktop:test:metadata` | Isolated generated map/metadata harnesses                                                                    |
| `npm run desktop:test:albums`                                | Isolated generated Albums harness                                                                            |
| `npm run desktop:build`                                      | Normal unsigned macOS `.app` bundle; not a Windows installer command                                         |
| `npm run test:release` / `npm run release:check`             | Policy regressions / production-resource and feature preflight                                               |
| `npm run release:smoke -- <executable>`                      | Built-worker and in-memory SQLite probe without a user catalog                                               |
| `npm run native:cross-volume`                                | Platform-specific generated cross-volume fixtures                                                            |

The scripts in [package.json](../package.json) and [scripts/](../scripts) are the
command reference. Format/performance/transport probes have separate owned
fixture and external-endpoint requirements; historical invocations are preserved
in the [changelog](changelog/README.md), not ordinary setup instructions.

## Validation and CI

Choose checks for changed behavior and consumers, including shared dependencies.
Run `git diff --check` for every change and Prettier on changed supported files.

| Change                                                            | Local validation                                                                                        |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Documentation only                                                | Formatting, local links/anchors, paths, commands and whitespace                                         |
| UI behavior                                                       | Types, lint, affected component/consumer cases and focused native smoke when native interaction changes |
| Shared theme/types, IPC, dependencies or build inputs             | Broader affected consumers, frontend build, IPC drift where relevant and affected native/Rust checks    |
| Rust logic                                                        | Rust formatting, affected Clippy package/targets/features and domain tests/callers                      |
| Filesystem mutations, recovery, catalog migrations or permissions | Applicable preservation, failure/recovery and generated native checks                                   |
| Release or milestone qualification                                | Complete applicable acceptance and both target-platform gates                                           |

Use focused checks while developing. Reserve `npm run check` and complete native/
transport suites for CI, broad changes or explicit qualification. Reuse a passing
result only while its relevant code, dependencies and build inputs are unchanged;
identify reused evidence separately from fresh runs. Rerun after relevant edits
or failures. Never invent a pass or call browser checks native desktop evidence.

Before merging, require these checks for the current head and an up-to-date base:

- `native (macos-15)`
- `native (windows-latest)`
- `transports (macos-15)`
- `transports (windows-latest)`

[Foundation CI](../.github/workflows/check.yml) runs the aggregate, explicit native
preservation opt-ins, desktop/catalog suites and cross-volume checks.
[Transport CI](../.github/workflows/m4-transports.yml) supplies the separate
disposable transport matrix. Keep both platforms, evidence uploads and required
gates; canceled/pending runs are not passes. Live branch protection is a repository
setting, not a consequence of a committed workflow.

## Isolate native automation

Run native drivers serially. Do not rebuild a binary used by another agent or
development session. For an isolated instrumented build on macOS:

```sh
CARGO_TARGET_DIR="$PWD/.context/native-test/target" npm run desktop:test:build
```

Point an ignored WDIO wrapper at that target's executable in both the service's
`appBinaryPath` and capability's `application`. Use absolute spec paths and an
unused driver port. If copying the executable, keep it beside its required helper
resources. Verify the listening PID and executable before activation or shutdown.
Keep copied artifacts, receipts and screenshots under `.context/`.

Instrumented native and production frontend builds share `dist`. Keep builds and
drivers sequential; after an instrumented build, restore ordinary frontend output
with `npm run build`, then run `npm run release:check`. Default preflight must
exclude test routes, WebDriver dependencies and experimental owned features.
Never use `--all-features` for a release.

The default WDIO suite gives each spec a new native process and generated data
tree. Catalog activation uses its separate three-launch harness and retained
receipts; never manufacture a receipt to skip failed preparation. Format probes
use their own configuration and generated/pinned sources. Fixture cleanup must
prove ownership of the exact directory and preserve failed-session evidence.

## Native fixture preconditions

- Embedded select-option clicks do not reliably change controlled selects. Use
  the established DOM value/change helper and assert the resulting selected value.
- Generated managed destinations must finish discovery and the exact root's
  `thumbnail:initial:<rootId>` job before building a preview. The readiness helper
  also requires online/prepared roots and no active scan/thumbnail work. Keep
  stale-preview, identity, overlap and neighbor/source checks; do not force-enable
  the chooser or loop through approvals to hide invalidation.
- Move hover and keyboard focus away from thumbnails before checking resting
  opacity. The pinned embedded driver's `pointerMove` dispatches DOM events and
  does not move the OS cursor. On Windows the page-chrome fixture uses Tauri's
  native cursor API with logical window coordinates and deliberately asserts
  hover before leaving. Non-Windows hosts retain the embedded-driver setup;
  macOS cursor warping alone does not deliver a mouse-moved event to WKWebView.
  Wait for generated preparation to settle before measuring stable tiles. The
  page-chrome readiness helper accepts `completed` or `partial` for both discovery
  and the exact root's initial-thumbnail job: intentionally invalid media can make
  both jobs partial on Windows. Require an online, prepared root and no unfinished
  or failed jobs. This UI fixture rule grants no integrity or mutation authority
  and does not relax the stricter managed-destination checks above. Setup writes
  a path-free root/job receipt even when its readiness wait times out.
  Cursor movement is permitted only by `tauri.test.conf.json` instrumentation;
  production capabilities are unchanged. Keep exact visible/resting states and
  menu/hit geometry; do not disable animation or accept partial opacity to pass
  a test. CI retains page-chrome selection and menu-position receipts/screenshots.
- A hidden/unfocused WKWebView can suspend floating-menu positioning and physical
  focus styles. Record `document.hidden`, `document.hasFocus()` and actual DOM
  focus separately. DOM activation is not proof of physical keyboard/pointer input.
- Use the existing focus-first/DOM scroll setup where the pinned embedded driver
  reverses click/focus ordering or stalls wheel/animation-frame scrolling. Do not
  weaken viewport containment or preservation assertions.

## Documentation changes

Keep current guidance in the pages indexed by [docs/README.md](README.md). Describe
implemented behavior and distinguish nondefault experimental features from enabled
capabilities. Keep unresolved safety/release gates in [Release status](RELEASE_STATUS.md).

Append a concise dated record to [IMPLEMENTATION_LOG.md](IMPLEMENTATION_LOG.md)
with the request, scope, decisions, actual checks, limitations and next gate.
Architecture changes receive an ADR under `docs/decisions/`; existing numbered
records are preserved in the changelog and must not be reused.

Superseded plans, prompts, task lists and long dated evidence belong in
`docs/changelog/`. Preserve their limitations and distinguish them from current
instructions. Update incoming/outgoing links when moving a page. The archive is
excluded from automatic formatting so historical text does not receive unrelated
rewrites. Archiving does not waive an acceptance obligation.

## Optional editor setup

`.vscode/` contains extension recommendations, formatting settings and the owner's
optional MCP definitions. These are editor configuration, separate from the pinned
application toolchain. Availability and authentication depend on the editor;
checking in an entry does not start or connect a server.
