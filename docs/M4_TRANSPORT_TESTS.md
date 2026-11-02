# Disposable M4 transport tests

Owner selection, 2026-11-02: build local disposable test servers. No existing account, real library, OS user, share, service or external endpoint is used.

Use Python 3.13. Build-time helper preparation verifies both archive and executable SHA-256 against `vendor/backup-helper-manifest.json`. Helper downloads are explicit; the application does not download or invoke them yet. Run from the repository root:

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
