# ADR-0079: ID-bound backup application workflows

Date: 2026-11-02. Status: implementation in progress; acceptance gates remain open.

The owner authorized all M4 stages and disposable local transport servers. The
generated labs establish reusable engine evidence, but do not substitute for
application workflows or exact-version disaster recovery.

Schema 13 adds separate durable readable-copy jobs/plans/events and snapshot
profiles/jobs/records/events/schedules. Migration remains transactional with an
independently openable pre-migration catalog; persisted records never reconstruct
interactive approvals. Profile and snapshot history is retained. A distinct
`backup-workflow` feature prevents enabling unfinished M2 original operations as
a side effect. Native acceptance precedes enabling an affected production path.

Readable copy uses separate preparation/copy/continuation approvals. Native
complete-set name preparation retains exclusive probes until aliases have been
tested together. The shared copy engine stages separately, compares full bytes,
journals intent before effects and publishes without replacement. Existing
destination files require explicit keep decisions; replacement additionally
requires the exact old destination's independently verified recovery evidence.
Source-root snapshots do not prove recovery of a manually edited destination.

Restic 0.19.1 and rclone 1.75.2 are fixed host resources with reviewed executable
hashes and licenses. Build tooling downloads only explicitly; application code
does not search PATH or download executables. Each helper launch revalidates
native resource identity and pins. Children use a scoped environment, bounded
output/time, Windows owned jobs or macOS owned process groups, and cancellation.
No recovery invokes unlock, repair, forget or prune automatically.

Passwords enter native dialogs and go directly into the OS credential store.
The renderer receives opaque catalog/purpose-bound references. Transport secrets
also bind the complete typed endpoint/trust context. Re-enrollment creates a new
reference and a superseding immutable profile after repository validation;
existing references, namespace identity and history remain intact. Recovery-key
export uses native selection and explicit plaintext-key confirmation, a new
owner-only file, durable nonsecret intent and reopened byte comparison.

Snapshot staging inventories complete authorized roots independently of gallery
filters, retains source revisions and hashes, and uses SQLite-consistent catalog
copies. Whole-snapshot restore publishes a new protected namespace and compares
every file and independently opens the catalog read-only. Included, structural
check, full data read, restored comparison and independent recovery are distinct
states. Ordinary-byte staging does not certify ACLs, alternate streams, extended
attributes or resource forks and cannot issue strict original-mutation evidence.

P-04 requires exact reviewed operation approval. Running-process schedules may
repeat only an explicitly approved manifest/profile digest, sealed in OS storage
and bound to the native catalog instance. New content, versions, roots or endpoint
settings require new review. Imported/restored catalog schedules do not acquire
authority. Intervals use elapsed UTC minutes with no DST shift; one catch-up is
allowed only for an unchanged scope. Fully quit execution remains unavailable.

The SHS2 OS record binds the policy digest, generation, next-admissible UTC
deadline and last scheduled attempt. SHS1 digest-only records cannot silently
acquire timing authority and require fresh reviewed consent. Catalog timing is
an untrusted mirror; display and admission read the OS state. One process-wide
transaction serializes enable/disable/read/claim/start, while the existing native
catalog lifetime lock excludes another app process. Native replacement writes
are not treated as compare-and-swap: under that transaction, advance the deadline
and verify readback before catalog or worker-start writes. Failure/crash never
rolls the high-water mark backward. Attempts, including failed admission, are
not successful backup claims.

Authority keys always use the current native catalog instance, never the
instance supplied by an imported row. Imported disable cannot revoke another
catalog's OS grant. Successful disable removes the current authority. Catch-up
admits at most one exact-scope attempt; unchecked catch-up skips deadlines more
than 30 seconds overdue, allowing two normal 15-second polling periods. Skipped
or attempted deadlines advance from current admission time, including sleep,
restart and busy backlogs; no platform background agent is installed.

Validation is recorded in M4_VALIDATION.md and the implementation log as commands
complete. No application/metadata/independent-recovery gate is implied by the
successful Windows/macOS disposable protocol round trips.

Native metadata recovery is being integrated as a versioned, bounded policy.
Ordinary owner/permission/descriptive-time restoration is distinct from complete
security/stream fidelity. Windows alternate streams/extended attributes and macOS
extended attributes/resource forks/extended ACLs outside the tested adapter are
explicitly unsupported for strict recovery. Audit/label and unknown security
metadata must not silently acquire certification. New-output metadata restoration
never authorizes changing the original object.

Fresh local failure-domain checks map actual native physical device membership,
rather than comparing volume labels/UUIDs alone. Two partitions on one disk, a
shared multi-device set, file-backed disks, unknown storage and localhost remote
servers do not provide independent-device proof. Windows uses fixed read-only
storage controls; macOS traverses native IOKit physical ancestry. Observations are
ephemeral and rechecked, not imported catalog authority. Native validation remains
required before claiming either classifier supported.

## Pinned helper failure and resource policy (2026-11-02)

Remote commands use one shared fixed argument policy with a 10-second connect
timeout, a 30-second I/O-idle timeout, one high-level rclone attempt and two
low-level attempts. Restic bounds rclone startup at 20 seconds and stuck requests
at 30 seconds. Healthy streaming activity resets idle timing; the existing total
helper deadline remains. Restic progress is sampled every five seconds so
heartbeats cannot exhaust bounded output merely because a healthy job is long.

The adapter observes only the pinned backend's recognized retry/failure header
shapes. A monotonic failure signal stops and reaps the owned process tree and
waits for nested native completion; a late report also defeats a zero exit code.
This yields `needs_recovery` with sources, partial copies, repository objects and
locks retained. It authorizes inspection, never an automatic backup replay,
unlock, prune or repair. The observer neither exposes raw output nor treats
source filenames or JSON progress as executable/error-control instructions.
