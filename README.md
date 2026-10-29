# Stillharbour

M1 adds a native read-only library: saved folders, background scans, gallery/grid/text-list browsing, metadata, cached previews and catalog-only favorites, ratings and tags. Original-file operations remain unavailable. The specification package below remains the product's normative roadmap.

```sh
npm ci
npm run desktop:dev
```

See [development commands](docs/DEVELOPMENT.md), [M0 architecture](docs/decisions/ADR-0001-stack.md), [dependency and helper inventory](docs/DEPENDENCIES.md), [skill recommendations](docs/SKILLS.md), and [actual implementation evidence and remaining gates](docs/IMPLEMENTATION_LOG.md). A Rust toolchain and native Tauri prerequisites are required. See [M1 validation and platform gates](docs/M1_VALIDATION.md). M2 remains separately gated.

## Original specification package

Created: 2026-10-20
Package version: 1.2.1
Name updated: 2026-10-26
Status at package creation: specification and research only. The sections below preserve the original roadmap/history; current implementation evidence is linked above.

## Product identity

**Stillharbour**  
_A home for the moments you keep._  
Descriptor: **Photo & Video Library**

The owner has selected this name, using the British spelling **harbour**. This 1.2.1 revision updates branding only: the product requirements, preservation rules, milestone gates and 187 specified acceptance/failure scenarios remain in force. Trademark/domain availability has not been established.

See [Product identity](docs/PRODUCT_IDENTITY.md) for naming conventions and [the supplied SVG logo](assets/branding/stillharbour-logo.svg) for the unchanged user-provided mark. Catalog examples now use `Family.stillharbour`. This documentation update does not rename any existing user file or migrate an application.

## Recommendation

Build a local-first Windows/macOS application with a Rust core, Tauri 2 desktop shell, React/TypeScript interface, SQLite catalog, MapLibre map view and established media/backup adapters. Rust is a good fit, not a universal "fastest language" guarantee.

The product's central promise is reviewed, recoverable organization of real photo folders. Originals stay in ordinary folders. Scans do not rewrite them. Organization/import/duplicate cleanup use one durable plan-and-execute system. Backup uses versioned snapshots with a tested restore path, not destructive synchronization.

## Package contents

| File                                                                         | Purpose                                                                                                                                   |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| [CODEX_SPEC.md](docs/CODEX_SPEC.md)                                          | Full product/engineering specification: behavior, architecture, data model, IPC, safety, platforms, performance targets and release gates |
| [AGENTS.md](AGENTS.md)                                                       | Persistent repository instructions for implementation agents                                                                              |
| [CODEX_START_PROMPT.md](docs/CODEX_START_PROMPT.md)                          | Copy-paste prompt for the first approved implementation milestone                                                                         |
| [docs/RESEARCH.md](docs/RESEARCH.md)                                         | Official-source market review and prioritized feature recommendations                                                                     |
| [docs/SOURCES.md](docs/SOURCES.md)                                           | Dated primary-source register                                                                                                             |
| [docs/DECISIONS.md](docs/DECISIONS.md)                                       | Proposed defaults, owner decisions and feasibility gates                                                                                  |
| [docs/ACCEPTANCE_TESTS.md](docs/ACCEPTANCE_TESTS.md)                         | User-requirement traceability and 85 acceptance/failure scenarios                                                                         |
| [docs/IMPLEMENTATION_LOG.md](docs/IMPLEMENTATION_LOG.md)                     | Initially empty evidence log; Codex records real outcomes here                                                                            |
| tasks/M0__.md through tasks/M6__.md                                          | Seven bounded implementation milestones                                                                                                   |
| [templates/organization_examples.json](templates/organization_examples.json) | Data-only example presets for the proposed template DSL                                                                                   |
| docs/reference/                                                              | The two user-supplied visual references and usage notes                                                                                   |

## Start in Codex

Place the package contents at the repository root, keeping the `CODEX_*.md` files in `docs/` and `AGENTS.md` at the root. For an existing repository, merge instructions deliberately rather than overwriting an existing AGENTS.md or codebase. Give Codex the contents of docs/CODEX_START_PROMPT.md.

The first prompt authorizes M0 only: a real shell, feasibility checks, foundational adapters and evidence. After reviewing its output, the next task is tasks/M1_READ_ONLY_LIBRARY.md. Continue through M6 in order, approving each milestone separately. The smaller first deliverable is intentional: a working read-only library precedes any real original-file mutation.

OpenAI documents AGENTS.md as recognized repository guidance; the other document names and phase structure are helpful conventions, not a special mandatory Codex specification language. [R34 in docs/SOURCES.md]

## First-release scope

M0 foundation; M1 read-only library/timeline/list; M2 previewed organization and verified import; M3 duplicates/health/storage; M4 backups and restore; M5 map/metadata/format completeness; M6 hardened packaged release.

Every requested feature is mapped in docs/ACCEPTANCE_TESTS.md. ID3 is explicitly treated as not applicable to photo-only matching rather than promised as a meaningless toggle. Perceptual similarity, local AI and mobile/cloud synchronization are later additions unless the owner changes scope.

