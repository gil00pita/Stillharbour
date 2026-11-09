> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0091: Explicit catalog Keychain authorization

Status: accepted for the owner's startup repair, 2026-11-05.

The file-based macOS Keychain can display authentication UI despite the SecItem
query's existing UI Fail attribute. The native regression observes the process
interaction flag still enabled after an ordinary lookup. Additionally, the local
development executable has an ad-hoc code-hash designated requirement, so rebuilds
can lose Keychain trust. There are no valid signing identities on this Mac.

Serialize every native Keychain operation with a single mutex. Set the legacy
process UI policy to deny before ordinary calls and retain the existing per-query
UI Fail and exact service/account/non-synchronizing attributes. Reset the process
policy to deny when the guard exits, including failures. Fail before item access
if policy establishment fails. Authentication-required statuses become
CREDENTIAL_ACCESS_REQUIRED; missing remains exclusively errSecItemNotFound.

The native startup shell may offer one explicit Authorize action only when real
catalog selection returns that typed error. Hold the existing appdata lifetime
lock and revalidate its identity before reading only that scope's activation item
with foreground interaction enabled. This exception neither writes authority nor
reenrolls a secret. No renderer request or background scheduler can invoke it.
After consent, repeat ordinary catalog selection once; preserve every existing
generation, history, descriptor, schema, root and retained-copy check. Cancellation
and a failed retry stop, without a loop or empty-catalog fallback. Unrelated
catalog errors cannot trigger authorization. This narrowly supersedes the earlier
universal no-interaction policy for this explicit native startup action only.

Keep the same credential service/accounts, file-based keychain, app identifiers,
catalog paths and UUIDs. Do not weaken ACLs, cache stale authority as a substitute
for readback, migrate to a new store or trust a code requirement containing only
an identifier. Stable cross-rebuild trust needs consistent actual signing; this
change does not invent a certificate or remove that release gate.

See [startup behavior and primary sources](../KEYCHAIN_STARTUP.md) and the
implementation log for actual tests and remaining manual/native limitations.

## Activation lock lifetime follow-up

The first aggregate Rust run reports CatalogInUse while reopening a just-closed
generated activation store (518 pass, one fail, fifteen existing ignores). A
controlled Unix child held before exec reproduces that failure deterministically:
CLOEXEC descriptors are inherited until exec, and closing the parent's descriptor
alone leaves flock held by the child. This is distinct from Keychain denial.
Use an ActivationLock RAII wrapper that explicitly unlocks only the owned lock
on final drop, including errors after acquisition. This follows CatalogLock's
existing correction; it does not add a retry or weaken a live owner's lock.
The regression requires a second open to fail while the owner is live, reopening
to succeed while the unrelated child is still pre-exec after owner closure, and
another second open to fail again while the new owner is live. Catalog's retained
native owner keeps the activation guard through worker/SQLite shutdown.
