# M4 continuation validation

2026-11-02. This matrix records implementation evidence, not full M4 acceptance. The owner authorized all M4 stages and publishing a PR. No further per-stage authorization is required; preservation gates remain mandatory.

| Slice | Implemented evidence | Open production gate |
| --- | --- | --- |
| M4A | Read-only comparison, history, profiles and review; both native platforms at `90acf91`, see [M4A_VALIDATION.md](M4A_VALIDATION.md). | Does not authorize transfers. |
| M4B | Complete-report append rules and native scope revalidation; separate staging in shared executor; generated Windows append verification, nine real child exits and preparation/corruption/disk/journal/approval failures. | Catalog-backed append jobs, native preparation recovery, capacity/fidelity review, actual controls/UI and both-platform acceptance. |
| M4C | Closed generated retained replacement; exact old native object and descriptive times retained; three previous versions recoverable; full old/new byte verification and fresh-output old restore; seven real child exits, corrupt baseline/history collision/late occupant rejection. | Actual replacement adapters/UI, essential metadata/stream fidelity, exact-version independent recovery gate and both-platform failure evidence. |
| M4D | Hash-pinned upstream helpers, licenses, extraction/supervision tests and disposable loopback servers; all five Windows backup/source-removal/restore round trips, incorrect credential/trust/corruption rejection. | Bundled application helper adapters/process supervision, OS-backed credentials, actual catalog snapshots, history/browser/restore/protection reporting, schedules and independent recovery. macOS transport results pending CI. |

Actual local receipts/logs are ignored under `.context`, with native and transport CI workflows retaining per-platform artifacts. Windows desktop integration passes nine cases in 2m08s (17 platform-specific skips across two specs), including generated append and retained-version probes. Both receipts explicitly report production execution unavailable and the replacement receipt reports independent recovery unverified.

Seven final focused `m4c_` regressions pass in 6.31s, including separate restore approval and durable output-directory intent. Seven Python extraction/process-supervision tests pass outside the restricted sandbox. [Transport evidence](M4_TRANSPORT_TESTS.md) records five passing strengthened round trips and retained earlier failures. Full repository checks, final rebuilt native tests and production frontend release exclusions are recorded in the implementation log as their commands complete. Do not infer a passing command from an intermediate build.

All 40 preservation scenarios and original M4 acceptance criteria remain required. Same-disk generated history is not disaster protection; helper transport success is not an application workflow, physical I/O cap, signed distribution or provider-original fidelity claim. Production original mutation, transfer/replacement and backup remain disabled. Stop before M5.
