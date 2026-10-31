# ADR-0024 — Read-only destination target observations

Date: 2026-10-31. Status: implemented continuation, native acceptance remains scoped to actual evidence in the implementation log.

## Context and decision

The owner requests continued work until M2 and M3 finish. M2's identity-only destination preflight cannot distinguish a vacant proposal from an occupied target or a file/link in an ancestor position. Extend that existing picker-session/fingerprint-bound command with bounded metadata-only native lookup, available/total volume bytes and per-row observations. Retain all draft/executor/recovery blockers.

On Windows, share the existing fresh-handle NtQueryDirectoryFile parser while separating native returned-name evidence from exact-name source authorization. Source child opens still require the recorded exact native name before/after the file-ID open. A target query treats native case and short-name aliases as occupied, rejects search expressions and retains native errors; only explicit native no-match statuses mean absent. Target directory traversal opens the returned exact name with the existing guarded child adapter. Never open target file contents.

On macOS/Unix, use handle-relative statat with SYMLINK_NOFOLLOW and guarded openat for ordinary directories. Only NOENT means absent. Check volume/file identity after child opens, reject special/placeholder ancestors and cross-volume traversal, and retain directory handles until all recorded revisions are rechecked. Reject Apple Photos package components even after an absent earlier component. Bound existing-directory handles to 1,024 and selected rows to the existing 240-photo preview limit. Any native or bound failure fails the review rather than reporting an empty destination.

The version-2 destination fingerprint binds target evidence and directory revisions alongside the earlier picker/root/preview inputs. Rechecking or saving rebuilds observations. Ambient free space is displayed separately; unrelated system activity must not change the identity/target fingerprint. Saved drafts retain the fingerprint with all existing blockers. No existing immutable records or schema are rewritten. Available space is an observation; selected photo bytes omit companions, staging, retained versions and metadata reserves.

## Limits

An unoccupied observation neither reserves a name nor proves equivalence between two names that do not yet exist. Missing ancestors are not created to probe semantics. No native-target gate is removed: future-name case/Unicode equivalence, write/fidelity/capacity guarantees, immutable execution preconditions and atomic no-replace publication still require their own evidence. Directory revision checks cannot establish immunity to all external races. Existing entries survive every check.

The source-tree continuation uses optional display-only backend components derived from exact native relative paths, with a 65-component display bound. Renderers never parse display strings into execution paths. Legacy rows omit this field when reserialized, preserving immutable plan digests. Source trees contain only frozen selected photos, not unknown/excluded/companion inventories.

## Validation and primary references

Generated native fixtures cover occupied files, occupied parent names, missing parents, case/Unicode native lookups, symlink/junction ancestors, package rejection, source/destination preservation, deterministic fingerprints and conflict invalidation. Legacy row and retained-plan digest tests cover absent component fields. Actual full/native/platform outcomes belong in the implementation log.

- [NtQueryDirectoryFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntquerydirectoryfile) documents first-query search capture and the native response contract. A fresh directory handle per query preserves the existing source-lookup policy.
- [Microsoft filesystem behavior overview](https://download.microsoft.com/download/4/3/8/43889780-8d45-4b2e-9d3a-c696a890309f/file%20system%20behavior%20overview.pdf) describes native query matching using filesystem case tables; no universal Unicode lowercase approximation is used.

No new dependency, helper, shell command, filesystem write permission or executable plan is introduced.
