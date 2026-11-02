"""Native generated fault boundaries, byte progress and baseline immutability."""
import contextlib
import copy
import errno
import importlib.util
import json
from pathlib import Path
import secrets
import socket
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location('failure_server', Path(__file__).parents[1] / 'm4_failure_server.py')
FAULT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FAULT)
PROBE_SPEC = importlib.util.spec_from_file_location('failure_probe', Path(__file__).parents[1] / 'm4-failure-probe.py')
FAILURE_PROBE = importlib.util.module_from_spec(PROBE_SPEC)
with patch.dict(sys.modules, {'m4_failure_server': FAULT}):
    PROBE_SPEC.loader.exec_module(FAILURE_PROBE)
    PROXY_SPEC = importlib.util.spec_from_file_location('disconnect_proxy', Path(__file__).parents[1] / 'm4_disconnect_proxy.py')
    PROXY = importlib.util.module_from_spec(PROXY_SPEC)
    PROXY_SPEC.loader.exec_module(PROXY)


@contextlib.contextmanager
def fixture(scenario):
    owner = secrets.token_hex(16)
    with tempfile.TemporaryDirectory(prefix=f'stillharbour-m4-failure-{owner}-') as temporary:
        base = Path(temporary).resolve()
        (base / 'owner').write_bytes(owner.encode())
        (base / 'failure-control').mkdir()
        (base / 'failure-control/owner').write_bytes(owner.encode())
        (base / 'server/data/aa').mkdir(parents=True)
        (base / 'server/snapshots').mkdir()
        original = base / 'server/data/aa' / ('a' * 64)
        original.write_bytes(b'unique generated baseline encrypted object')
        controller = FAULT.FaultScope(base, owner, scenario)
        (controller.control / 'failure.arm').write_bytes(owner.encode())
        controller.poll()
        yield base, controller, original


