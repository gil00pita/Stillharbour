#!/usr/bin/env python3
"""Disposable loopback snapshot transport tests; never accepts a source library.

Only fixed manifest-verified helpers execute. Every repository, server, key,
password and source belongs to this invocation. Receipts contain no secrets.
This is transport evidence, not a production application workflow test.
"""
import argparse
import base64
import contextlib
import datetime
import hashlib
import importlib.metadata
import ipaddress
import json
import logging
import os
from pathlib import Path
import platform
import secrets
import signal
import shutil
import socket
import sqlite3
import stat
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
OUTPUT_LIMIT = 16 * 1024 * 1024


def redact(text, environment):
    for name, value in environment.items():
        if any(word in name for word in ('PASS', 'TOKEN')) and value:
            text = text.replace(value, '[fixture-secret]')
    return text


def fixture_certificate(base):
    from cryptography import x509
    from cryptography.hazmat.primitives import hashes, serialization
    from cryptography.hazmat.primitives.asymmetric import rsa
    from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID
    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    subject = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, 'Stillharbour disposable loopback')])
    now = datetime.datetime.now(datetime.timezone.utc)
    certificate = (x509.CertificateBuilder().subject_name(subject).issuer_name(subject)
                   .public_key(key.public_key()).serial_number(x509.random_serial_number())
                   .not_valid_before(now - datetime.timedelta(minutes=5))
                   .not_valid_after(now + datetime.timedelta(days=1))
                   .add_extension(x509.BasicConstraints(ca=True, path_length=0), critical=True)
                   .add_extension(x509.SubjectAlternativeName([x509.IPAddress(ipaddress.ip_address('127.0.0.1'))]), critical=False)
                   .add_extension(x509.ExtendedKeyUsage([ExtendedKeyUsageOID.SERVER_AUTH]), critical=False)
                   .sign(key, hashes.SHA256()))
    owned(base, base / 'server.key').write_bytes(key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.TraditionalOpenSSL, serialization.NoEncryption()))
    owned(base, base / 'server.crt').write_bytes(certificate.public_bytes(serialization.Encoding.PEM))


def stop_tree(process):
    if process.poll() is not None:
        return
    if sys.platform == 'win32':
        # This test-owned process is still live; do not target by image name.
        taskkill = Path(os.environ['SystemRoot']) / 'System32/taskkill.exe'
        result = subprocess.run([str(taskkill), '/PID', str(process.pid), '/T', '/F'],
                                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL, timeout=10,
                                creationflags=subprocess.CREATE_NO_WINDOW)
        if result.returncode != 0 and process.poll() is None:
            process.kill()
    else:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    process.wait(timeout=10)


def owned(base, path):
    resolved = path.resolve()
    if not resolved.is_relative_to(base) or resolved == base or path.is_symlink():
        raise RuntimeError('Generated protocol scope containment failed.')
    return resolved


def helpers():
    target = 'windows-x86_64' if sys.platform == 'win32' else 'macos-aarch64'
    manifest = json.loads((ROOT / 'vendor/backup-helper-manifest.json').read_text())
    result = {}
    for entry in manifest['artifacts']:
        if entry['platform'] == target:
            path = ROOT / 'vendor/backup-helpers' / target / entry['executable']
            if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != entry['executableSha256']:
                raise RuntimeError('Pinned helper integrity check failed.')
            result[entry['helper']] = path.resolve()
    if set(result) != {'restic', 'rclone'}:
        raise RuntimeError('Host helper platform is unavailable.')
    return result, manifest


