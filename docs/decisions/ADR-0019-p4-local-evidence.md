# ADR-0019 — Local P4 trace and catalog lifetime evidence

Date: 2026-10-30  
Status: accepted for the owner-authorized local P4 closeout; all three scoped native trace routes pass

## Context

The prior 100k route lost two events below capacity. A focused concurrent recorder regression loses one of 1,024 events; holding the single producer mutex reproduces one lost event deterministically. The existing mutex makes unrelated producers and volume alias lookup contend. A first eight-lane rotating prototype still loses two events during the sixth planned repeat. Preserve those failures.

A separate owned-child regression reproduces CatalogInUse/OS35 after the catalog actor is joined: a forked child can retain the open file description until exec closes inherited CLOEXEC descriptors. This establishes a reproducible cause of this error class; the earlier isolated historical failure had no process evidence to prove its individual cause.

## Decision

Use sixteen fixed producer lanes of 256 numeric events each, with a stable preferred lane per thread and at most sixteen nonblocking try-lock attempts. No event-capacity or dependency increase. Keep volume aliases under a separate bounded lock. Consumer drains allocate before acquiring lanes, swap buffers under the locks and flatten afterwards; snapshots remain bounded and loss counters are never suppressed. A fully busy/full recorder still drops and reports the event. Thread preference is an internal scheduling aid, not identity or a serialized field. Backend events are grouped by lane; the format does not establish a global temporal order across producers. Tests instantiate the same recorder implementation independently so parallel catalog tests cannot contaminate their signal. Default production calls remain no-ops.

Own the SQLite connection and lifetime-lock guard together, in connection-before-lock field-drop order. Explicitly unlock the app-owned lock after SQLite closes, including early return/unwind paths. Keep strict acquisition; no retry/bypass or source operation. Owned fork/exec regression and live-catalog exclusion checks validate this class without claiming native Windows evidence.

Report clean, partial, failed, unavailable and unverified observed-process memory collection separately. Complete WebContent/GPU/XPC ownership remains unestablished without appropriate native evidence; the nonprivileged host diagnostic cannot grant itself elevated access. Keep earlier P4 runs historical and release automation forbidden.

## Consequences

The recorder has a small fixed lane/lock overhead and may still lose events under full contention, during consumer critical sections or beyond capacity. Strict reports continue to fail on any observed loss. No universal real-time or whole-app memory guarantee follows. Original-file safety, schema, dependencies, bounded queues and all external platform/preservation gates remain intact. Actual commands, run identities, failures and final acceptance are in [PERFORMANCE_LOCAL_CLOSEOUT.md](../PERFORMANCE_LOCAL_CLOSEOUT.md).
