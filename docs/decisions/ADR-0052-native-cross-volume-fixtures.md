# ADR-0052 — Owned native cross-volume fixture harness

Date: 2026-11-01. Status: implementation in progress; mounted-volume evidence pending.

## Decision

Use disposable file-backed filesystems to exercise actual distinct native volume identities. The runner accepts no supplied disk/library paths. Create a nonce-owned image under `.context/cross-volume/<uuid>`, a fresh filesystem, a marked fixture parent and a bounded scope record. The Rust example requires both fixture-probes and copy-workflow, rejects path arguments, validates the run/image/marker boundaries and refuses equal native device or volume tokens before claiming cross-volume execution. Sources and catalog remain constructor-generated temporary data.

On Windows, DiskPart receives only creation of the new named VHDX file. Storage cmdlets derive their disk/partition objects from that exact mounted image. Check one attached non-system/non-boot virtual disk of the expected size, RAW state before initialization, and unchanged image-to-disk mapping before partition/format. Format only the derived fresh partition. The harness requires an administrator runner; preflight unavailability is not a native pass. Invoke the local authored script under process-only RemoteSigned and let Windows PowerShell discover its own built-in modules rather than inherit incompatible host module paths. No persistent execution policy or privilege elevation changes.

On macOS, create a new sparse APFS image and mount it at the owned evidence path. Parse native plist responses, bind detach to the same image's returned device chain and never force detach. Both harnesses retain reports on failure, remove only the exact regular owned image after successful detachment/pass, and never recursively delete a mounted volume or choose a physical disk by a supplied number. Windows cleanup rejects reparse-point ancestors. Record OS helper versions/checksums. Disk creation/format helpers are outside the application's paced payload-I/O budget and are used only on these disposable images.

The application-core probe runs actual separate preparation/copy approvals and jobs, fresh output inspection and source/destination-only preservation. It additionally exercises nine in-process interruption boundaries through the catalog engine, rejects raw replay and requires fresh approved continuation with complete-byte inspection. These are native integration/in-process interruption results, not desktop picker UI, process-kill, physical-device unplug or power-loss evidence. A file-backed second filesystem is not an independent hardware recovery copy and cannot enable original mutations or satisfy M4.

## Checks and gates

Run `npm run native:cross-volume -- --preflight` for capability reporting only. Run `npm run native:cross-volume` on native CI; unavailable creation/mounting is a failure, never a skipped pass. The workflow uploads JSON/log/help evidence and excludes disk images. Local generated negative tests reject same-volume parents, foreign IDs/images and unknown record fields without creating copy fixtures. Existing production defaults, schema, application IDs and real-library mutation gates remain unchanged.

Use the installed helper's own help and the primary sources recorded in SOURCES.md. Actual local/CI outcomes follow in IMPLEMENTATION_LOG.md. M2's remaining target-root/multi-root/case-cycle/undo and native failure gates still precede M3.
