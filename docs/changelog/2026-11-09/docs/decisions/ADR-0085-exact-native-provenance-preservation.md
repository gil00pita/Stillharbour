> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0085: Exact bounded macOS provenance preservation

Date: 2026-11-02. Status: implemented; acceptance results in [M5 validation](../M5_VALIDATION.md).

The owner requested fixing build errors, inherited preservation-test failures and remaining M5 acceptance. The focused native inspection regression reproduces the inherited failure: newly created files on this host carry `com.apple.provenance`, which v1 deliberately rejects. Ignoring or stripping that attribute would misrepresent preservation.

## Decision

Add a distinct metadata manifest policy, `essential-macos-provenance-v2-owner-permissions-creation-modification`, only when the sole extra attribute is `com.apple.provenance`. Capture its complete opaque bytes, bounded to 4096 bytes, through the existing descriptor. The existing UID/GID, POSIX permission, creation/modification time, file-flag and extended-ACL checks stay mandatory. Every other xattr/resource fork, unknown/nonzero file flag and extended ACL remains unsupported for strict replacement. The 256 KiB manifest bound is unchanged.

Treat provenance as data, never trust/authorization. A setter's return code does not prove it installed the requested bytes. On a different, journal-owned NEW output, leave an already matching value in place; otherwise request its exact captured bytes and independently read them back. Mismatch, denial, changed metadata or an unexpected target attribute fails without publication/false completion. Never call removexattr, normalize the bytes or rewrite the source. Existing source identity/revision, single-link, approved-scope, durable journal and post-restore read-back guards remain in force. Retained restore callers continue flushing before recording verified metadata.

This is a versioned preservation adapter, not a weaker exclusion policy. Descriptor APIs are defined by [Apple's fsetxattr documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsetxattr.2.html) and [Darwin xattr declarations](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/xattr.h). No undocumented provenance format is interpreted or assumed. A target/OS that cannot reproduce the exact value remains unavailable; no universal OS/cross-host fidelity follows from a successful local fixture.

## Compatibility

Existing manifests retain version 1 and their original policy string. The new optional field is omitted when absent, retaining the original serialized digest bytes. Version 1 may not carry provenance, and version 2 must carry it; unknown versions, policy/field mismatches and over-bound values fail validation. A v1 manifest cannot silently adopt a target's new OS attribute. Imported approvals and manifests gain no execution authority. No SQLite migration, catalog UUID, root, keychain item, approval or backup repository is renamed/reset.

Windows' metadata/ADS/audit-security policy remains unchanged. Complete M4 preservation certification, clean-machine/other-platform acceptance and original-file production gates are not enabled by this adapter.

## Regression and native evidence

The new provenance test was run red against v1 before implementation. Native descriptor probes establish observed values independently of setter success. Tests cover exact byte capture/restore, digest changes on altered bytes, invalid policy downgrade, missing/oversized v2 data, immutable source revision/bytes, retained unsupported source forks/attributes, target-attribute rejection before restoration writes, and exact v1 digest compatibility against the old serialized bytes. Actual retained-copy process-exit/recovery tests exercise the adapter without weakening their assertions. [M5 validation](../M5_VALIDATION.md) and the implementation log record counts and complete-suite outcomes after they run.

Follow-up: a full-suite native provenance fixture produced an intermittent restoration EINVAL while isolated/sixty-repeat runs passed. The positive fixture now retains existing OS-provided provenance rather than attempting to replace it with synthetic bytes; absent-attribute hosts still establish a bounded fixture attribute. Exact restore/source assertions remain required, and static restoration-step diagnostics distinguish native provenance from creation-time refusal. No broad host restoration capability is inferred from a retry; M5 validation retains the failed run and required full rerun.
