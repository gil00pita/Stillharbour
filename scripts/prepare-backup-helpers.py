#!/usr/bin/env python3
"""Build-time preparation of fixed, hash-pinned upstream helpers. No runtime download.

Only the named executable member is read; archives are never extracted as trees.
The manifest is repository-owned input, not an imported repository manifest.
"""
import argparse
import bz2
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import stat
import sys
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
MAX_ARCHIVE = 100 * 1024 * 1024
MAX_EXECUTABLE = 150 * 1024 * 1024


def digest(data):
    return hashlib.sha256(data).hexdigest()


def host_platform():
    if sys.platform == 'win32' and platform.machine().lower() in ('amd64', 'x86_64'):
        return 'windows-x86_64'
    if sys.platform == 'darwin' and platform.machine().lower() in ('arm64', 'aarch64'):
        return 'macos-aarch64'
    raise SystemExit('This helper gate supports Windows x64 and macOS Apple Silicon only.')


def verified_archive(path, artifact, download):
    if not path.exists():
        if not download:
            raise SystemExit(f'Missing reviewed archive: {path.name}; use --download explicitly.')
        request = urllib.request.Request(artifact['url'], headers={'User-Agent': 'Stillharbour-build'})
        with urllib.request.urlopen(request, timeout=60) as response:
            data = response.read(MAX_ARCHIVE + 1)
        if len(data) > MAX_ARCHIVE or digest(data) != artifact['archiveSha256']:
            raise SystemExit('Downloaded helper archive failed its pinned size/hash check.')
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(data)
    if path.is_symlink() or path.stat().st_size > MAX_ARCHIVE:
        raise SystemExit('Helper archive is a link or exceeds its bound.')
    data = path.read_bytes()
    if digest(data) != artifact['archiveSha256']:
        raise SystemExit(f'Helper archive checksum mismatch: {path.name}')
    return data


def unpack(data, artifact):
    if artifact['member'] is None:
        decoder = bz2.BZ2Decompressor()
        executable = decoder.decompress(data, max_length=MAX_EXECUTABLE + 1)
        if not decoder.eof or decoder.unused_data:
            raise SystemExit('Unexpected restic bzip2 structure or oversized executable.')
    else:
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            matches = [i for i in archive.infolist() if i.filename == artifact['member']]
            if len(matches) != 1:
                raise SystemExit('Missing or repeated reviewed executable member.')
            member = matches[0]
            if member.is_dir() or stat.S_ISLNK(member.external_attr >> 16) or member.file_size > MAX_EXECUTABLE:
                raise SystemExit('Unsafe or oversized executable member.')
            executable = archive.read(member)
    if len(executable) > MAX_EXECUTABLE or digest(executable) != artifact['executableSha256']:
        raise SystemExit('Helper executable failed its pinned checksum.')
    return executable


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--download', action='store_true')
    parser.add_argument('--platform', choices=['windows-x86_64', 'macos-aarch64'])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    selected = args.platform or host_platform()
    manifest = json.loads((ROOT / 'vendor/backup-helper-manifest.json').read_text())
    output = ROOT / 'vendor/backup-helpers' / selected
    output.mkdir(parents=True, exist_ok=True)
    if output.is_symlink() or not output.resolve().is_relative_to(ROOT):
        raise SystemExit('Helper output escaped the workspace.')
    for artifact in manifest['artifacts']:
        if artifact['platform'] != selected:
            continue
        target = output / artifact['executable']
        if target.is_symlink():
            raise SystemExit('Refusing a linked helper executable.')
        if args.check:
            if not target.is_file() or digest(target.read_bytes()) != artifact['executableSha256']:
                raise SystemExit(f'Missing or changed pinned helper: {artifact["helper"]}')
            continue
        cache = ROOT / '.context/m4-helpers' / artifact['helper'] / artifact['archive']
        executable = unpack(verified_archive(cache, artifact, args.download), artifact)
        if target.exists():
            if digest(target.read_bytes()) != artifact['executableSha256']:
                raise SystemExit('Existing helper differs; review rather than overwrite it.')
        else:
            with target.open('xb') as stream:
                stream.write(executable)
                stream.flush()
                os.fsync(stream.fileno())
        if sys.platform != 'win32':
            target.chmod(0o755)
    print(f'Verified pinned restic/rclone helpers: {selected}')


if __name__ == '__main__':
    main()
