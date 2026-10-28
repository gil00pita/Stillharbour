# Skills useful for Stillharbour

Research date: 2026-10-27. Skills are workflow guidance; AGENTS.md, the preservation contract and milestone scope remain authoritative. No third-party skills were installed globally or vendored into this repository by M0.

Already available in this environment: `impeccable` for accessible desktop-shell design; `react-doctor` for frontend diagnostics; `tdd` for safety-relevant state transitions; `codebase-design` for the planner/adapter boundary; `diagnosing-bugs` for native failures; `playwright` for browser component/visual checks; and the bundled Conductor skill for workspace scripts. Browser checks do not replace native desktop tests.

The find-skills search checked the directory leaderboard, ran `npx skills find tauri` and `npx skills find rust`, read the recommended source SKILL.md files, and checked their GitHub repositories.

| Candidate                                                                                             | Evidence at research time                                                  | Fit and limits                                                                                                                                                                   |
| ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Apollo rust-best-practices](https://skills.sh/apollographql/skills/rust-best-practices)              | 21.2K installs; official Apollo organisation; 119 repository stars         | Strongest addition for ownership, typed errors, clippy and Rust module review. Its generic recommendations do not prove filesystem safety.                                       |
| [Vercel React best practices](https://skills.sh/vercel-labs/agent-skills/vercel-react-best-practices) | About 843K installs; official Vercel organisation; 33,081 repository stars | Useful for M1's virtualized gallery, unnecessary renders and bundle discipline. Apply client React guidance; Next.js/server-rendering rules do not describe this Tauri/Vite app. |

Optional project-local installation commands, after reviewing the chosen source revision:

```sh
npx skills add apollographql/skills --skill rust-best-practices --agent codex
npx skills add vercel-labs/agent-skills --skill vercel-react-best-practices --agent codex
```

The search also found `nodnarbnitram/claude-code-extensions@tauri-v2` (8.1K installs), but its repository had only 16 stars. It is a lower-confidence discovery, not a recommendation on popularity alone. Prefer [official Tauri security documentation](https://v2.tauri.app/security/) and the native test APIs verified in ADR-0001 before adopting it. No generic SQLite skill supersedes the exact bundled runtime, consistent-snapshot and defensive-catalog tests required here.

M1 applied `codebase-design` to keep the library behind the existing catalog/worker boundary, `find-similar-functions` to reuse the IPC and media paths (`rg` fallback because truffler was unavailable), `impeccable` to extend the pinned desktop design and `react-doctor` to check the completed React changes. No additional global skills were installed.
