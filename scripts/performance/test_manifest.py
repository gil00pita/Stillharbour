"""Lossless stat transport regression; generated temporary fixtures only."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from manifest import manifest

class StatTransport(unittest.TestCase):
    def test_lossless_stat_preserves_large_values_through_javascript_and_legacy_is_unchanged(self):
        with tempfile.TemporaryDirectory(prefix='stillharbour-stat-fixture-') as owned:
            root=Path(owned); source=root/'generated.txt'; source.write_bytes(b'Generated stat fixture')
            os.utime(source,ns=(1793200000123456789,1793200000123456789))
            actual=source.stat()
            legacy=manifest(root); self.assertIsInstance(legacy['entries'][0]['mtimeNs'],int)
            output=subprocess.check_output(['python3',str(Path(__file__).with_name('manifest.py')),str(root),'--lossless-stat'])
            # Verify the consumer that exposed the bug, not just Python JSON.
            result=subprocess.check_output(['node','-e','process.stdout.write(JSON.stringify(JSON.parse(process.argv[1])))',output.decode()])
            record=json.loads(result)['entries'][0]
            self.assertEqual(record['mtimeNs'],str(actual.st_mtime_ns))
            self.assertEqual(record['inode'],str(actual.st_ino))
            self.assertEqual(record['device'],str(actual.st_dev))
            self.assertEqual(record['size'],str(actual.st_size))
            self.assertEqual(legacy['entries'][0]['sha256'],record['sha256'])

if __name__=='__main__': unittest.main()
