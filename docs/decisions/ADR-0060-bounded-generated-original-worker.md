# ADR-0060: Bounded generated worker and cooperative stop

Date: 2026-11-01
Status: implemented and locally validated; target CI pending; M2 remains open

## Decision

Replace the generated dialog's blocking operation request with one backend worker and no pending-work backlog. Consume approval and persist its queued attempt before spawning; reject another start, approval or native inspection while active. An independent typed status/stop command reads the actual catalog job and journal-step counter without taking the native session lock. Requests contain no paths; stop is accepted only for the current job UUID.

The worker owns its scope through shared controller state. Catalog readers drop before the session actor and generated-directory owner. A source/verification step finishes before the next cooperative stop check; do not promise instantaneous cancellation or rollback. Stop leaves files/stages and facts retained, records needs_recovery and requires a fresh inspection/acknowledgment/approval to continue. A stop arriving during an already committing verified catalog transaction may finish as completed. Panic/launch failures remain errors and do not restore approval. Catch worker panics while retaining the session guard outside unwinding, then revalidate every fact on subsequent inspection.

The central executor publishes completed movement counts only after durable completion facts. Resumed counters begin at the retained cursor. The UI polls at most one metadata request at a time, disables review/start controls while active, labels paths as the last inspection, and offers an explicit Stop after current step action. On termination it obtains fresh native/catalog inspection. Closing retires late responses and stops polling; it does not implicitly stop the backend worker or restore a ticket.

A native-test-only action can arm a hold after the first durable move. It accepts no supplied scope/index/path, lasts at most fifteen seconds and stops with retained objects on expiry; it never silently resumes. This makes the actual UI stop scenario deterministic. Default production handlers/capabilities/frontend exclude both generated commands and the hold route. No schema change, arbitrary-scope adoption, general library orchestration, watcher reconciliation or production original-mutation authorization is added.

## Validation

An isolated owned core test process (45-second parent limit, no supplied paths) avoids conflating other fault matrices’ shared I/O pacing with worker liveness. It holds the first recorded move, rejects a second start/native inspection and a foreign stop, stops with one retained step, refuses the old grant, and completes only after fresh approval. Test async dialog status/stop/refresh, separate consent, late grant/status disposal and no implicit stop on close. Native desktop automation arms the bounded hold, clicks the real stop control, observes the retained stage/old catalog paths/needs-recovery job, then separately approves continuation and undo. Full/feature/native results belong in the implementation log; remaining M2 gates still precede M3.
