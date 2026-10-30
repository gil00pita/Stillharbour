# P4 — close the remaining local evidence gaps

Owner choice, 2026-10-30: finish remaining local P4 work before M2. Preserve the uncommitted cold-summary slice. M2 is held; this task does not authorize original-file operations or production automation.

Build fast regressions for under-capacity concurrent trace loss and the intermittent CatalogInUse restart class. Keep the original 4,096-event total and 16-volume privacy limits, nonblocking producers, explicit loss reporting and production no-ops. Any revised recorder remains diagnostic-only; zero loss on one workload is not a universal lossless guarantee. Preserve the first partition prototype’s failed repeat run.

Reproduce catalog locking with an owned child paused between fork and exec. Close SQLite before explicitly releasing the lifetime lock; a child’s transient inherited descriptor must not prolong a closed catalog’s lock. A genuine live catalog must still reject a second owner. Do not bypass the lock, add open retries or touch user catalogs.

Run fresh, separately identified native 10k/100k/1m routes with 120 steps and at least 60 seconds each, ten date jumps, four resizes, viewer/selection return, bounded residents and no text image requests. Retain all sequential trace segments, losses, sampler stdout/stderr/exit metadata, build identity and independent lossless source manifests on success and failure. No concurrent builds, OS cache purge or real-library development scans.

Report observed-process memory collection separately from full WebContent/GPU/XPC ownership. Attempt the available host ownership diagnostic against an owned process without elevation; inaccessible privileged attribution remains blocked, not passed. Preserve production release instrumentation exclusion, Windows/minimum-OS/clean-machine, release presentation, backup/endpoint/pacing/NAS/HDD and recovery gates.

Run full repository, release regressions, default Clippy, evidence tests, native generated-library regressions, unsigned build, packaged helpers/SQLite and production exclusions. Document exact outcomes in docs/PERFORMANCE_LOCAL_CLOSEOUT.md and the implementation log, update requirements/acceptance/development/decisions, then stop at the local P4 gate.
