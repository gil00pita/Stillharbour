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
