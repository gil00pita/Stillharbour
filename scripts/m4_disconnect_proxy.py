"""Generated loopback-only TCP interruption; no SSH/SMB parsing or helper changes."""
import os
from pathlib import Path
import selectors
import socket
import time

from m4_failure_server import FaultScope, LIMIT

MAX_CONNECTIONS = 16
MAX_QUEUE = 262144
ARMED_BYTES_PER_SECOND = 262144


def progressing_pack(scope, observed):
    """Only a new, partially written pack can establish upload progress."""
    count = 0
    for path in scope.root.rglob('*'):
        count += 1
        if count > 8192 or path.is_symlink():
            raise RuntimeError('Generated proxy repository scope exceeds its bound.')
        if path.is_file():
            relative = scope.relative(path)
            size = path.stat().st_size
            if size > LIMIT:
                raise RuntimeError('Generated proxy pack exceeds its bound.')
            if 'data' in Path(relative).parts and relative not in scope.baseline:
                previous = observed.get(relative)
                observed[relative] = size
                if previous is not None and size > previous and 32768 <= size < 2097152:
                    return size
    return 0


class Forwarder:
    def __init__(self, scope, listen_port, backend_port):
        if (scope.scenario != 'disconnect_upload' or listen_port == backend_port
                or not all(1024 < port <= 65535 for port in (listen_port, backend_port))):
            raise RuntimeError('Invalid fixed generated proxy endpoint.')
        self.scope = scope
        self.backend_port = backend_port
        self.selector = selectors.DefaultSelector()
        self.listener = socket.socket()
        self.listener.bind(('127.0.0.1', listen_port))
        self.listener.listen(MAX_CONNECTIONS)
        self.listener.setblocking(False)
        self.selector.register(self.listener, selectors.EVENT_READ, None)
        self.peers = {}
        self.forwarded = 0
        self.armed_forwarded = 0
        self.observed = {}

    def close_pair(self, endpoint):
        peer = self.peers.get(endpoint, {}).get('peer')
        for sock in (endpoint, peer):
            if sock in self.peers:
                if self.peers[sock]['registered']:
                    self.selector.unregister(sock)
                del self.peers[sock]
                sock.close()

    def close(self):
        for sock in list(self.peers):
            self.close_pair(sock)
        self.selector.close()
        self.listener.close()

    def accept(self):
        client, _ = self.listener.accept()
        if ((self.scope.triggered and not self.scope.disarmed)
                or len(self.peers) >= MAX_CONNECTIONS * 2):
            client.close()
            return
        try:
            backend = socket.create_connection(('127.0.0.1', self.backend_port), timeout=2)
        except OSError:
            client.close()
            return
        now = time.monotonic()
        for sock, peer, upstream in ((client, backend, False), (backend, client, True)):
            sock.setblocking(False)
            self.peers[sock] = {'peer': peer, 'queue': bytearray(), 'upstream': upstream,
                                'tokens': 0.0, 'updated': now, 'registered': True}
            self.selector.register(sock, selectors.EVENT_READ)

    def interest(self, sock):
        info = self.peers[sock]
        peer = self.peers.get(info['peer'])
        events = selectors.EVENT_READ if peer is not None and len(peer['queue']) < MAX_QUEUE else 0
        now = time.monotonic()
        info['tokens'] = min(65536, info['tokens'] + (now - info['updated']) * ARMED_BYTES_PER_SECOND)
        info['updated'] = now
        if info['queue'] and (not self.scope.armed or not info['upstream'] or info['tokens'] >= 1024):
            events |= selectors.EVENT_WRITE
        if not events:
            if info['registered']:
                self.selector.unregister(sock)
                info['registered'] = False
        elif info['registered']:
            self.selector.modify(sock, events)
        else:
            self.selector.register(sock, events)
            info['registered'] = True

    def step(self):
        self.scope.poll()
        if self.scope.armed and not self.scope.triggered and self.armed_forwarded:
            size = progressing_pack(self.scope, self.observed)
            if size:
                self.scope.write('failure.proxy.json', {
                    'ownerNonce': self.scope.owner, 'scenario': 'disconnect_upload',
                    'nativeNewPackBytes': size, 'armedForwardedBytes': self.armed_forwarded,
                    'ownedConnectionPairs': len(self.peers) // 2,
                    'maxConnectionPairs': MAX_CONNECTIONS, 'maxQueueBytes': MAX_QUEUE,
                    'physicalUnplug': False})
                self.scope.trigger(size)
                for sock in list(self.peers):
                    self.close_pair(sock)
        for sock in self.peers:
            self.interest(sock)
        for key, events in self.selector.select(timeout=0.01):
            sock = key.fileobj
            if sock is self.listener:
                self.accept()
                continue
            if sock not in self.peers:
                continue
            info = self.peers[sock]
            peer = self.peers.get(info['peer'])
            try:
                if events & selectors.EVENT_READ and peer is not None:
                    available = MAX_QUEUE - len(peer['queue'])
                    if available > 0:
                        data = sock.recv(min(65536, available))
                        if not data:
                            self.close_pair(sock)
                            continue
                        peer['queue'].extend(data)
                if events & selectors.EVENT_WRITE and info['queue']:
                    if self.scope.armed and info['upstream']:
                        limit = min(len(info['queue']), int(info['tokens']))
                    else:
                        limit = len(info['queue'])
                    if limit:
                        sent = sock.send(info['queue'][:limit])
                        del info['queue'][:sent]
                        self.forwarded += sent
                        if self.scope.armed and info['upstream']:
                            info['tokens'] -= sent
                            self.armed_forwarded += sent
                if sock in self.peers:
                    self.interest(sock)
                    self.interest(info['peer'])
            except BlockingIOError:
                pass
            except (ConnectionError, OSError):
                self.close_pair(sock)


def main():
    scope = FaultScope(Path(os.environ['STILLHARBOUR_PROTOCOL_SCOPE']),
                       os.environ['STILLHARBOUR_PROTOCOL_OWNER'], 'disconnect_upload')
    proxy = Forwarder(scope, int(os.environ['STILLHARBOUR_PROTOCOL_PORT']),
                      int(os.environ['STILLHARBOUR_BACKEND_PORT']))
    try:
        while True:
            proxy.step()
    finally:
        proxy.close()


if __name__ == '__main__':
    main()
