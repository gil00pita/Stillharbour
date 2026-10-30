# P4 continuation — empty filtered results

The owner's 2026-10-29 “Start the new step” continues the published P4 findings. Reproduce the filtered-query slowdown against owned synthetic catalogs in a release core build, isolate a fast regression and implement a bounded fix. Stop after validation and reporting; P4's wider trace/memory/platform/backup gates remain open.

The release reproduction identifies a no-match page that scans the ordered asset projection after its exact filtered count is already zero, followed by a redundant empty timeline scan. Reuse the existing connection-owned summary cache and freshness checks. Skip redundant reads only when an exact count proves emptiness. Inventory counters can lag a streaming scan and must never hide discovered rows or timeline entries. Preserve validation, page fields/revision/cursors, other query scopes, source access, schema, dependencies and durability.

Cover all five sort orders, both empty-page/timeline request orders, invalid requests, partial inventory and actor/external writes that make an empty result nonempty again. Use logical pager budgets for deterministic regression tests; report release timings and positive-match controls separately. The fixture probe accepts only a previously generated UUID/row-count namespace, never a user-library path. Keep independent lossless source inventories, raw samples, binary/lock identity, failures and unrun gates.

Record actual checks and next gates in [PERFORMANCE_FILTERED_READS.md](../docs/PERFORMANCE_FILTERED_READS.md), update owning specifications/acceptance/development guidance and append the implementation log. No original mutation, source scan, migration, new scheduling system or publication is included.