def run(argv, cwd, env, *, input_text=None, success=True, timeout=90):
    # File capture avoids anonymous-pipe deadlocks and bounds retained output.
    with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        process = subprocess.Popen([str(a) for a in argv], cwd=cwd, env=env,
                                   stdin=subprocess.PIPE if input_text is not None else subprocess.DEVNULL,
                                   stdout=out, stderr=err,
                                   start_new_session=sys.platform != 'win32',
                                   creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == 'win32' else 0)
        if input_text is not None:
            process.stdin.write(input_text.encode())
            process.stdin.close()
        deadline = time.monotonic() + timeout
        while process.poll() is None:
            if time.monotonic() > deadline or os.fstat(out.fileno()).st_size > OUTPUT_LIMIT or os.fstat(err.fileno()).st_size > OUTPUT_LIMIT:
                stop_tree(process)
                err.seek(0)
                diagnostic = err.read(OUTPUT_LIMIT).decode(errors='replace')[-2500:]
                diagnostic = redact(diagnostic, env)
                phase = next((str(a) for a in argv if str(a) in {'init','backup','check','ls','restore','snapshots','lsd','obscure','req'}),'helper')
                raise RuntimeError(f'Owned {Path(argv[0]).name} {phase} exceeded its time/output budget: {diagnostic}')
            time.sleep(0.01)
        out.seek(0)
        err.seek(0)
        stdout, stderr = out.read(OUTPUT_LIMIT + 1), err.read(OUTPUT_LIMIT + 1)
        if len(stdout) > OUTPUT_LIMIT or len(stderr) > OUTPUT_LIMIT:
            raise RuntimeError('Helper output exceeded its budget.')
        if success and process.returncode != 0:
            # Test paths are generated; passwords never enter argv or this text.
            diagnostic = redact(stderr.decode(errors='replace')[-3000:], env)
            raise RuntimeError(f'Owned {Path(argv[0]).name} {argv[-1]} failed ({process.returncode}): {diagnostic}')
        return process.returncode, stdout.decode(errors='strict'), stderr.decode(errors='replace')


def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def config_read_control(binaries, protocol, base, environment):
    remote = 'fixture:FIXTURE/repository' if protocol == 'smb' else 'fixture:repository'
    listen_port = port()
    password = secrets.token_urlsafe(24)
    control_env = {**environment, 'RCLONE_USER': 'fixture-http', 'RCLONE_PASS': password}
    command = [binaries['rclone'], 'serve', 'restic', '--addr', f'127.0.0.1:{listen_port}', '--transfers', '1', '--checkers', '1', '--buffer-size', '64k', '--multi-thread-streams', '0', '--retries', '1', '--low-level-retries', '1', '--contimeout', '5s', '--timeout', '15s', remote]
    # Use the same REST object-read adapter as the actual stdio helper. HTTP/1
    # loopback is test-only, Basic-authenticated, and serves encrypted repo bytes.
    with server(command, base, control_env, listen_port, allow_failed_start=True) as process:
        if process is None:
            return None
        authorization = base64.b64encode(f'fixture-http:{password}'.encode()).decode()
        request = urllib.request.Request(f'http://127.0.0.1:{listen_port}/config', headers={'Authorization': f'Basic {authorization}'})
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                body = response.read(OUTPUT_LIMIT + 1)
                if len(body) > OUTPUT_LIMIT:
                    raise RuntimeError('Config control exceeded its output bound.')
                return body if response.status == 200 else None
        except urllib.error.HTTPError:
            return None


@contextlib.contextmanager
def server(argv, base, environment, listen_port, *, allow_failed_start=False):
    log = open(base / 'server.log', 'ab')
    process = subprocess.Popen([str(a) for a in argv], cwd=base, env=environment,
                               stdin=subprocess.DEVNULL, stdout=log, stderr=log,
                               start_new_session=sys.platform != 'win32',
                               creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == 'win32' else 0)
    try:
        deadline = time.monotonic() + 20
        while True:
            if process.poll() is not None:
                if allow_failed_start:
                    yield None
                    return
                diagnostic = (base/'server.log').read_bytes()[-3500:].decode(errors='replace')
                for name,value in environment.items():
                    if any(word in name for word in ('PASS','TOKEN')) and value:
                        diagnostic=diagnostic.replace(value,'[fixture-secret]')
                raise RuntimeError(f'Disposable protocol server stopped during startup: {diagnostic}')
            try:
                with socket.create_connection(('127.0.0.1', listen_port), timeout=0.2):
                    break
            except OSError:
                if time.monotonic() > deadline:
                    raise RuntimeError('Disposable protocol server failed to become ready.')
                time.sleep(0.05)
        yield process
    except Exception as error:
        diagnostic = (base / 'server.log').read_bytes()[-3500:].decode(errors='replace')
        raise RuntimeError(f'{error}; generated server diagnostic: {redact(diagnostic, environment)}') from error
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                stop_tree(process)
        log.close()


