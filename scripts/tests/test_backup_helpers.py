"""Untrusted archive regression cases for the pinned build-time helper reader."""
import bz2
import importlib.util
import io
from pathlib import Path
import stat
import unittest
import warnings
import zipfile

SPEC = importlib.util.spec_from_file_location('backup_helpers', Path(__file__).parents[1] / 'prepare-backup-helpers.py')
HELPERS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HELPERS)


class ExtractionTests(unittest.TestCase):
    def artifact(self, member='reviewed/tool'):
        return {'member': member, 'executableSha256': HELPERS.digest(b'pinned executable')}

    def archive(self, entries):
        output = io.BytesIO()
        with warnings.catch_warnings():
            warnings.simplefilter('ignore', UserWarning)
            with zipfile.ZipFile(output, 'w') as package:
                for name, content in entries:
                    package.writestr(name, content)
        return output.getvalue()

    def test_only_reviewed_member_is_read_without_extracting_unrelated_paths(self):
        content = self.archive([('../../untrusted', b'ignored'), ('reviewed/tool', b'pinned executable')])
        self.assertEqual(HELPERS.unpack(content, self.artifact()), b'pinned executable')

    def test_duplicate_executable_is_rejected(self):
        content = self.archive([('reviewed/tool', b'pinned executable')] * 2)
        with self.assertRaises(SystemExit):
            HELPERS.unpack(content, self.artifact())

    def test_link_member_is_rejected_even_when_content_hash_matches(self):
        link = zipfile.ZipInfo('reviewed/tool')
        link.create_system = 3
        link.external_attr = (stat.S_IFLNK | 0o777) << 16
        with self.assertRaises(SystemExit):
            HELPERS.unpack(self.archive([(link, b'pinned executable')]), self.artifact())

    def test_changed_executable_is_rejected(self):
        with self.assertRaises(SystemExit):
            HELPERS.unpack(self.archive([('reviewed/tool', b'changed')]), self.artifact())

    def test_bzip_trailing_stream_is_rejected(self):
        content = bz2.compress(b'pinned executable') + bz2.compress(b'extra')
        with self.assertRaises(SystemExit):
            HELPERS.unpack(content, self.artifact(None))

    def test_bounded_bzip_output_is_rejected(self):
        original = HELPERS.MAX_EXECUTABLE
        try:
            HELPERS.MAX_EXECUTABLE = 8
            with self.assertRaises(SystemExit):
                HELPERS.unpack(bz2.compress(b'pinned executable'), self.artifact(None))
        finally:
            HELPERS.MAX_EXECUTABLE = original


if __name__ == '__main__':
    unittest.main()
