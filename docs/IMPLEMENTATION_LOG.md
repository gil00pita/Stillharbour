# Implementation log

Records after the current-documentation cleanup belong here. The complete earlier
history, including failed checks and incomplete gates, is preserved in the
[2026-11-09 archive](changelog/2026-11-09/docs/IMPLEMENTATION_LOG.md).

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
