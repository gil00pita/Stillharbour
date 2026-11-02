# Disposable M4 transport tests

Owner selection, 2026-11-02: build local disposable test servers. No existing account, real library, OS user, share, service or external endpoint is used.

Use Python 3.13. Build-time helper preparation verifies both archive and executable SHA-256 against `vendor/backup-helper-manifest.json`. The application never downloads helpers at runtime; its feature-gated native adapter invokes only prepared, pinned resources. Production execution remains gated; transport evidence alone does not enable it. Run from the repository root:

```text
python scripts/prepare-backup-helpers.py --download
python -m venv .context/m4-protocol-env
# Windows interpreter: .context/m4-protocol-env/Scripts/python.exe
# macOS interpreter: .context/m4-protocol-env/bin/python
<venv-python> -m pip install --require-hashes -r scripts/m4-protocol-requirements.txt
# Windows only:
<venv-python> scripts/prepare-smb-fixture.py --download
<venv-python> -m unittest discover -s scripts/tests -p test_backup_helpers.py
<venv-python> scripts/m4-protocol-probe.py --protocols local sftp ftp ftps smb --allow-insecure-ftp --evidence .context/m4-protocol-receipt.json
```

`python` must refer to Python 3.13 (on Windows `py -3.13` may be used to create the venv). GitHub workflow `m4-transports.yml` runs the same generated round trips on Windows and Apple Silicon macOS. Normal developer execution needs permission to launch loopback servers and read temporary-tree ancestor metadata; a sandbox denial is a failed run, never a successful partial backup. No administrative service setup is necessary. The pinned cryptography dependency generates a one-day invocation-owned FTPS certificate with the loopback IP subject alternative name; no external OpenSSL command is required.

Every transport runs init, backup, structural check, full data-read, generated source removal, restore into a new directory with `--verify --overwrite never`, full manifest/byte comparison, restored SQLite opening and snapshot-history retention checks. Sources include video-like opaque bytes, sidecars, hidden, Unicode and empty files. The SQLite file is a generated test database, not an application catalog acceptance result. Destination-only neighbors must remain unchanged. Unknown SFTP hosts, untrusted FTPS certificates and incorrect virtual credentials must fail. A same-length one-bit pack corruption after the good restore must fail full data-read verification; the harness never repairs or unlocks the damaged repository.

Each subprocess has time/output bounds. Timeout cleanup targets the owned process group on macOS or the still-live owned PID tree on Windows; an integration regression checks that a descendant heartbeat stops. Credentials use scoped process environment/stdin; evidence redacts secrets. The helpers operate on invocation-owned data only. This development harness is not the production process supervisor, paced original-reader or OS credential adapter. Physical I/O limits, independent-device recovery and filesystem stream/metadata fidelity are not established by these tests.

## Actual Windows evidence, 2026-11-02

`.context/m4-protocol-all-windows-unrestricted.json` records five passed round trips: native local filesystem; rclone 1.75.2 SFTP; pyftpdlib 2.2.0 FTP and FTPS; SMBLibrary 1.5.7 SMB2/3 (SMB1 disabled). Restic is 0.19.1. All restore byte checks and history/neighbor checks pass; SFTP unknown-host and FTPS untrusted-certificate rejection pass. These receipts remain local ignored artifacts; CI uploads separate per-platform receipts.

FTPS server policy is TLS 1.2 with encrypted control/data and validated certificate. TLS 1.3 attempts timed out and are not accepted evidence. Earlier diagnostic failures are retained under `.context`; the combined sandboxed run failed because restic could not read ancestor metadata. The subsequent explicitly escalated run passed. macOS transport results remain pending its actual CI run. Neither platform's transport harness alone enables production backup/copy/replacement.

The strengthened `.context/m4-protocol-corruption-windows.json` run passes all five transports again, including same-length pack corruption rejection and all four remote wrong-credential checks. The earlier truncation experiment passes local/FTP/FTPS/SMB but SFTP exceeds the harness deadline while restic retries unexpected EOF; it remains failed evidence in `m4-protocol-failures-windows.json`, not proof of successful timeout recovery. The seven Python extraction/supervision tests pass outside the restricted sandbox; the sandboxed process-tree test failed cleanup due to denied process enumeration. No protection setting was disabled.

