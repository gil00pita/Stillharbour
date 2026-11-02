# Stillharbour - Backup & Sync primary-source register

Consulted 2026-10-23. These sources describe upstream capabilities, not implementation evidence for this app. Proposed policies and preset rates in the addendum are design decisions. Verify and pin exact dependency versions during implementation; documentation may evolve.

| ID | Source | Relevant fact / use |
| --- | --- | --- |
| SB01 | FreeFileSync - Comparison Settings: https://freefilesync.org/manual.php?topic=comparison-settings | Separates time/size and content comparison; informs explicit evidence labels. |
| SB02 | FreeFileSync - Synchronization Settings: https://freefilesync.org/manual.php?topic=synchronization-settings | Mirror can remove target files; update and two-way have different semantics. |
| SB03 | FreeFileSync - Versioning: https://freefilesync.org/manual.php?topic=versioning | Retained historical versions provide recovery from replacement/removal; app policy is stricter than a replace-only history folder. |
| SB04 | FreeFileSync - Expert Settings: https://freefilesync.org/manual.php?topic=expert-settings | Background priority and binary post-copy verification; explicit caveats about cache layers and already-corrupt source data. |
| SB05 | rclone - Documentation: https://rclone.org/docs/ | Bandwidth, transfer/checker concurrency, buffering and multi-thread controls; capabilities must be tested per adapter. |
| SB06 | rclone - copy: https://rclone.org/commands/rclone_copy/ | Copy does not remove destination-only files; does not replace the application's complete safety contract. |
| SB07 | restic - Working with repositories: https://restic.readthedocs.io/en/stable/045_working_with_repos.html | Structural check versus --read-data verification; subset checks do not establish full coverage. |
| SB08 | restic - Restoring from backup: https://restic.readthedocs.io/en/stable/050_restore.html | In-place restore interruption risks; existing files are overwritten by default unless configured otherwise. |
| SB09 | restic - Backing up: https://restic.readthedocs.io/en/stable/040_backup.html | Snapshot behavior and original-file backup workflow. |
| SB10 | rclone - Local Filesystem: https://rclone.org/local/ | Source-change checks, cloning/fidelity considerations and filesystem-specific options. |

No claim of trademark/domain availability for the owner-selected name Stillharbour is made. No FreeFileSync code or assets are included.


## Native recovery classification implementation references (2026-11-02)

The storage adapter uses [Microsoft's volume disk extents control](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-ioctl_volume_get_volume_disk_extents) and [VOLUME_DISK_EXTENTS](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ns-winioctl-volume_disk_extents) to distinguish partitions from physical disk membership. Its strict bus allowlist and refusal of virtual/unknown storage are application policy, not a Microsoft guarantee of disaster independence. Fresh observations never become imported catalog authority. macOS physical interconnect classifications follow [Apple's IOStorage protocol definitions](https://github.com/apple-oss-distributions/IOStorageFamily/blob/main/IOStorageProtocolCharacteristics.h); synthesized APFS volume identifiers alone do not prove independence. Actual native results and unsupported paths remain separately qualified in M4 validation.

M4 macOS process ownership review (2026-11-02): pinned restic 0.19.1 [foreground_unix.go](https://raw.githubusercontent.com/restic/restic/v0.19.1/internal/terminal/foreground_unix.go) sets the rclone process group before checking for a terminal; its [rclone backend](https://raw.githubusercontent.com/restic/restic/v0.19.1/internal/backend/rclone/backend.go) also explicitly kills that child on setup failure. The application therefore supervises the actual rclone through a private same-executable worker and guardian, with registration before spawn and completion held until reap. One restic process-group signal is insufficient evidence of nested-helper completion. Actual macOS tests remain the execution gate.

Apple [libproc.c](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/libsyscall/wrappers/libproc/libproc.c) returns a PID count from proc_listchildpids, unlike proc_listpids's byte count. The generated ownership probe follows the count contract. Apple [sys/acl.h](https://raw.githubusercontent.com/apple-oss-distributions/Libc/main/include/sys/acl.h) provides the native ACL constants and signatures used by the narrow metadata classifier. Source review does not replace native execution.

Darwin temporary-directory review: Apple [confstr.c](https://raw.githubusercontent.com/apple-oss-distributions/Libc/main/gen/confstr.c) can fall back to TMPDIR for its user-temp query when the native directory helper fails. Completion sockets cross-check the native user-directory query and its known temp sibling, validate native ownership/containment, and reject oversized socket paths. They do not rely on an inherited long TMPDIR or silently fall back to it. The actual Mac protocol fixture exposed the 105-byte socket path that prompted this regression fix.
