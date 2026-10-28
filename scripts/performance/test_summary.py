"""Synthetic evidence-shape tests only, never application performance evidence."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('summarize.py').resolve()
RUN = '00000000-0000-4000-8000-000000000001'


class SummaryTests(unittest.TestCase):
    def test_phase_isolation_and_unmeasured_fields_are_not_zero(self):
        with tempfile.TemporaryDirectory(prefix='stillharbour-summary-test-') as owned:
            base = Path(owned)/'.context/performance'
            for rows in (10000,100000,1000000):
                folder = base/f'{RUN}-{rows}'
                folder.mkdir(parents=True)
                data = {'phase':'P1','sourceManifestEqual':True,'sourceEntryCount':0,
                        'catalog':{'trace':{'events':[],'dropped':0},'metrics':{},'wire':{'packedBytes':42}},
                        'memory':{},'media':{'metrics':{}}}
                (folder/'baseline.json').write_text(json.dumps(data))
            # P0's older failure must not contaminate the current phase report.
            for name, phase in [('old','P0'),('current','P1')]:
                folder = base/f'native-{name}'
                folder.mkdir()
                (folder/'baseline.json').write_text(json.dumps({'phase':phase,'failure':'synthetic','sourceManifestEqual':True}))
            subprocess.run([sys.executable,str(SCRIPT),'--catalog-run',RUN,'--phase','P1'],cwd=owned,check=True,capture_output=True)
            result = json.loads((base/f'{RUN}-summary.json').read_text())
            self.assertEqual([f['run'] for f in result['failures']],['native-current'])
            self.assertEqual(result['catalog']['10000']['payloadBytes'],42)
            self.assertIsNone(result['catalog']['10000']['freezeOnlyMs'])
            self.assertIsNone(result['catalog']['10000']['catalogBytes'])
            wrong_phase = subprocess.run([sys.executable,str(SCRIPT),'--catalog-run',RUN],cwd=owned,capture_output=True)
            self.assertNotEqual(wrong_phase.returncode,0)


if __name__ == '__main__':
    unittest.main()
