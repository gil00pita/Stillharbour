"""Native generated fault boundaries, byte progress and baseline immutability."""
import contextlib
import copy
import errno
import importlib.util
import json
from pathlib import Path
import secrets
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location('failure_server', Path(__file__).parents[1] / 'm4_failure_server.py')
FAULT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FAULT)
PROBE_SPEC = importlib.util.spec_from_file_location('failure_probe', Path(__file__).parents[1] / 'm4-failure-probe.py')
FAILURE_PROBE = importlib.util.module_from_spec(PROBE_SPEC)
with patch.dict(sys.modules, {'m4_failure_server': FAULT}):
    PROBE_SPEC.loader.exec_module(FAILURE_PROBE)


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