def server_smb():
    from impacket import smbserver
    from impacket.ntlm import compute_lmhash, compute_nthash
    from impacket.nt_errors import STATUS_NO_SUCH_FILE, STATUS_OBJECT_NAME_NOT_FOUND
    from impacket.smb3structs import SMB2_CREATE
    base = Path(os.environ['STILLHARBOUR_PROTOCOL_SCOPE']).resolve()
    token = os.environ['STILLHARBOUR_PROTOCOL_TOKEN']
    if not base.is_relative_to(Path(tempfile.gettempdir()).resolve()) or not base.name.startswith('stillharbour-m4-protocol-'):
        raise RuntimeError('SMB child scope is not generated.')
    if (base / 'owner').read_text() != os.environ['STILLHARBOUR_PROTOCOL_OWNER']:
        raise RuntimeError('SMB child scope owner mismatch.')
    root = owned(base, base / 'server')
    logging.getLogger().setLevel(logging.CRITICAL)
    logging.getLogger('impacket.smbserver').setLevel(logging.ERROR)
    backend = smbserver.SimpleSMBServer(listenAddress='127.0.0.1', listenPort=int(os.environ['STILLHARBOUR_PROTOCOL_PORT']))
    backend.addShare('FIXTURE', str(root), 'Generated Stillharbour protocol fixture', readOnly='no')
    backend.setSMB2Support(True)
    backend.addCredential('fixture', 0, compute_lmhash(token), compute_nthash(token))
    config = backend.getServer().getServerConfig()
    config.set('global', 'anonymous_logon', 'no')
    backend.getServer().setServerConfig(config)
    backend.getServer().processConfigFile()
    # The test server's public hooks record numeric operations/statuses only.
    # Never emit packets, auth challenges/responses, keys or file names.
    trace_count = 0
    for command in range(19):
        original = backend.getServer().hookSmb2Command(command, None)
        if original is None:
            continue
        def traced(*arguments, command=command, original=original):
            nonlocal trace_count
            trace_count += 1
            report = trace_count <= 128
            if report:
                print(f'SMB2 operation={command} enter', flush=True)
            try:
                response = original(*arguments)
            except Exception as error:
                if report:
                    print(f'SMB2 operation={command} errorType={type(error).__name__}', flush=True)
                raise
            # This fixture uses the Windows-compatible missing-name status for
            # CREATE only. The pinned client does not classify Impacket's other
            # ENOENT status as missing. Signing/encoding still belongs upstream.
            if command == SMB2_CREATE and response[2] == STATUS_NO_SUCH_FILE:
                response = (response[0], response[1], STATUS_OBJECT_NAME_NOT_FOUND)
            if report:
                print(f'SMB2 operation={command} status={int(response[2])}', flush=True)
            return response
        backend.getServer().hookSmb2Command(command, traced)
    backend.start()


def server_ftp():
    from pyftpdlib.authorizers import DummyAuthorizer
    from pyftpdlib.handlers import FTPHandler, TLS_FTPHandler
    from pyftpdlib.servers import FTPServer
    base = Path(os.environ['STILLHARBOUR_PROTOCOL_SCOPE']).resolve()
    if not base.is_relative_to(Path(tempfile.gettempdir()).resolve()) or not base.name.startswith('stillharbour-m4-protocol-'):
        raise RuntimeError('FTP child scope is not generated.')
    if (base / 'owner').read_text() != os.environ['STILLHARBOUR_PROTOCOL_OWNER']:
        raise RuntimeError('FTP child scope owner mismatch.')
    authorizer = DummyAuthorizer()
    authorizer.add_user('fixture', os.environ['STILLHARBOUR_PROTOCOL_TOKEN'], str(owned(base, base / 'server')), perm='elradfmwMT')
    secure = os.environ['STILLHARBOUR_PROTOCOL_TYPE'] == 'ftps'
    handler = TLS_FTPHandler if secure else FTPHandler
    handler.authorizer = authorizer
    handler.permit_foreign_addresses = False
    handler.masquerade_address = '127.0.0.1'
    handler.use_sendfile = False
    if secure:
        handler.certfile = str(owned(base, base / 'server.crt'))
        handler.keyfile = str(owned(base, base / 'server.key'))
        handler.tls_control_required = True
        handler.tls_data_required = True
        # Exercise an explicit, recorded TLS 1.2 server capability. The client
        # keeps secure cipher defaults and full certificate verification.
        from OpenSSL import SSL
        context = SSL.Context(SSL.TLS_SERVER_METHOD)
        context.set_min_proto_version(SSL.TLS1_2_VERSION)
        context.set_max_proto_version(SSL.TLS1_2_VERSION)
        context.use_certificate_chain_file(handler.certfile)
        context.use_privatekey_file(handler.keyfile)
        handler.ssl_context = context
    logging.getLogger().setLevel(logging.ERROR)
    with FTPServer(('127.0.0.1', int(os.environ['STILLHARBOUR_PROTOCOL_PORT'])), handler) as backend:
        backend.serve_forever()


