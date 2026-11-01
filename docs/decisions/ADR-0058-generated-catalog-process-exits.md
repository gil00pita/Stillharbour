# ADR-0058: Generated native-to-catalog process-exit recovery

Date: 2026-11-01
Status: implemented with full local validation and unchanged-runtime Windows smoke; checkpoint CI pending; M2 remains open

## Decision

Test actual process exits before database staging, after staging, during final target updates, before receipt insertion, after receipt insertion and after transaction commit, in both forward and reverse generated rename operations. Exit from the database actor bypasses Rust destructors; the parent waits for the child, then cold-reconstructs the generated scope and reopens SQLite. This exercises rollback/WAL behavior rather than an in-process error standing in for a crash.

A bounded, deny-unknown-fields test-only manifest retains canonical forward/active plans, exact original/version object identities, essential metadata, probe objects and catalog/asset identities. Admission requires a UUID owner marker, the generated temporary-directory prefix and native component containment, the exact five constructor mappings and known generated payloads, retained version objects/bytes, complete current native inventory, and the same retained catalog object/identity. Paths are derived only from that owned scope. The parent alone owns cleanup; child/reopened scopes borrow it, and actors close before cleanup. No serialized approval or desktop IPC adoption route exists. This does not implement production restart discovery or authorize opening arbitrary catalogs/libraries.

After a pre-commit exit, every catalog path must roll back and inspection must report needs_catalog_commit while all native movement facts/objects survive. Inspection never commits or moves files. A new session cannot consume an old grant. A fresh acknowledgment/approval revalidates files and commits once without adding native movement facts. After a committed exit, both exact path rows and protected receipt must already agree and inspection reports completed. Asset IDs remain stable in both directions. Child runs are bounded to 45 seconds and only their owned processes can be terminated on timeout.

The hooks and reconstruction helpers compile only in Rust tests. Instrumented desktop and production runtime behavior are unchanged from ADR-0057. Full/feature-graph/native results belong in the implementation log. General application jobs/root organization/restart discovery, multi-root policies and the other M2 gates remain open before M3; independent recovery and real-library mutation gates remain unchanged.