class FaultBoundaryTests(unittest.TestCase):
    def test_proxy_requires_new_growing_partial_pack_not_existing_or_complete_object(self):
        with fixture('disconnect_upload') as (base, controller, original):
            observed = {}
            self.assertEqual(PROXY.progressing_pack(controller, observed), 0)
            new = base / 'server/data/aa' / ('b' * 64)
            new.write_bytes(b'x')
            self.assertEqual(PROXY.progressing_pack(controller, observed), 0)
            new.write_bytes(b'x' * 65536)
            self.assertEqual(PROXY.progressing_pack(controller, observed), 65536)
            self.assertEqual(PROXY.progressing_pack(controller, observed), 0)
            new.write_bytes(b'x' * 2097152)
            self.assertEqual(PROXY.progressing_pack(controller, observed), 0)
            self.assertEqual(original.read_bytes(), b'unique generated baseline encrypted object')

    def test_proxy_forwards_then_disconnects_owned_sockets_and_reconnects_only_after_disarm(self):
        with fixture('disconnect_upload') as (base, controller, _):
            backend = socket.socket()
            backend.bind(('127.0.0.1', 0))
            backend.listen(16)
            backend.settimeout(0.1)
            stopped = threading.Event()

            def echo():
                while not stopped.is_set():
                    try:
                        stream, _ = backend.accept()
                    except socket.timeout:
                        continue
                    with stream:
                        stream.settimeout(0.1)
                        while not stopped.is_set():
                            try:
                                data = stream.recv(4096)
                                if not data:
                                    break
                                stream.sendall(data)
                            except socket.timeout:
                                continue
                            except OSError:
                                break

            worker = threading.Thread(target=echo)
            worker.start()
            reserved = socket.socket()
            reserved.bind(('127.0.0.1', 0))
            proxy_port = reserved.getsockname()[1]
            reserved.close()
            proxy = PROXY.Forwarder(controller, proxy_port, backend.getsockname()[1])

            def round_trip():
                client = socket.create_connection(('127.0.0.1', proxy_port), timeout=1)
                client.sendall(b'owned generated echo')
                client.setblocking(False)
                received = bytearray()
                until = time.monotonic() + 5
                while len(received) < len(b'owned generated echo'):
                    self.assertLess(time.monotonic(), until)
                    proxy.step()
                    try:
                        part = client.recv(4096)
                        self.assertTrue(part)
                        received.extend(part)
                    except BlockingIOError:
                        pass
                self.assertEqual(received, b'owned generated echo')
                return client

            client = None
            reconnect = None
            try:
                client = round_trip()
                pack = base / 'server/data/aa' / ('c' * 64)
                pack.write_bytes(b'x')
                proxy.step()
                pack.write_bytes(b'x' * 65536)
                proxy.step()
                self.assertTrue(controller.triggered)
                self.assertEqual(client.recv(1), b'')
                self.assertFalse(proxy.peers)
                with socket.create_connection(('127.0.0.1', proxy_port), timeout=1) as refused:
                    proxy.step()
                    self.assertEqual(refused.recv(1), b'')
                (controller.control / 'failure.disarm').write_bytes(controller.owner.encode())
                proxy.step()
                reconnect = round_trip()
                self.assertTrue(controller.disarmed)
                proof = json.loads((controller.control / 'failure.proxy.json').read_bytes())
                self.assertGreater(proof['armedForwardedBytes'], 0)
                self.assertEqual(proof['nativeNewPackBytes'], 65536)
                self.assertFalse(proof['physicalUnplug'])
            finally:
                if client is not None:
                    client.close()
                if reconnect is not None:
                    reconnect.close()
                proxy.close()
                stopped.set()
                worker.join(2)
                backend.close()
                self.assertFalse(worker.is_alive())

    def test_unresolved_lock_requires_exact_refusals_and_never_earns_recovery(self):
        evidence = {
            'outcome': 'preserved_unresolved_lock', 'helpersStopped': True,
            'restoreNeighborPreserved': True, 'localNeighborPreserved': True,
            'sourceFilesCompared': 5, 'repositoryLocked': True,
            'recoveryCompleted': False, 'baselineRestoredAfterFault': False,
            'baselineVerification': {'state': 'blocked_repository_locked',
                                     'repositoryLocked': True, 'verified': False},
            'baselineRestore': {'state': 'blocked_repository_locked',
                                'repositoryLocked': True, 'restoredCompared': False},
            'reconciliation': {'attempted': True, 'repositoryLocked': True},
            'nativeFailure': {'jobState': 'needs_recovery', 'included': False,
                              'structuralChecked': False, 'dataReadVerified': False,
                              'restoredCompared': False},
        }
        FAILURE_PROBE.validate_outcome(evidence, 'disconnect_upload')
        no_candidate = copy.deepcopy(evidence)
        no_candidate['reconciliation'] = {
            'attempted': True, 'repositoryLocked': False,
            'noCompletedSnapshotVisible': True, 'snapshotPresent': False,
            'job': copy.deepcopy(evidence['nativeFailure']),
        }
        FAILURE_PROBE.validate_outcome(no_candidate, 'disconnect_upload')
        for field, value in (('snapshotPresent', True), ('noCompletedSnapshotVisible', False)):
            changed = copy.deepcopy(no_candidate)
            changed['reconciliation'][field] = value
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                FAILURE_PROBE.validate_outcome(changed, 'disconnect_upload')
        for section, field, value in (
                (None, 'helpersStopped', False),
                (None, 'recoveryCompleted', True),
                (None, 'baselineRestoredAfterFault', True),
                ('baselineVerification', 'state', 'failed'),
                ('baselineRestore', 'repositoryLocked', False),
                ('reconciliation', 'attempted', False),
                ('reconciliation', 'repositoryLocked', False),
                ('nativeFailure', 'included', True)):
            changed = copy.deepcopy(evidence)
            (changed if section is None else changed[section])[field] = value
            with self.subTest(section=section, field=field), self.assertRaises(RuntimeError):
                FAILURE_PROBE.validate_outcome(changed, 'disconnect_upload')

    def test_partial_native_control_write_never_arms_or_disarms_a_fault(self):
        with fixture('disconnect_upload') as (_, controller, _):
            marker = controller.control / 'failure.disarm'
            marker.write_bytes(controller.owner[:8].encode())
            controller.poll()
            self.assertFalse(controller.disarmed)
            marker.write_bytes(controller.owner.encode())
            controller.poll()
            self.assertTrue(controller.disarmed)

    def test_quota_raises_actual_enospc_after_retaining_positive_partial_bytes(self):
        with fixture('quota_exhausted') as (base, controller, original):
            before = FAULT.snapshot_objects(controller.root)
            target = base / 'server/data/aa' / ('b' * 64)
            stream = FAULT.FaultFile(target.open('wb'), controller, Mock())
            try:
                with self.assertRaises(OSError) as error:
                    stream.write(b'x' * 65536)
                self.assertEqual(error.exception.errno, errno.ENOSPC)
            finally:
                stream.close()
            self.assertGreater(target.stat().st_size, 0)
            self.assertLess(target.stat().st_size, 65536)
            self.assertEqual(FAULT.snapshot_objects(controller.root)[original.relative_to(controller.root).as_posix()], before[original.relative_to(controller.root).as_posix()])
            self.assertGreater(json.loads((controller.control / 'failure.triggered').read_bytes())['payloadBytes'], 0)
            (controller.control / 'failure.disarm').write_bytes(controller.owner.encode())
            controller.poll()
            self.assertTrue(json.loads((controller.control / 'failure.audit.json').read_bytes())['oldObjectsUnchanged'])

    def test_disconnection_happens_after_actual_owned_writer_progress(self):
        with fixture('disconnect_upload') as (base, controller, _):
            channel = Mock()
            target = base / 'server/data/aa' / ('b' * 64)
            stream = FAULT.FaultFile(target.open('wb'), controller, channel)
            try:
                stream.write(b'x' * 65536)
                self.assertEqual(target.stat().st_size, 65536)
                channel.close.assert_called_once()
                self.assertTrue(controller.triggered)
            finally:
                stream.close()

    def test_corruption_changes_only_new_complete_pack_and_never_repairs_it(self):
        with fixture('repository_corrupted') as (base, controller, original):
            baseline = original.read_bytes()
            target = base / 'server/data/aa' / ('b' * 64)
            payload = b'new generated encrypted pack' * 256
            stream = FAULT.FaultFile(target.open('wb'), controller, Mock())
            stream.write(payload)
            self.assertFalse(controller.triggered)
            stream.close()
            damaged = target.read_bytes()
            self.assertEqual(len(damaged), len(payload))
            self.assertEqual(sum(a != b for a, b in zip(damaged, payload)), 1)
            self.assertEqual(original.read_bytes(), baseline)
            (controller.control / 'failure.disarm').write_bytes(controller.owner.encode())
            controller.poll()
            self.assertEqual(target.read_bytes(), damaged)

    def test_snapshot_write_alone_does_not_claim_acknowledgment_loss(self):
        with fixture('publication_ack_lost') as (base, controller, _):
            channel = Mock()
            target = base / 'server/snapshots' / ('b' * 64)
            stream = FAULT.FaultFile(target.open('wb'), controller, channel)
            stream.write(b'complete encrypted snapshot')
            stream.close()
            self.assertGreater(channel._stillharbour_pending_ack, 0)
            self.assertFalse(controller.triggered)
            self.assertFalse((controller.control / 'failure.triggered').exists())

    def test_existing_control_is_never_replaced_and_nonce_mismatch_is_rejected(self):
        with fixture('disconnect_upload') as (base, controller, _):
            marker = controller.control / 'failure.triggered'
            marker.write_bytes(b'pre-existing occupant')
            with self.assertRaises(FileExistsError):
                controller.write('failure.triggered', controller.evidence())
            self.assertEqual(marker.read_bytes(), b'pre-existing occupant')
            (controller.control / 'failure.disarm').write_bytes(b'not-the-owned-nonce')
            with self.assertRaisesRegex(RuntimeError, 'marker changed'):
                controller.poll()

    def test_baseline_mutation_attempt_is_blocked_and_fails_audit(self):
        with fixture('quota_exhausted') as (base, controller, original):
            baseline = original.read_bytes()
            with self.assertRaises(PermissionError):
                controller.protect(original)
            (controller.control / 'failure.disarm').write_bytes(controller.owner.encode())
            controller.poll()
            audit = json.loads((controller.control / 'failure.audit.json').read_bytes())
            self.assertTrue(audit['oldObjectsUnchanged'])
            self.assertEqual(audit['protectedMutationAttempts'], 1)
            self.assertEqual(original.read_bytes(), baseline)


if __name__ == '__main__':
    unittest.main()
