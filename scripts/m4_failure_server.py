"""Generated-only pyftpdlib fault adapters; no production server or source paths.

Reviewed against pinned pyftpdlib 2.2.0: AbstractedFS.open supplies the DTP file,
DTP.handle_read wraps writer OSError, and DTP.close closes that file BEFORE
responding 226. on_file_received is AFTER 226, so it cannot model lost ACKs.
"""
import errno
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import weakref

SCENARIOS = ('disconnect_upload', 'quota_exhausted', 'credential_expired',
             'publication_ack_lost', 'repository_corrupted')
LIMIT = 128 * 1024 * 1024


def snapshot_objects(root):
    result = {}
    total = 0
    for path in sorted(root.rglob('*')):
        if path.is_symlink():
            raise RuntimeError('Unexpected link in generated repository.')
        if path.is_file():
            info = path.stat()
            total += info.st_size
            if len(result) >= 8192 or total > LIMIT:
                raise RuntimeError('Generated repository audit exceeds its bound.')
            digest = hashlib.sha256()
            with path.open('rb') as stream:
                for chunk in iter(lambda: stream.read(65536), b''):
                    digest.update(chunk)
            result[path.relative_to(root).as_posix()] = (
                info.st_dev, info.st_ino, info.st_size,
                stat.S_IMODE(info.st_mode), info.st_mtime_ns, digest.hexdigest())
    return result