See [ADR-0078](decisions/ADR-0078-generated-retained-replacement-and-transport-labs.md), [M4 scope](M4_BACKUP_SYNC.md) and [dependency inventory](DEPENDENCIES.md).

First CI at `db9b322`: Windows passes all five transports/negative checks; macOS SFTP/FTP pass, local fails only deliberate damage of an owned read-only pack, FTPS fails external LibreSSL certificate setup and SMB init fails HTTP500. Corrections use an explicit owner-write bit only for the generated corruption target and the pinned cryptography certificate builder. The macOS SMB fixture prepares its empty repository and runs a bounded repository preflight before restic retries. An exploratory Windows SMBLibrary `lsf` of empty directories fails SMB signature verification; preserve that failed evidence and do not claim general share browsing/empty-directory listing support. The accepted Windows evidence is the actual scoped restic backup/check/restore sequence, with client signing and trust checks unchanged. macOS SMB remains unverified until corrected CI passes.

Credential/trust controls now use the actual rclone REST object-read adapter: first read and compare the complete encrypted repository config, repeat that same request with invalid credentials/trust, then prove the valid read still succeeds. The control uses a temporary Basic-authenticated loopback HTTP listener over the same backend, rather than treating an unrelated failed share-listing command as proof of credential rejection. It reads binary bytes without text decoding. There is no extra Python HTTP2 dependency; normal restic continues using its HTTP2 stdio helper. Generic SMB listing/cat experiments do not replace the required full backup/check/source-removal/restore sequence. The added diagnostic `lsf` preflight is removed so the prepared macOS repository can reach the actual restic workflow; all original verification assertions and SMB signing remain in force.

