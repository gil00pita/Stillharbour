# Release status

Stillharbour is in development. The default application enables browsing,
catalog organization, reviewed new metadata exports and read-only analysis.
It does not enable original-file mutation or production backup/recovery execution.

## Product boundaries

| Area                                             | Status                                                                                                           |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| Default native app                               | Implemented; see [build capabilities](CODEX_SPEC.md)                                                             |
| Experimental copy and backup workflows           | Implemented behind nondefault features and tested with disposable fixtures; not accepted production capabilities |
| Original rename/move, duplicate quarantine/purge | Disabled                                                                                                         |
| Production snapshots, restore and schedules      | Disabled pending preservation/recovery qualification                                                             |
| Signed distribution                              | Developer ID/notarization, Windows signing and trusted distribution acceptance remain open                       |
| Clean-machine installation and upgrades          | Acceptance remains open; development hosts and disposable CI runners are not substitutes                         |
| Formats and operating systems                    | Bounded evidence only; see [format support](FORMAT_SUPPORT.md)                                                   |
| Accessibility and performance                    | Bounded tests exist; whole-application and declared reference-hardware acceptance remain open                    |
| Editing, provider imports and mobile             | Not implemented product capabilities                                                                             |

## Gates that remain binding

Before enabling an original-file operation, require immutable reviewed scope,
fresh identities and revisions, companion coverage, durable journaling and
verified exact-version recovery in an appropriate independent failure domain.
Keep source/destination-only files and retained versions. Do not enable execution
to bypass an unavailable recovery provider or an unrun native check.

Release qualification also requires exact artifact/resource provenance, signatures,
notarization where applicable, helper/codec behavior on clean target machines,
current dependency/license review, installed-package instrumentation exclusions,
recovery and upgrade tests, and the applicable native failure/concurrency matrix.
Minimum OS values in configuration do not establish minimum-OS support.

Full-app keyboard, VoiceOver/NVDA, high-DPI and reduced-motion acceptance is not
established by an isolated component or bounded manual review. Performance claims
need the declared workload, hardware, complete process attribution and exact
build inputs; historical benchmark reports retain their original limits.

The complete earlier [acceptance backlog](changelog/2026-11-09/docs/ACCEPTANCE_TESTS.md),
[preservation tests](changelog/2026-11-09/docs/PRESERVATION_TESTS.md) and
[release evidence matrix](changelog/2026-11-09/docs/M6_EVIDENCE_MATRIX.md) remain
preserved obligations, not newly passing results or waived requirements.

## CI and evidence

Before merging, require current-head passes for native macOS, native Windows and
both transport jobs. See [Development](DEVELOPMENT.md#validation-and-ci) for the
exact checks. A documentation cleanup is not release certification.

Read dated outcomes in the [implementation log](IMPLEMENTATION_LOG.md) and
[archived reports](changelog/README.md). Ignored `.context` artifacts can be local
or unavailable in another checkout. Do not infer a test pass from a filename,
old CI run or historical status paragraph.