class FaultScope:
    def __init__(self, base, owner, scenario):
        self.base = Path(base).resolve()
        if (not re.fullmatch(r'[0-9a-f]{32}', owner)
                or not self.base.name.startswith(f'stillharbour-m4-failure-{owner}-')
                or (self.base / 'owner').read_bytes() != owner.encode()
                or scenario not in SCENARIOS):
            raise RuntimeError('Generated failure scope authorization failed.')
        info = self.base.stat()
        self.identity = (info.st_dev, info.st_ino)
        self.owner, self.scenario = owner, scenario
        self.root = self.base / 'server'
        self.control = self.base / 'failure-control'
        if self.control.is_symlink() or (self.control / 'owner').read_bytes() != owner.encode():
            raise RuntimeError('Generated fixed control directory authorization failed.')
        info = self.control.stat()
        self.control_identity = (info.st_dev, info.st_ino)
        self.armed = False
        self.disarmed = False
        self.triggered = False
        self.baseline = None
        self.handlers = weakref.WeakSet()
        self.payload_bytes = 0
        self.publication = False
        self.protected_attempts = 0

    def boundary(self):
        info = self.base.stat()
        if (self.base.is_symlink() or (info.st_dev, info.st_ino) != self.identity
                or (self.base / 'owner').read_bytes() != self.owner.encode()):
            raise RuntimeError('Generated failure scope membership changed.')
        info = self.control.stat()
        if (self.control.is_symlink() or (info.st_dev, info.st_ino) != self.control_identity
                or (self.control / 'owner').read_bytes() != self.owner.encode()):
            raise RuntimeError('Generated fixed control directory membership changed.')

    def marker(self, name):
        self.boundary()
        path = self.control / name
        if path.exists():
            if path.is_symlink() or path.stat().st_size > 32:
                raise RuntimeError('Generated failure control marker changed.')
            data = path.read_bytes()
            # Native create-new is visible before its bounded write finishes.
            # A matching incomplete prefix grants no fault authority yet.
            if len(data) < 32 and self.owner.encode().startswith(data):
                return False
            if data != self.owner.encode():
                raise RuntimeError('Generated failure control marker changed.')
            return True
        return False

    def write(self, name, value):
        self.boundary()
        data = json.dumps(value, separators=(',', ':')).encode()
        if len(data) > 4096:
            raise RuntimeError('Generated fault evidence exceeds its bound.')
        temporary = self.control / f'.{name}.{self.owner}.new'
        with temporary.open('xb') as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        # Atomic complete publication with no replacement on Windows/APFS/NTFS.
        # A pre-existing control/evidence file is retained and fails the case.
        if os.name == 'nt':
            # Win32 rename is atomic and refuses an existing destination;
            # CreateHardLinkW can require sharing rights held by the native app.
            os.rename(temporary, self.control / name)
        else:
            os.link(temporary, self.control / name)
            temporary.unlink()

    def evidence(self):
        return {'ownerNonce': self.owner, 'scenario': self.scenario,
                'payloadBytes': self.payload_bytes,
                'publicationObserved': self.publication,
                'injectionApplied': self.triggered}

    def poll(self):
        if not self.armed and not self.disarmed and self.marker('failure.arm'):
            self.baseline = snapshot_objects(self.root)
            if not self.baseline:
                raise RuntimeError('Failure injection requires actual baseline objects.')
            self.write('failure.baseline.json', {'ownerNonce': self.owner,
                                                'scenario': self.scenario,
                                                'objects': self.baseline})
            self.armed = True
            print(f'STILLHARBOUR_GENERATED_FAULT phase=armed scenario={self.scenario} objects={len(self.baseline)}', flush=True)
            if self.scenario == 'credential_expired':
                # Existing authenticated pooled connections must also expire;
                # a future valid-password login supplies the rejection proof.
                for handler in list(self.handlers):
                    abrupt_close(handler)
        if not self.disarmed and self.marker('failure.disarm'):
            self.armed = False
            self.disarmed = True
            current = snapshot_objects(self.root)
            unchanged = (self.baseline is not None and all(
                current.get(name) == value for name, value in self.baseline.items()))
            self.write('failure.audit.json', {
                'ownerNonce': self.owner, 'scenario': self.scenario,
                'oldObjects': len(self.baseline or {}),
                'oldObjectsUnchanged': unchanged,
                'protectedMutationAttempts': self.protected_attempts})
            self.write('failure.ack', self.evidence())
            print(f'STILLHARBOUR_GENERATED_FAULT phase=disarmed scenario={self.scenario} applied={int(self.triggered)}', flush=True)

    def relative(self, path):
        self.boundary()
        candidate = Path(path)
        if candidate.is_symlink():
            raise RuntimeError('Generated fault target cannot be a link.')
        resolved = candidate.resolve()
        if not resolved.is_relative_to(self.root) or resolved == self.root:
            raise RuntimeError('Generated fault target escaped the server scope.')
        return resolved.relative_to(self.root).as_posix()

    def protect(self, path):
        relative = self.relative(path)
        if self.baseline is not None and relative in self.baseline:
            self.protected_attempts += 1
            if self.protected_attempts == 1:
                self.write('failure.protected-attempt.json', {
                    'ownerNonce': self.owner, 'scenario': self.scenario,
                    'baselineMutationAttempted': True})
            raise PermissionError(errno.EACCES, 'Baseline fixture objects are immutable.')
        return relative

    def data(self, path):
        return 'data' in Path(self.relative(path)).parts

    def snapshot(self, path):
        relative = Path(self.relative(path))
        return ('snapshots' in relative.parts
                and re.fullmatch(r'[0-9a-f]{64}', relative.name) is not None)

    def trigger(self, count, publication=False):
        if not self.triggered:
            self.payload_bytes = count
            self.publication = publication
            self.triggered = True
            self.write('failure.triggered', self.evidence())
            print(f'STILLHARBOUR_GENERATED_FAULT phase=triggered scenario={self.scenario} bytes={count} publication={int(publication)}', flush=True)

    def received(self, path, channel, count):
        if not self.armed or self.triggered or not count:
            return
        if self.scenario == 'publication_ack_lost' and self.snapshot(path):
            print(f'STILLHARBOUR_GENERATED_FAULT phase=publication_ready scenario={self.scenario} bytes={count}', flush=True)
            channel._stillharbour_pending_ack = count
        if self.scenario == 'repository_corrupted' and self.data(path):
            self.protect(path)
            info = Path(path).stat()
            if info.st_size < 2 or info.st_size > LIMIT:
                raise RuntimeError('Generated pack corruption target is outside bounds.')
            Path(path).chmod(stat.S_IMODE(info.st_mode) | stat.S_IWUSR)
            with Path(path).open('r+b') as stream:
                stream.seek(info.st_size // 2)
                byte = stream.read(1)
                stream.seek(info.st_size // 2)
                stream.write(bytes([byte[0] ^ 1]))
                stream.flush()
                os.fsync(stream.fileno())
            self.trigger(count)


def abrupt_close(handler):
    # TLS mixin's error branch deliberately skips graceful TLS shutdown. This
    # models a dropped owned connection without changing TLS trust or policy.
    handler._error = True
    handler.close()


class FaultFile:
    def __init__(self, stream, control, channel):
        self.stream, self.control, self.channel = stream, control, channel
        self.written = 0
        self.finished = False

    def __getattr__(self, name):
        return getattr(self.stream, name)

    def write(self, data):
        controller = self.control
        controller.poll()
        failing = controller.armed and controller.data(self.stream.name)
        if failing and controller.scenario == 'quota_exhausted':
            remaining = max(0, 32768 - self.written) if not controller.triggered else 0
            if remaining:
                self.written += self.stream.write(data[:remaining])
                self.stream.flush()
                os.fsync(self.stream.fileno())
            controller.trigger(self.written)
            raise OSError(errno.ENOSPC, 'Generated server quota exhausted.')
        result = self.stream.write(data)
        self.written += result
        if failing and controller.scenario == 'disconnect_upload' and self.written >= 32768:
            self.stream.flush()
            os.fsync(self.stream.fileno())
            controller.trigger(self.written)
            abrupt_close(self.channel)
        return result

    def close(self):
        if self.finished:
            return
        self.finished = True
        if not self.stream.closed:
            self.stream.flush()
            os.fsync(self.stream.fileno())
            self.stream.close()
        self.control.received(self.stream.name, self.channel, self.written)


def attach(handler, authorizer, base, owner, scenario):
    from pyftpdlib.authorizers import AuthenticationFailed, DummyAuthorizer
    controller = FaultScope(base, owner, scenario)
    base_fs = handler.abstracted_fs

    class FaultFS(base_fs):
        def open(self, filename, mode):
            controller.poll()
            if any(flag in mode for flag in 'wa+'):
                controller.protect(filename)
                if (controller.armed and controller.triggered
                        and controller.scenario == 'disconnect_upload'
                        and controller.data(filename)):
                    raise ConnectionResetError(errno.ECONNRESET, 'Generated persistent disconnection.')
                return FaultFile(super().open(filename, mode), controller, self.cmd_channel)
            return super().open(filename, mode)

        def remove(self, path):
            controller.protect(path)
            return super().remove(path)

        def rename(self, source, destination):
            controller.protect(source)
            controller.protect(destination)
            result = super().rename(source, destination)
            if controller.armed and not controller.triggered and controller.scenario == 'publication_ack_lost' and controller.snapshot(destination):
                self.cmd_channel._stillharbour_pending_ack = Path(destination).stat().st_size
            return result

    class FaultAuthorizer(DummyAuthorizer):
        def validate_authentication(self, username, password, channel):
            super().validate_authentication(username, password, channel)
            controller.poll()
            if controller.armed and controller.scenario == 'credential_expired':
                controller.trigger(0)
                raise AuthenticationFailed('Generated credential expired after baseline initialization.')

    class FaultHandler(handler):
        abstracted_fs = FaultFS
        auth_failed_timeout = 0.1

        def on_connect(self):
            controller.handlers.add(self)
            return super().on_connect()

        def on_disconnect(self):
            controller.handlers.discard(self)
            return super().on_disconnect()

        def pre_process_command(self, line, command, argument):
            controller.poll()
            return super().pre_process_command(line, command, argument)

        def respond(self, response, logfun=None):
            pending = getattr(self, '_stillharbour_pending_ack', 0)
            if (pending and controller.armed and not controller.triggered
                    and response.startswith(('226 ', '250 '))):
                self._stillharbour_pending_ack = 0
                controller.trigger(pending, publication=True)
                abrupt_close(self)
                return None
            return super().respond(response) if logfun is None else super().respond(response, logfun)

    replacement = FaultAuthorizer()
    replacement.user_table = authorizer.user_table
    return FaultHandler, replacement, controller
