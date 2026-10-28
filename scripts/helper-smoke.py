#!/usr/bin/env python3
"""Developer-only probe. Every read/write is inside a new owned TemporaryDirectory.

Helpers must be explicitly supplied from checksum-reviewed local installations.
This script is never bundled or called by the application.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import struct
import subprocess
import tempfile
import zlib


def generated_png():
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 16, 8, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress((b'\0' + b'\x14\x64\x50' * 16) * 8)) + chunk(b'IEND', b''))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--restic', type=Path, required=True)
    parser.add_argument('--rclone', type=Path, required=True)
    parser.add_argument('--exiftool', type=Path, required=True)
    parser.add_argument('--perl-library', type=Path, required=True)
    args = parser.parse_args()
    args.restic, args.rclone, args.exiftool, args.perl_library = (
        p.resolve(strict=True) for p in (args.restic, args.rclone, args.exiftool, args.perl_library))
    with tempfile.TemporaryDirectory(prefix='stillharbour-helper-m0-') as name:
        root = Path(name).resolve()
        source = root / 'generated-source'
        source.mkdir()
        hostile = source / '-quote" percent%\n港📷.png'
        hostile.write_bytes(generated_png())
        (source / '.hidden').write_bytes(b'preserve hidden bytes')
        (source / 'companion.xmp').write_bytes(b'<fixture>preserve metadata</fixture>')
        (source / 'unknown.data').write_bytes(bytes(range(256)))
        manifest = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in source.iterdir()}
        # A config that would leave a marker if loaded. Both cwd and HOME are disposable.
        marker = root / 'config-was-executed'
        (root / '.ExifTool_config').write_text(f'open(my $fh, ">", "{marker}"); print $fh "unsafe"; close($fh); 1;\n')
        environment = {'PATH': '/usr/bin:/bin', 'HOME': str(root), 'TMPDIR': str(root),
                       'PERL5LIB': str(args.perl_library), 'LC_ALL': 'en_US.UTF-8'}

        def run(argv):
            result = subprocess.run([str(x) for x in argv], cwd=root, env=environment,
                                    capture_output=True, text=True, timeout=60, check=True)
            return result.stdout

        metadata = json.loads(run(['/usr/bin/perl', args.exiftool, '-config', '', '-j', '-n',
                                   '-ImageWidth', '-ImageHeight', '--', hostile]))
        assert metadata[0]['ImageWidth'] == 16 and metadata[0]['ImageHeight'] == 8
        assert not marker.exists(), 'Untrusted ExifTool configuration executed'
        assert manifest == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in source.iterdir()}
        print('ExifTool: hostile filename is data, local/home config disabled, original bytes unchanged')
        password = root / 'fixture-password'
        descriptor = os.open(password, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, 'w') as stream:
            stream.write(secrets.token_urlsafe(32))
        base = [args.restic, '--repo', root / 'repository', '--password-file', password,
                '--cache-dir', root / 'restic-cache']
        run(base + ['init'])
        run(base + ['backup', '--json', source])
        run(base + ['check', '--read-data'])
        # Deletion is restricted to this known generated directory owned by this run.
        assert source.parent == root and source.name == 'generated-source' and not source.is_symlink()
        shutil.rmtree(source)
        target = root / 'restored'
        run(base + ['restore', 'latest', '--target', target])
        restored = target.joinpath(*source.parts[1:])
        actual = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in restored.iterdir()}
        assert actual == manifest
        print(f'restic: local encrypted snapshot, full data check, restore and byte comparison passed ({len(actual)} generated files)')
        config = root / 'empty-rclone.conf'
        config.write_text('')
        listing = json.loads(run([args.rclone, '--config', config, '--cache-dir', root / 'rclone-cache',
                                  '--log-level', 'ERROR', '--local-encoding', 'None', 'lsjson', restored]))
        # rclone exposes control characters with its Standard encoding even when
        # local disk encoding is None. Record this boundary; do not turn display
        # names into native execution paths or claim exact filename fidelity.
        expected_display = {name.replace('\n', '\u240a') for name in manifest}
        assert {p['Name'] for p in listing} == expected_display
        print('rclone: isolated config/local inventory passed; newline is encoded as U+240A, so native-name round trips remain gated')
        print('Same-device disposable probe only; no independent recovery, filesystem-metadata or packaged-helper claim.')


if __name__ == '__main__':
    main()
