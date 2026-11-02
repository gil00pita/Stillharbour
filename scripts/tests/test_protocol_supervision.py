"""A timeout must stop this test's helper descendants as well as its parent."""
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest

SPEC = importlib.util.spec_from_file_location('protocol_probe', Path(__file__).parents[1] / 'm4-protocol-probe.py')
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


class SupervisionTests(unittest.TestCase):
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
