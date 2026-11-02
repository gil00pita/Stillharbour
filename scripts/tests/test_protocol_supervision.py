"""A timeout must stop this test's helper descendants as well as its parent."""
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location('protocol_probe', Path(__file__).parents[1] / 'm4-protocol-probe.py')
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


class SupervisionTests(unittest.TestCase):
    def test_application_diagnostics_retain_only_fixed_phase_and_numeric_markers(self):
        stderr = '\n'.join([
            'private/path and password must not be included',
            'STILLHARBOUR_GENERATED_BACKUP_DIAGNOSTIC phase=repository_inspection code=WorkerFailed',
            'STILLHARBOUR_GENERATED_HELPER_DIAGNOSTIC command=init exit_code=126',
            'STILLHARBOUR_GENERATED_BACKUP_DIAGNOSTIC phase=private/path code=Conflict',
        ])
        result = PROBE.application_diagnostics(stderr)
        self.assertIn('phase=repository_inspection code=WorkerFailed', result)
        self.assertIn('command=init exit_code=126', result)
        self.assertNotIn('private', result)
        self.assertNotIn('password', result)

    def test_failed_tree_termination_never_kills_only_the_parent(self):
        process = Mock()
        process.pid = 5678
        process.poll.return_value = None
        with patch.object(PROBE.sys, 'platform', 'win32'), \
                patch.object(PROBE.subprocess, 'CREATE_NO_WINDOW', 0, create=True), \
                patch.dict(PROBE.os.environ, {'SystemRoot': 'C:\\Windows'}), \
                patch.object(PROBE.subprocess, 'run', return_value=Mock(returncode=1)):
            with self.assertRaisesRegex(RuntimeError, 'Could not stop the owned protocol process tree'):
                PROBE.stop_tree(process)
        process.kill.assert_not_called()
        process.wait.assert_not_called()

    def test_timeout_stops_owned_descendant_heartbeat(self):
        with tempfile.TemporaryDirectory(prefix='stillharbour-supervision-test-') as temporary:
            base = Path(temporary).resolve()
            marker = base / 'heartbeat'
            descendant = 'import time\nfrom pathlib import Path\nwhile True:\n Path("heartbeat").write_text(str(time.time_ns()))\n time.sleep(0.05)\n'
            parent = 'import subprocess,sys,time\nsubprocess.Popen([sys.executable,"-c",' + repr(descendant) + '])\ntime.sleep(60)\n'
            with self.assertRaisesRegex(RuntimeError, 'exceeded its time/output budget'):
                PROBE.run([sys.executable, '-c', parent], base, dict(os.environ), timeout=2)
            observed = marker.read_bytes()
            time.sleep(0.2)
            self.assertEqual(marker.read_bytes(), observed, 'Helper descendant survived timeout cleanup')


if __name__ == '__main__':
    unittest.main()
