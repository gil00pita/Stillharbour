# Stillharbour - M3 - Duplicate review, integrity and storage

## Objective

Find redundant files with explainable evidence; review keepers and recoverably quarantine approved copies; expose integrity and size diagnostics.

## Prerequisites and references

M2 executor and recovery gates; docs/CODEX_SPEC.md sections 11 and 14; acceptance AC-040 through AC-057 and AC-059 through AC-065.

## Work

Implement exact-content size/sample/full-hash candidate pipeline with versioned fingerprints, direct full-byte verification before quarantine, and typed per-file errors. Support BLAKE3/SHA-256 plus SHA-1/MD5 compatibility modes. Sampling and legacy hashes never authorize removal alone.

Implement advanced filename/extension/size/created/modified/attribute criteria with explicit AND/OR semantics, unknown values and normalization previews. Add normalized-image comparison only for declared fully supported format/precision/color paths. Expose ID3 as not applicable to photo modes. Recognize hard-link identity independently of content hashes.

Create a side-by-side duplicate review and detailed table. Support preferred/protected folders, oldest/newest filesystem-date policies, manual overrides and keep-multiple. The backend blocks last-accessible-copy removal and unique companion loss. Use only M2 quarantine/restore planning, never a new ad hoc delete function.

Add quick/deep/checksum-baseline health jobs and distinct coverage/status results. Implement largest files, recursive folder totals, a treemap and accessible table with separate media/all-files scope and honest logical/allocated/reclaimable metrics.

## Later scope

Perceptual similarity and smart culling are post-v1 by default. Do not present them as implemented. AC-058 becomes mandatory if the owner explicitly adds that mode now.

## Gate

Relevant acceptance scenarios pass, especially mock hash collisions, same-size/restored-mtime mutations, offline keepers, protected paths and unique bundle companions. Demonstrate quarantine restoration and no false disk-space reduction. Record unsupported validation formats and stop before M4.

## Version 1.1 preservation gate

Read docs/PRESERVATION_CONTRACT.md and docs/PRESERVATION_TESTS.md. Default to source-retaining verified copies. Develop mutation/recovery against disposable fixtures, but leave real-library in-place mutations and duplicate quarantine disabled until exact-version independent recovery evidence and applicable native failure gates are available. M4 may supply that provider later; do not waive it to complete an earlier milestone. No speed optimization may bypass full verification or protected-version retention.

## Version 1.2 integration

N1 adds structured inventory/content reports. Treat checksum mismatch, no baseline, missing, offline and incomplete independently; never auto-repair or reset baseline history. Read docs/LIBRARY_FEATURES_ADDENDUM.md.
