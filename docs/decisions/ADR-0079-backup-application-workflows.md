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
