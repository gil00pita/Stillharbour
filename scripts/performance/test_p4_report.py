"""Evidence validation tests; the numbers here are synthetic, never benchmark results."""
import json
from pathlib import Path
import tempfile
import unittest

from p4_report import summarize


class ReportTests(unittest.TestCase):
    def fixture(self, root):
        manifest = {'entries': [{'size': '1', 'mtimeNs': '1793200000123456789',
                                  'inode': '123', 'device': '1', 'sha256': 'fixture'}]}
        for name in ('before', 'after'):
            (root / f'manifest-{name}.json').write_text(json.dumps(manifest))
        segment = {'label': 'scroll', 'frontend': {'capacity': 4096, 'dropped': 0, 'events': [
            {'metric': 'resident_summaries', 'value': 720},
            {'metric': 'mounted_items', 'value': 42},
        ]}, 'backend': {'trace': {'capacity': 4096, 'dropped': 0, 'events': []}}}
        for n in range(12):
            (root / f'segment-{n}.json').write_text(json.dumps(segment))
        data = {'phase': 'P4', 'sourceManifestEqual': True,
                'segments': [f'segment-{n}.json' for n in range(12)],
                'scrollRouteSteps': 120, 'scrollRouteMs': 60001,
                'textNonemptyThumbnailRequests': 0, 'randomJumpCompletionsMs': [1] * 10,
                'resizeCommandMs': [1] * 4, 'build': {
                    'profile': 'debug desktop-test; production release automation remains forbidden',
                    'executableSha256': 'a' * 64, 'cargoLockSha256': 'b' * 64,
                    'npmLockSha256': 'c' * 64}}
        (root / 'baseline.json').write_text(json.dumps(data))
        return data, segment

    def test_scoped_success_keeps_missing_measurements_unknown(self):
        with tempfile.TemporaryDirectory(prefix='stillharbour-p4-report-') as owned:
            root = Path(owned)
            self.fixture(root)
            report = summarize(root)
            self.assertEqual(report['status'], 'passed_scoped_native_diagnostics')
            self.assertEqual(report['maxResidentSummaries'], 720)
            self.assertIsNone(report['metrics']['ipc_ms'])
            self.assertIsNone(report['observedMemory']['peakRssBytes'])
            self.assertIn('not run', report['releasePresentation'])

    def test_loss_manifest_failure_short_route_and_image_requests_fail(self):
        with tempfile.TemporaryDirectory(prefix='stillharbour-p4-report-') as owned:
            root = Path(owned)
            data, segment = self.fixture(root)
            data.update({'sourceManifestEqual': False, 'scrollRouteSteps': 6,
                         'textNonemptyThumbnailRequests': 1, 'failure': 'retained'})
            data['build'].pop('executableSha256')
            segment['frontend']['dropped'] = 4
            segment['backend']['trace']['dropped'] = 1
            (root / 'segment-0.json').write_text(json.dumps(segment))
            (root / 'baseline.json').write_text(json.dumps(data))
            report = summarize(root)
            self.assertEqual(report['status'], 'failed')
            self.assertTrue({'native_route_failed', 'trace_loss', 'source_manifest_mismatch',
                             'incomplete_60_second_route', 'text_image_requests', 'build_identity_missing'} <= set(report['failures']))
            self.assertEqual(report['frontendDropped'], 4)

    def test_phase_and_segment_bounds_are_enforced(self):
        with tempfile.TemporaryDirectory(prefix='stillharbour-p4-report-') as owned:
            root = Path(owned)
            data, _ = self.fixture(root)
            data['phase'] = 'P2'
            (root / 'baseline.json').write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                summarize(root)
            data.update({'phase': 'P4', 'segments': ['../segment-0.json']})
            (root / 'baseline.json').write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                summarize(root)


if __name__ == '__main__':
    unittest.main()