def snapshot_source(base, token):
    source = base / 'generated-source'
    source.mkdir()
    (source / 'Album').mkdir()
    (source / 'Album/photo.bin').write_bytes(bytes(range(256)) * 257)
    (source / 'Album/photo.xmp').write_bytes(b'<fixture>unique companion</fixture>')
    (source / '.hidden').write_bytes(b'hidden source bytes')
    (source / '港📷.bin').write_bytes(b'Unicode native name')
    (source / 'empty.bin').write_bytes(b'')
    (source / f'owner-{token}.marker').write_bytes(token.encode())
    live = sqlite3.connect(base / 'live-fixture.sqlite')
    live.execute('PRAGMA journal_mode=WAL')
    live.executescript('CREATE TABLE identity(id TEXT PRIMARY KEY); CREATE TABLE annotations(tag TEXT);')
    live.execute('INSERT INTO identity VALUES(?)', (token,))
    live.execute('INSERT INTO annotations VALUES(?)', ('committed fixture annotation',))
    live.commit()
    with contextlib.closing(sqlite3.connect(source / 'Catalog.snapshot.sqlite')) as snapshot:
        live.backup(snapshot)
    live.close()
    with contextlib.closing(sqlite3.connect(source / 'Catalog.snapshot.sqlite')) as snapshot:
        assert snapshot.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        assert snapshot.execute('SELECT id FROM identity').fetchone()[0] == token
    return source


def contents(source):
    result = {}
    for path in source.rglob('*'):
        if path.is_symlink():
            raise RuntimeError('Unexpected link in generated fixture.')
        if path.is_file():
            result[path.relative_to(source).as_posix()] = path.read_bytes()
    return result