macOS SMB tracing identifies missing-leaf CREATE returning `STATUS_NO_SUCH_FILE` (`0xc000000f`), which the pinned client does not classify as a missing object and exposes as restic HTTP500. The disposable Impacket wrapper changes only that CREATE result to `STATUS_OBJECT_NAME_NOT_FOUND` (`0xc0000034`); both map to file-not-found in [Microsoft's error mapping](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cifs/8f11e0f3-d545-46cc-97e6-f00569e3e1bc). Other operation statuses, denied access, offline/network errors, authentication and upstream signing/packet encoding remain unchanged. Receipts explicitly identify this fixture adaptation. It is not a production workaround or proof of unmodified Impacket interoperability; the actual full round trip still must pass.

## Actual application fault evidence, 2026-11-02

Build `backup_application_probe` with `fixture-probes,backup-workflow`, then run:

```text
<venv-python> -m unittest discover -s scripts/tests -p "test_protocol_*.py"
<venv-python> scripts/m4-failure-probe.py --protocols ftp ftps --allow-insecure-ftp --application-probe target/debug/examples/backup_application_probe --evidence .context/m4-application-failure-final3.json
```

The Windows final3 matrix passes all ten cases. For each protocol, disconnect, server-writer `ENOSPC`, and lost publication acknowledgment preserve data but leave an explicitly unresolved repository lock. Credential expiry permits a fresh baseline restore after reconnect. A corrupted new pack is included but rejected by actual verification; the whole-repository check also blocks a subsequent baseline restore. None of these blocked outcomes is completed recovery.

Every case first earns a real application baseline snapshot and full restore, then changes only generated source bytes and arms the scoped fault after explicit second-job approval. Final audits compare all six baseline encrypted repository objects by native identity, mode, modification time and complete digest, plus five current source files, baseline history and destination/restore neighbors. Owned helpers must be stopped and generated OS credential references removed. Read-only reconciliation is attempted: exact no-visible-snapshot and exact pinned restic lock refusals remain distinct. There is no automatic unlock, repair, backup replay or deletion of old objects. Receipts retain actual earned flags; listing an existing snapshot may earn inclusion while later checks remain blocked. The eleven Python fault/supervision regressions pass. Earlier timeout and assertion-failure receipts remain failed evidence.

These ten cases establish disposable FTP/FTPS loopback faults, not physical disk exhaustion, cable removal, independent-device recovery or macOS execution results. Individual final3 cases took 49.841–109.992 seconds. Pinned backend-error observation stops an errored helper; fixed startup and idle-I/O bounds do not impose a short total duration on healthy progressing transfers.

Strict replacement certificates require additional native proof. On this Windows token, the adapter's read-only audit-security reopen fails with native error 1314; no privilege is enabled and audit fidelity stays unknown. macOS can qualify only the bounded no-extra-ACL/xattr/resource-fork/flags policy, successful native metadata restoration, fresh exact OLD-version restore comparison, and verified disjoint physical-device sets. No actual macOS strict replacement certificate is established by the Windows receipts or by successful remote transfer; unknown devices, remote failure domains, unsupported metadata and missing native tests remain unavailable.

## Actual SMB/SFTP disconnect and reconnect

```text
<venv-python> scripts/m4-failure-probe.py --protocols sftp smb --scenarios disconnect_upload --application-probe target/debug/examples/backup_application_probe --evidence .context/m4-application-smb-sftp-disconnect.json
```

Both Windows application cases pass: SFTP in 60.100 seconds and SMB in 65.680 seconds. The unchanged pinned SFTP server and existing disposable SMB2/3 server sit behind a generated, loopback-only TCP forwarder. After the exact second-job approval and arm marker, it requires observed growth of a new repository pack and actual forwarded bytes before dropping its owned sockets. SFTP records 258048 native new-pack bytes and 284380 armed forwarded bytes; SMB records 1048576 and 1072532 respectively. The bounded forwarder permits at most sixteen socket pairs and 256 KiB per-direction queues; arm-only pacing makes a partial upload observable. Protocol authentication/signing, binaries and production helper behavior are unchanged.

Reconnect remains refused until the native job stops and the exact disarm marker is received. After reconnect, the application attempts read-only reconciliation and baseline verification/restore. Both results are `preserved_unresolved_lock`: exact no-candidate reconciliation refuses adoption, both subsequent baseline operations report the actual repository lock, and no recovery is claimed. All six old encrypted objects retain native identity, mode, modification time and complete bytes; the five current source files, baseline history, restore/destination neighbors and generated credential cleanup pass. Owned helpers are stopped. The opaque proxy cannot certify whether a protocol client attempted a write; it reports the final native identity/metadata/byte audit, separately from the FTP filesystem mutation-prevention hook. It does not parse encrypted traffic, remove locks, retry backup or simulate a physical unplug.

The Python fault/supervision cases include actual echo forwarding, owned connection closure, reconnect only after disarm, and rejection of existing/non-growing/complete packs as interruption evidence. The proxy uses [Python 3.13 selectors](https://docs.python.org/3.13/library/selectors.html) and bounded [socket send operations](https://docs.python.org/3.13/library/socket.html#socket.socket.send).

Checkpoint `3278459` CI passes the Windows transport suite and the actual macOS SFTP disconnect case. Its macOS SMB disconnect case fails: the proxy sampled a temporary repository lock after restic had removed it, crashed with `FileNotFoundError`, and never earned fault evidence. That run remains failed evidence. The corrected progress monitor filters for eligible NEW data packs before querying metadata; a vanished NEW pack earns no evidence and resets its growth witness. It never relaxes the final frozen OLD-object audit. Dedicated regressions cover disappearing locks/packs and rejection of a missing baseline object. Actual macOS SMB retest remains required; a Windows rerun cannot substitute for it.

The corrected Windows SMB application rerun passes in 59.796 seconds (`.context/m4-smb-proxy-transient-fixed-windows.json`): 1048576 native new-pack bytes and 1072503 armed forwarded bytes precede the owned disconnect. All earlier encrypted objects, source/history/neighbors and helper-stop audits pass; recovery remains explicitly locked and unresolved. All fifteen Python fault/supervision regressions pass after the race fix. macOS SMB remains pending its corrected CI run.