## Important limits

At specification-package creation, no application code had been built or benchmarked. Performance numbers are proposed targets. No native Windows/macOS test, decoder coverage, transport restore test or signing operation has been performed here. Those are implementation gates, not completed claims.

The initial proposed mandatory targets are Windows 11 x64 and Apple Silicon macOS. The exact macOS minimum and an Intel Mac build require native verification. The commercial/license model and map provider remain owner/implementation decisions. The selected product name is Stillharbour; public brand clearance is still outstanding.

The screenshots are reference material only. Keep them out of shipped application resources and do not use their photos as the application's sample library.

## Version 1.1 update - 2026-10-23

The owner made prevention of file loss/damage the highest-priority requirement. A working name was proposed at this stage; the later owner selection of Stillharbour is recorded in the 1.2.1 naming revision.

New files:

| File                                                                                 | Purpose                                                                                                   |
| ------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| [docs/PRESERVATION_CONTRACT.md](docs/PRESERVATION_CONTRACT.md)                       | Overriding no-loss/recovery requirements, safe defaults, fidelity checks and honest limitations           |
| [docs/BACKUP_SYNC_ADDENDUM.md](docs/BACKUP_SYNC_ADDENDUM.md)                         | Default-library compare/review/sync, retained destination versions, contextual errors and transfer limits |
| [docs/PRESERVATION_TESTS.md](docs/PRESERVATION_TESTS.md)                             | 40 additional acceptance/failure scenarios; 125 total with the 85 original scenarios                      |
| [docs/BACKUP_SYNC_SOURCES.md](docs/BACKUP_SYNC_SOURCES.md)                           | Primary sources consulted for this extension                                                              |
| [CODEX_BACKUP_SYNC_PROMPT.md](docs/CODEX_BACKUP_SYNC_PROMPT.md)                      | Staged implementation prompt for an existing repository, with prerequisite checks                         |
| [templates/safe_sync_profile.example.json](templates/safe_sync_profile.example.json) | Proposed, data-only profile settings; not an implemented API/config                                       |
| [docs/SPEC_CHANGELOG.md](docs/SPEC_CHANGELOG.md)                                     | Record of specification changes, distinct from application implementation evidence                        |

Read the preservation contract before implementation. It strengthens any weaker original default, including source-moving organization. Existing spec/history/reference files are retained. The original v1.0 ZIP is not modified. This update is specifications only: no application build, native integration test, data-loss test or benchmark is claimed.

## Version 1.2 update - 2026-10-26

New library features and cloud-import design. Main guide: docs/LIBRARY_FEATURES_ADDENDUM.md. Detailed contracts: docs/LIBRARY_FEATURES_DATA_MODEL.md. Provider evidence: docs/LIBRARY_FEATURES_SOURCES.md. Additional proposed tests: docs/LIBRARY_FEATURES_TESTS.md.

docs/CODEX_LIBRARY_FEATURES_PROMPT.md starts N1 only after prerequisite inspection. tasks/N1 through N5 define read-only integrity/recent browsing, catalog/tags, recipe editing, staged imports and future mobile editing. templates/edit_recipe.example.json and templates/cloud_import_manifest.example.json are design examples, not implemented configurations.

The six requests are included, but platform scope remains desktop-first. Original-safe edits/tags live in the catalog. Catalog files do not contain original media. Large imports begin with provider exports; limited APIs must disclose missing metadata/renditions and cannot claim a full original archive. Existing preservation/sync requirements remain binding.

The original specification package contained no application source or test execution evidence. Its structural validation report is docs/SPEC_VALIDATION.md. Previous packages are unchanged.

## Version 1.2.1 naming update - 2026-10-26

Stillharbour is now the selected app name throughout the main specification, repository guidance, all three Codex prompts, document titles and catalog examples. The original v1.2 source archive has not been modified. Research dates and earlier feature-version labels are retained; they do not indicate new verification of external services.

| File                                                            | Purpose                                                              |
| --------------------------------------------------------------- | -------------------------------------------------------------------- |
| [Product identity](docs/PRODUCT_IDENTITY.md)                    | Name, descriptor, tagline, catalog example and safe adoption rules   |
| [Supplied logo](assets/branding/stillharbour-logo.svg)          | Exact SVG supplied by the owner, without redrawing or changing paths |
| [Starter prompt](docs/CODEX_START_PROMPT.md)                    | M0-only implementation entry point                                   |
| [Backup & Sync prompt](docs/CODEX_BACKUP_SYNC_PROMPT.md)        | Prerequisite-gated backup extension                                  |
| [Library feature prompt](docs/CODEX_LIBRARY_FEATURES_PROMPT.md) | N1-only extension entry point                                        |
| [Package validation](docs/SPEC_VALIDATION.md)                   | Checks of this documentation package, not tests of an application    |

No feature has been implemented, removed or newly approved by this naming revision.
