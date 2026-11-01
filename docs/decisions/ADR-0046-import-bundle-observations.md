# ADR-0046 — Whole-bundle import observations

Date: 2026-10-31. Status: experimental M2 planning; full checks and Windows/macOS native CI pass.

## Decision

Extend the read-only comparison report with connected confirmed-bundle observations. A shared companion joins all affected selected bundles transitively; every selected file appears in exactly one group. Keep the original manifest, IDs and target assignments unchanged.

A complete existing group requires a directly compared match for every selected member, distinct indexed keeper IDs, one registered root/native parent and a common renamed stem with exact source suffixes. Primary photo matches must be indexed media. Compound suffixes such as `.JPG.xmp` remain intact. Derive grouping from native locators, never shortened/lossy display names. Native separator-equivalent source paths identify the original primary. Non-Unicode names, alternative source naming, missing primary evidence and incomplete coverage require attention. Scattered file matches, missing companions or non-media primary matches cannot produce a complete existing bundle; retain the whole source bundle for copying.

Expose bounded candidate groups as observations for a later explicit import plan. Reuse the comparison worker's bounds, read leases, direct byte evidence and version checks. These reports grant no skip, copy or removal authority and do not persist approval. Future execution must bind the selected complete group and freshly recompare every member. No schema/dependency/default-feature changes.

The UI validates complete source coverage, connected bundle membership, distinct keepers and corresponding byte-match/root evidence before rendering. It shows whole-bundle outcomes and the fresh-verification requirement alongside individual file evidence.

## Validation

Generated tests cover renamed RAW/JPEG/compound sidecars, transitive shared companions, different folders/roots/stems, missing companions, non-media primary matches, ambiguous/reused keepers, incomplete coverage, alternate source naming and invalid native Unicode. An actual catalog/native comparison checks coherent renamed photo/sidecar groups and unchanged source/candidate/journal evidence. Component cases cover whole-bundle presentation and rejection of unverified keeper evidence. Full/native outcomes follow in the implementation log.

This is a planning prerequisite. Actual import skipping, mixed copy/skip execution and the remaining M2 gate still precede M3. Real-library original mutation and duplicate quarantine remain disabled.

Both target-platform CI jobs pass in [run 41252874028](https://github.com/gil00pita/Stillharbour/actions/runs/41252874028). This validates this checkpoint only; remaining milestone gates are unchanged.
