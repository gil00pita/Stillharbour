# ADR-0026 — Complete folder-name review and retained companion decisions

Date: 2026-10-31. Status: implemented planning prerequisite; execution remains gated.

## Decision

A saved draft offers explicit metadata-only inventory of the selected photo's direct native folder, including hidden/unindexed entries independently of gallery filters. IPC accepts catalog IDs and source revisions; private native locators bind opaque entry IDs. Lossy names never become execution inputs. Root, parent, source and catalog identities/revisions are checked around observation. Missing/replaced/inaccessible roots fail rather than returning an empty folder.

Bound one concurrent inventory with the destination-review gate and shared interactive metadata reservation. Observe at most 4,096 native and 4,096 indexed entries; bound private locator storage to 8 MiB. Truncation, unreadable metadata and unobserved indexed companion candidates prevent completeness. This bounded direct-folder filename inventory is not recursive inventory, content integrity or provider-original fidelity. Resource reservations are estimates, not physical metadata-I/O rate guarantees.

Windows files use by-ID attribute-only handles and exact-name checks before/after opening. macOS uses directory-relative O_EVTONLY/no-follow handles. Links, special files and cloud placeholders are exposed without following/downloading. Ordinary source content readers retain their access policy. Windows regressions prove content access is denied. macOS regressions verify event-only flags and metadata queries without reading contents; O_EVTONLY alone does not enforce read denial. See the [XNU open implementation](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/vfs/vfs_syscalls.c). Native regressions must pass on each target platform.

## Confirmation

The user reviews all paginated filenames, acknowledges scope and selects at most 64 regular, representable portable companions. Every possible exact same-stem, full-filename XMP/AAE or AppleDouble companion must be selectable and included; unsupported hints block confirmation. Other regular files can be explicitly included. This conservative heuristic does not infer arbitrary provider relationships or normalize Unicode/case.

Confirmation rebuilds inventory and matches its fingerprint, retained draft source evidence and catalog revision. Pure target mapping preserves sidecar extensions, full-primary-name sidecars and AppleDouble prefix; additional files keep their basename. Unsafe/conflicting targets block the decision. Exact byte totals do not establish capacity/fidelity.

Append a bounded bundle_confirmed journal record containing draft digest, full reviewed public inventory, explicit acknowledgement, selected native locators, source evidence and targets. Schema 3 and draft payload remain unchanged. Bind a record digest and selection idempotency key; a fresh validated retry returns the same decision. Retain at most 16 different decisions per photo/draft. History verifies retained digests without live revalidation.

## Next gate

UI reads filenames only on request, pages 64 entries, preserves choices on errors and rejects stale responses. All confirmations have executionAvailable=false. Draft JSON/CSV exports retain primary-row scope; companion decisions are separate. Future executable plans must explicitly bind decision IDs, reconcile overlapping bundles and revalidate content, native targets, capacity, fidelity and recovery. Never silently select the latest decision or treat imported history as approval. No original write, migration, dependency or production mutation permission is added.

Primary declarations: [Apple open flags](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/fcntl.h), [Microsoft file access rights](https://learn.microsoft.com/en-us/windows/win32/fileio/file-access-rights-constants). Actual validation belongs in the implementation log.