def probe(protocol, binaries, versions, allow_ftp):
    if protocol == 'ftp' and not allow_ftp:
        raise RuntimeError('Plain FTP testing requires --allow-insecure-ftp explicitly.')
    with tempfile.TemporaryDirectory(prefix='stillharbour-m4-protocol-') as temporary:
        base = Path(temporary).resolve()
        token = secrets.token_urlsafe(24)
        owner_nonce = secrets.token_hex(16)
        (base / 'owner').write_text(owner_nonce)
        remote = base / 'server'
        remote.mkdir()
        if protocol == 'smb' and sys.platform == 'darwin':
            (remote / 'repository').mkdir()
        neighbor = remote / 'outside-repository.bin'
        neighbor.write_bytes(b'preserved destination-only neighbor')
        config = base / 'rclone.conf'
        config.write_text('')
        env = {name: os.environ[name] for name in ('SystemRoot', 'WINDIR', 'COMSPEC') if name in os.environ}
        env.update({'TEMP': str(base), 'TMP': str(base), 'TMPDIR': str(base), 'RCLONE_CONFIG': str(config),
                    'RESTIC_PASSWORD': secrets.token_urlsafe(32), 'RCLONE_CACHE_DIR': str(base / 'rclone-cache'),
                    'RCLONE_BWLIMIT': '10M', 'RCLONE_TRANSFERS': '1', 'RCLONE_CHECKERS': '1',
                    'RCLONE_BUFFER_SIZE': '64k', 'RCLONE_MULTI_THREAD_STREAMS': '0', 'RESTIC_READ_CONCURRENCY': '1'})
        _, obscured, _ = run([binaries['rclone'], 'obscure', '-'], base, env, input_text=token)
        env['RCLONE_CONFIG_FIXTURE_PASS'] = obscured.strip()
        listen_port = port()
        server_env = {**env, 'RCLONE_USER': 'fixture', 'RCLONE_PASS': token}
        arguments = [binaries['rclone'], 'serve', 'sftp' if protocol == 'sftp' else 'ftp', remote, '--addr', f'127.0.0.1:{listen_port}', '--log-level', 'ERROR']
        trust = None
        if protocol == 'local':
            manager = contextlib.nullcontext()
            repository = str(remote / 'repository')
        elif protocol == 'sftp':
            from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
            from cryptography.hazmat.primitives import serialization
            key = Ed25519PrivateKey.generate()
            (base / 'host-key').write_bytes(key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.OpenSSH, serialization.NoEncryption()))
            (base / 'host-key').chmod(0o600)
            public = key.public_key().public_bytes(serialization.Encoding.OpenSSH, serialization.PublicFormat.OpenSSH).decode().split()[:2]
            trust = base / 'known-hosts'
            trust.write_text(f'[127.0.0.1]:{listen_port} {" ".join(public)}\n')
            arguments += ['--key', base / 'host-key']
            config.write_text(f'[fixture]\ntype = sftp\nhost = 127.0.0.1\nport = {listen_port}\nuser = fixture\nknown_hosts_file = {trust.as_posix()}\ndisable_hashcheck = true\n')
            manager = server(arguments, base, server_env, listen_port)
            repository = 'rclone:fixture:repository'
        elif protocol in ('ftp', 'ftps'):
            if protocol == 'ftps':
                fixture_certificate(base)
                env['RCLONE_CA_CERT'] = str(base / 'server.crt')
            # One transfer + one checker need a third connection for metadata;
            # rclone documents smaller pools as a potential deadlock.
            config.write_text(f'[fixture]\ntype = ftp\nhost = 127.0.0.1\nport = {listen_port}\nuser = fixture\nexplicit_tls = {str(protocol=="ftps").lower()}\nno_check_certificate = false\nconcurrency = 3\n')
            server_env.update({'STILLHARBOUR_PROTOCOL_SCOPE':str(base),'STILLHARBOUR_PROTOCOL_TOKEN':token,'STILLHARBOUR_PROTOCOL_OWNER':owner_nonce,'STILLHARBOUR_PROTOCOL_PORT':str(listen_port),'STILLHARBOUR_PROTOCOL_TYPE':protocol})
            manager = server([sys.executable, Path(__file__).resolve(), '--ftp-server'], base, server_env, listen_port)
            repository = 'rclone:fixture:repository'
        elif protocol == 'smb':
            server_env.update({'STILLHARBOUR_PROTOCOL_SCOPE': str(base), 'STILLHARBOUR_PROTOCOL_TOKEN': token,'STILLHARBOUR_PROTOCOL_OWNER':owner_nonce, 'STILLHARBOUR_PROTOCOL_PORT': str(listen_port)})
            config.write_text(f'[fixture]\ntype = smb\nhost = 127.0.0.1\nport = {listen_port}\nuser = fixture\ndomain = WORKGROUP\n')
            if sys.platform == 'win32':
                powershell = Path('C:/Program Files/PowerShell/7/pwsh.exe')
                manager = server([powershell,'-NoProfile','-NonInteractive','-File',ROOT/'scripts/m4-smb-server.ps1'],base,server_env,listen_port)
            else:
                manager = server([sys.executable, Path(__file__).resolve(), '--smb-server'], base, server_env, listen_port)
            repository = 'rclone:fixture:FIXTURE/repository'
        else:
            raise RuntimeError('Unknown fixed test protocol.')
        # restic parses rclone.program as an argument vector, never via a shell.
        quoted_program = "'" + str(binaries['rclone']).replace("'", "'\"'\"'") + "'"
        command = [binaries['restic'], '--repo', repository, '--cache-dir', base / 'restic-cache', '--limit-upload', '10240', '--json']
        if protocol != 'local':
            command += ['-o', f'rclone.program={quoted_program}', '-o', 'rclone.timeout=20s', '-o', 'rclone.args=serve restic --stdio --transfers 1 --checkers 1 --buffer-size 64k --multi-thread-streams 0 --contimeout 5s --timeout 15s --retries 1 --low-level-retries 1']
        with manager:
            source = snapshot_source(base, owner_nonce)
            expected = contents(source)
            run(command + ['init'], base, env)
            _, result, _ = run(command + ['backup', '--', source], base, env)
            summary = [json.loads(line) for line in result.splitlines() if line.startswith('{') and json.loads(line).get('message_type') == 'summary']
            if len(summary) != 1 or not summary[0].get('snapshot_id'):
                raise RuntimeError('Backup did not return a concrete snapshot summary.')
            snapshot_id = summary[0]['snapshot_id']
            _, listing, _ = run(command + ['ls', snapshot_id], base, env)
            nodes = [json.loads(line) for line in listing.splitlines() if line.startswith('{')]
            roots = [node['path'] for node in nodes if node.get('type') == 'dir' and node.get('path', '').endswith('/generated-source')]
            if len(roots) != 1 or '..' in roots[0].split('/'):
                raise RuntimeError('Snapshot scope cannot be safely mapped to a restore root.')
            run(command + ['check'], base, env)
            run(command + ['check', '--read-data'], base, env)
            # Remove only the exact generated source with an unchanged manifest.
            if owned(base, source) != base / 'generated-source' or contents(source) != expected:
                raise RuntimeError('Refusing removal of changed/unowned generated source.')
            shutil.rmtree(source)
            target = base / 'restored-new'
            run(command + ['restore', f'{snapshot_id}:{roots[0]}', '--target', target, '--verify', '--overwrite', 'never'], base, env)
            matches = list(target.rglob(f'owner-{owner_nonce}.marker'))
            if len(matches) != 1 or contents(matches[0].parent) != expected:
                raise RuntimeError('Full restored byte comparison failed.')
            with contextlib.closing(sqlite3.connect(matches[0].parent / 'Catalog.snapshot.sqlite')) as snapshot:
                assert snapshot.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
                assert snapshot.execute('SELECT id FROM identity').fetchone()[0] == owner_nonce
            _, snapshots, _ = run(command + ['snapshots'], base, env)
            if snapshot_id not in {item['id'] for item in json.loads(snapshots)}:
                raise RuntimeError('Source removal unexpectedly erased snapshot history.')
            if protocol != 'local':
                good_config = config_read_control(binaries, protocol, base, env)
                if good_config != (remote / 'repository/config').read_bytes():
                    raise RuntimeError('Positive credential/trust control differs from the generated repository config.')
            trust_rejected = None
            if protocol == 'sftp':
                trusted = trust.read_text()
                trust.write_text('')
                rejected_config = config_read_control(binaries, protocol, base, env)
                trust.write_text(trusted)
                if rejected_config is not None:
                    raise RuntimeError('Unknown SFTP host was incorrectly accepted.')
                trust_rejected = True
            elif protocol == 'ftps':
                untrusted = dict(env)
                del untrusted['RCLONE_CA_CERT']
                rejected_config = config_read_control(binaries, protocol, base, untrusted)
                if rejected_config is not None:
                    raise RuntimeError('Untrusted FTPS certificate was incorrectly accepted.')
                trust_rejected = True
            credential_rejected = None
            if protocol != 'local':
                invalid = dict(env)
                _, wrong, _ = run([binaries['rclone'], 'obscure', '-'], base, env, input_text=secrets.token_urlsafe(32))
                invalid['RCLONE_CONFIG_FIXTURE_PASS'] = wrong.strip()
                rejected_config = config_read_control(binaries, protocol, base, invalid)
                if rejected_config is not None:
                    raise RuntimeError('Incorrect protocol credential was accepted.')
                after_config = config_read_control(binaries, protocol, base, env)
                if after_config != good_config:
                    raise RuntimeError('Positive credential control did not recover after the negative test.')
                credential_rejected = True
            # Damage only a pack in this run's generated repository, after the
            # good restore has been verified. Never repair, unlock or prune it.
            packs = sorted((remote / 'repository' / 'data').rglob('*'))
            packs = [path for path in packs if path.is_file()]
            if not packs:
                raise RuntimeError('Generated repository contains no data packs for failure injection.')
            pack = owned(base, packs[0])
            original_pack = pack.read_bytes()
            if len(original_pack) < 2:
                raise RuntimeError('Generated data pack is unexpectedly empty.')
            damaged_pack = bytearray(original_pack)
            damaged_pack[len(damaged_pack)//2] ^= 1
            # The local restic backend deliberately makes generated packs 0400
            # on macOS. Change only this owned fixture's mode to inject damage.
            pack.chmod(stat.S_IMODE(pack.stat().st_mode) | stat.S_IWUSR)
            pack.write_bytes(damaged_pack)
            code, _, _ = run(command + ['check', '--read-data'], base, env, success=False)
            if code == 0:
                raise RuntimeError('Corrupted repository pack was incorrectly accepted.')
            if neighbor.read_bytes() != b'preserved destination-only neighbor':
                raise RuntimeError('Destination-only neighbor changed.')
            return {'protocol': protocol, 'result': 'passed', 'nativeOs': sys.platform, 'osVersion': platform.version(),
                    'restic': versions['resticVersion'], 'rclone': versions['rcloneVersion'],
                    'server': ('SMBLibrary 1.5.7 (user-mode, SMB1 disabled)' if sys.platform=='win32' else f'Impacket {importlib.metadata.version("impacket")} + fixture CREATE missing-name status mapping') if protocol == 'smb' else f'pyftpdlib {importlib.metadata.version("pyftpdlib")}' if protocol in ('ftp','ftps') else f'rclone {versions["rcloneVersion"]}' if protocol != 'local' else 'native local filesystem',
                    'sourceRemoved': True, 'restoredFiles': len(expected), 'fullByteCompared': True,
                    'structuralCheck': True, 'fullDataRead': True, 'catalogSnapshotIndependentlyOpenable': True,
                    'catalogSnapshotKind': 'generated SQLite fixture', 'snapshotRetainedAfterSourceRemoval': True,
                    'destinationOnlyPreserved': True, 'untrustedEndpointRejected': trust_rejected,
                    'incorrectCredentialRejected': credential_rejected, 'corruptedPackRejected': True,
                    'credentialTrustPositiveControl': protocol != 'local',
                    'plainFtpExplicitlyAcknowledged': protocol == 'ftp', 'loopbackOnly': True,
                    'tlsServerPolicy': 'TLS1.2, control/data TLS required, certificate verified' if protocol == 'ftps' else None,
                    'productionApplicationAcceptance': False, 'physicalIoCapClaimed': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--protocols', nargs='+', choices=['local', 'sftp', 'ftp', 'ftps', 'smb'], default=['local'])
    parser.add_argument('--allow-insecure-ftp', action='store_true')
    parser.add_argument('--evidence', type=Path, default=ROOT / '.context/m4-protocol-receipt.json')
    parser.add_argument('--smb-server', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--ftp-server', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.smb_server:
        server_smb()
        return
    if args.ftp_server:
        server_ftp()
        return
    evidence = args.evidence.resolve()
    if not evidence.is_relative_to(ROOT / '.context'):
        raise SystemExit('Evidence must stay in the workspace .context directory.')
    binaries, versions = helpers()
    receipts = []
    for protocol in args.protocols:
        try:
            receipts.append(probe(protocol, binaries, versions, args.allow_insecure_ftp))
            print(f'{protocol}: full data-read / removed generated source / restored bytes verified', flush=True)
        except Exception as error:
            receipts.append({'protocol': protocol, 'result': 'failed', 'reason': str(error), 'productionApplicationAcceptance': False})
            print(f'{protocol}: failed; see generated receipt', flush=True)
        evidence.parent.mkdir(parents=True, exist_ok=True)
        evidence.write_text(json.dumps({'scope': 'generated-loopback-only', 'receipts': receipts}, indent=2))
    if any(item['result'] != 'passed' for item in receipts):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
