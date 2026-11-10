> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0078: Generated retained replacement and disposable transport labs

Date: 2026-11-02. Status: accepted for fixture development; production M4 gates remain open.

The owner authorized all remaining M4 stages, commit/push/PR and local disposable protocol servers. This removes the need to request separate stage authorization. It does not remove preservation acceptance gates.

Extend the closed ADR-0077 constructor-owned lab with one reviewed different destination version. Freeze its native object, revision, descriptive times and independently generated full-byte digest. Use the shared paced executor for incoming staging and an old-byte baseline. Persist old-move intent before moving the exact old object into unique history using native no-replace publication. Reverify the retained object against the frozen digest before publishing incoming content. Retain two earlier history versions and destination-only neighbors. Restore the old version into a new protected output and compare every byte.

Use ordered bounded journal facts, fresh observations and a new single-use approval for recovery. Never infer permission from a journal. Tests interrupt seven boundaries, including actual child process exits after filesystem changes but before recording completion. Child admission is test-only, bounded to the exact generated temporary root/nonce and parent-hashed control record; it cannot adopt a library or persisted approval. Unknown occupants, changed baseline content and changed native objects block publication while preserving every version.

The lab makes no independent-device, essential-stream/permission fidelity or production replacement claim. Its native IPC is available only in the existing desktop-test diagnostic command. Production IPC/capabilities remain unchanged.

Transport probes independently exercise pinned restic 0.19.1 and rclone 1.75.2. They accept a transport list and evidence location, never a source library. Generate all sources, SQLite fixtures, repositories, credentials, host keys and certificates per run. Bind servers to loopback, stop them in finally blocks, and remove only the exact unchanged generated source after backup/check/full-data-read. Restore an explicitly identified snapshot subfolder into a new output with overwrite forbidden, then compare the complete manifest and open the restored SQLite fixture independently. This is not application catalog-snapshot or production backup acceptance.

Use rclone's SFTP server with strict known-host checking; pyftpdlib 2.2.0 for FTP/FTPS; pinned SMBLibrary 1.5.7 on Windows and Impacket 0.13.1 on macOS. Windows SMB uses virtual credentials, a high loopback port and no system service/account/share changes. The SMBLibrary DLLs are development-only and excluded from the app. Exact hashes/versions are committed. Python dependencies use hashes and platform markers; Windows does not install Impacket.

FTPS tests require encrypted control/data channels and certificate validation. The test server explicitly negotiates TLS 1.2 because the attempted TLS 1.3 data path timed out; retain that limitation. Test unknown SFTP hosts and untrusted FTPS certificates as failures. Plain FTP requires an explicit harness acknowledgment and is not confidential. Do not disable client certificate/host verification or claim TLS 1.3 acceptance.

Windows generated transport round trips pass for local/SFTP/FTP/FTPS/SMB. The macOS workflow must produce its own receipts before acceptance. Remaining production work includes catalog-backed transfer/recovery UI, capability/fidelity/capacity review, helper packaging/process supervision, credential/trust profiles, actual catalog snapshots, backup history/browser/restore, protection reporting and schedules. No M5 work is authorized by this decision.
