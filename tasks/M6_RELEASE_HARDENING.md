# Stillharbour - M6 - Hardening and distribution

## Objective

Turn the implemented milestones into a trustworthy first release with honest support/performance claims.

## Prerequisites and references

M0-M5 evidence; docs/CODEX_SPEC.md sections 3, 18-21; the complete applicable acceptance backlog.

## Work

Audit central mutation paths, capabilities, root containment, keeper protections, journal durability, sidecar behavior, backup scope and restore verification. Re-run failure injection and concurrency tests, not just happy paths.

Measure declared performance targets against documented hardware and real/synthetic fixtures. Tune caches, batching, thumbnail scheduling and per-volume concurrency only from measured bottlenecks. Record memory peaks of media and backup helpers separately.

Run keyboard/screen-reader/high-DPI/reduced-motion review. Validate all empty/error/offline/recovery states. Ensure original-dependent controls are disabled with useful reasons rather than silently failing.

Produce signed Windows packages and signed/notarized macOS bundles using owner-supplied credentials. Verify bundled helper provenance, licenses/notices and architecture. Test install, first run, upgrades, migration, backup and restore on clean machines without developer tools. Exclude references, private fixtures, dev tools, mock backends and test automation servers from production bundles.

Write user documentation for root modes, duplicate modes, template preview, quarantine/undo, backup repository keys, verification levels, restore, scheduling limits, format/OS support and known limitations. Enable updater only with a tested signature-verification flow.

## Gate

All applicable acceptance tests are mapped to real evidence. Missing macOS/Windows execution, signing credentials, codec coverage or transport restore tests are explicit release blockers, not inferred successes. Deliver packages only when their support claims match results. Summarize remaining post-v1 work without treating it as part of the shipped release.

## Version 1.1 release blockers

Include the 40 scenarios in docs/PRESERVATION_TESTS.md and the 85 original AC scenarios in the evidence matrix. Any unexplained byte/version loss, silent fidelity omission, unsafe overwrite, wrong-destination mutation or false protection status blocks release. Test the packaged helper defaults rather than assuming development behavior. Neither a specification review nor a Linux-only run proves native Windows/macOS or NAS behavior.

## Version 1.2 integration

For any approved N extension included in a release, run its LF acceptance scenarios plus original AC/PS gates. Verify catalog runtime patch status and actual provider/native capability; disabled follow-ons must be honestly labeled. Read docs/LIBRARY_FEATURES_ADDENDUM.md.
