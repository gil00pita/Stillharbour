> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0088: Remove vulnerable development dependency paths

Date: 2026-11-04. Status: implemented; current Windows execution remains required.

## Context

M6's initial audit records 33 affected development dependency entries from four underlying advisories. Production npm dependencies report zero findings. The pinned Tauri WebdriverIO service retains WebdriverIO 9 utilities alongside the application's WebdriverIO 10 tools; the older utility resolves a nested browser helper with vulnerable `extract-zip`. Updating only the top-level browser helper would leave that path intact. Automatic driver downloads are disabled in native test configurations, but the development supply chain still needs remediation.

## Decision

Keep application, framework and native-helper pins unchanged. Add exact npm overrides for `glob 14.0.3`, `minimatch 11.0.4` and `@puppeteer/browsers 3.2.6`, retaining the existing `basic-ftp 6.2.3` override. Commit the resulting lockfile with the change. The new browser helper removes `extract-zip`; its Node requirement, 22.12.0 or newer, matches this repository's existing floor. It is ESM-only and removes `makeProgressCallback`; inspection confirms the pinned WebdriverIO utility uses neither CommonJS loading nor that removed API. Tests import the actual consumer and verify all seven browser-helper exports it uses.

The glob/minimatch majors preserve the APIs actually consumed here: named `glob`, CommonJS `glob.sync`, and `Minimatch`. Retain Unicode, brace, exclusion and recursive matching checks through the installed archive utility as well as native spec discovery. Treat matching compatibility as an acceptance requirement; do not bypass failures with incompatible major downgrades or audit ignores.

The lock update adds 12 package paths, removes 75 and changes three retained versions (glob, minimatch and path-scurry). Every production package path, version, source URL and integrity pin is independently compared unchanged. These are development tools, not additional desktop runtime frameworks or bundled helpers. The updated glob/minimatch packages declare BlueOak-1.0.0; distribution notice/legal review remains a separate gate.

## Evidence and consequences

2026-11-04 CI clarification: default browser unpacking prefers platform-native tools; its behavior must not be equated with the patched JavaScript ZIP fallback. The regression directly invokes the resolved helper's `extractZipWithYauzl`, requires its actual containment cause for three escape forms, and separately tests benign regular ZIP extraction through both dispatch paths. Current-main CI exposed the earlier universal-rejection assumption on both hosts and a native-separator expectation on Windows. Preserve that failure and the earlier local evidence; do not certify native hostile-archive containment from the corrected fallback test. No pin, browser-download policy or production capability changes. See M6 dependency evidence for actual results.

Subsequent clean-runner evidence requires an explicit test-only `yauzl 3.4.0` peer rather than relying on an optional ancestor-installed parser. Add that exact dev pin and locked `pend 1.2.0` (both MIT), and verify the actual helper's peer resolution stays in this checkout. All 183 production package records remain unchanged. This narrowly completes the test fixture's dependencies; it does not add a runtime archive-import feature or certify native extraction. The earlier no-pin-change statement describes the initial correction checkpoint, before the missing-peer CI failure.

Before the overrides, the generated escaping-ZIP-symlink fixture fails because extraction resolves successfully; cancellation also fails with nine directory reads after a first-read abort. Afterward both regressions pass, including unchanged outside-neighbor bytes and exactly one directory read. Two compatibility cases also pass. Tests resolve the browser helper through the Tauri service's actual WebdriverIO utility, preventing a patched top-level copy from hiding a vulnerable nested copy. The ZIP fixture and its sibling destination are contained in a test-owned temporary root.

Fresh full and production-only npm audits report zero findings; the full pre-change audit is retained. No advisory ID is ignored. RustSec review is separately refreshed using locally installed, pinned `cargo-audit 0.22.2`; it adds no application dependency. Commands, exact database identity, informational warnings and current native/build validation belong in [M6 dependency evidence](../M6_DEPENDENCY_HARDENING.md). Successful local tests do not establish fresh Windows, signed-package or clean-machine acceptance.

Alternatives rejected: force npm's suggested WebdriverIO downgrades; replace the native test framework; maintain a bespoke ZIP extractor; suppress the advisories because tools are development-only; or update only the already patched top-level browser helper. The chosen overrides remove the vulnerable paths while retaining the existing desktop-test protocol and application behavior.

Primary references: [glob 14.0.2 security release](https://github.com/isaacs/node-glob/releases/tag/v14.0.2), [minimatch 11.0.1 security release](https://github.com/isaacs/minimatch/releases/tag/v11.0.1), [Puppeteer browser-helper changelog](https://github.com/puppeteer/puppeteer/blob/main/packages/browsers/CHANGELOG.md), and the installed consumer/package manifests. The changelog documents the browser helper's ESM/Node/API changes, replacement of extract-zip and subsequent ZIP symlink hardening.
