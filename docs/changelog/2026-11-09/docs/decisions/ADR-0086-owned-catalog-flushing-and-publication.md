> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0086: Owned catalog flushing and Windows publication guards

Date: 2026-11-02. Status: accepted implementation; Windows execution awaits fresh CI.

## Context

PR #15 Foundation job 125352931031 compiled successfully, then failed seven tests. Four portable-catalog cases returned Windows access denied: the completed SQLite output was reopened read-only before `sync_all`, but [FlushFileBuffers requires GENERIC_WRITE](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers). Two bound-catalog migration cases failed snapshot publication with a sharing violation. Their long-lived catalog-parent guard used the read-only directory sharing policy while creating owned migration snapshots. One retained-recovery child exceeded its twenty-second deadline at AfterPrepared; its discarded stderr prevents a reliable root-cause conclusion.

## Decision

Allocate portable output exclusively in the existing private review workspace, retain its read/write handle through SQLite generation and flush that handle. SQLite receives read/write, no-follow access without create authority. Validate ordinary single-link identity again before flushing and independently reopen the result for compiled-schema/UUID validation. On Windows, this owned file shares read/write access for SQLite but denies delete sharing. Existing output files, including empty files, are never adopted or truncated. Source libraries remain read-only.

Use the existing owned-publication directory guard for the validated Windows catalog parent. Its leaf permits write sharing while all ancestors and the leaf continue to deny delete sharing; identity/reparse and exact historical-schema validation remain unchanged. General read-only library guards are unchanged. [CreateFileW sharing rules](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew) require compatible sharing for concurrently open handles.

The test-only retained-recovery supervisor continuously drains bounded stderr using the existing 64 KiB diagnostic collector and allows sixty seconds per child. Every expected fault exit and all subsequent byte/history/no-replay assertions remain required. This improves diagnosis and scheduling headroom; a timeout cause or Windows pass is not inferred from a Mac run.

## Validation and limits

The existing-empty-output regression failed before the repair; the nine export tests pass afterward on macOS. Existing bound-migration and actual seven-boundary retained-child tests remain the platform regression seams. Record fresh full-suite and Windows outcomes in [M5 validation](../M5_VALIDATION.md). No catalog schema, original-file mutation gate, credentials, root identities or source permissions change.

## Windows compile follow-up

The first repaired-head Foundation/transport runs at c3591f7 failed before tests with `error[E0603]: function checked is private` at portable.rs:105 and :262. The existing validator was `pub(super)`; sharing it with the owned export adapter requires `pub(crate)`. Only that internal visibility changes: the module remains crate-private, its attribute checks are unchanged and no renderer command or filesystem authority is added. Fresh Windows CI must validate this correction before accepting the platform repair.
