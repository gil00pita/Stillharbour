#!/usr/bin/env python3
"""Prepare only the two reviewed DLLs for the Windows loopback test server.

Development only; never bundled or used by the application. No SDK, system
service, Windows account or operating-system share configuration is changed.
"""
import argparse
import hashlib
import io
from pathlib import Path
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
URL = 'https://github.com/TalAloni/SMBLibrary/releases/download/v1.5.7/SMBServer_1.5.7.zip'
ARCHIVE_HASH = 'f353c96bd499f01eb52b096ee81da9d9df8df042318461f13264fb63443732dc'
FILES = {
    'SMBLibrary.dll': '1d0dd528c3f7b04dabfde046d7afb131b747e4659a08e53b3ad9f615833d3d11',
    'SMBLibrary.Win32.dll': '935c2e508a66299e53be807fb7310d1d3a2f1682d7c0d8bcf2f8bdc92cbf3dbb',
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--download', action='store_true')
    args = parser.parse_args()
    archive = ROOT / '.context/m4-helpers/smb-library/SMBServer_1.5.7.zip'
    if not archive.exists():
        if not args.download:
            raise SystemExit('Missing pinned SMB fixture archive; use --download explicitly.')
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read(5 * 1024 * 1024 + 1)
        if len(data) > 5 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != ARCHIVE_HASH:
            raise SystemExit('SMB fixture archive failed its pin.')
        archive.parent.mkdir(parents=True, exist_ok=True)
        with archive.open('xb') as output:
            output.write(data)
    if archive.is_symlink() or archive.stat().st_size > 5 * 1024 * 1024:
        raise SystemExit('SMB fixture archive is linked or exceeds its size bound.')
    data = archive.read_bytes()
    if hashlib.sha256(data).hexdigest() != ARCHIVE_HASH:
        raise SystemExit('SMB fixture archive changed.')
    destination = ROOT / '.context/m4-smb-server'
    destination.mkdir(parents=True, exist_ok=True)
    if not destination.resolve().is_relative_to(ROOT / '.context'):
        raise SystemExit('SMB fixture output escaped .context.')
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        for name, expected in FILES.items():
            member = f'SMBServer/net40/{name}'
            if len([entry for entry in package.infolist() if entry.filename == member]) != 1:
                raise SystemExit('SMB fixture has missing/repeated members.')
            content = package.read(member)
            if hashlib.sha256(content).hexdigest() != expected:
                raise SystemExit('SMB fixture DLL changed.')
            path = destination / name
            if path.is_symlink():
                raise SystemExit('Refusing a linked SMB fixture DLL.')
            if path.exists():
                if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
                    raise SystemExit('Existing SMB fixture DLL differs; review before changing it.')
            else:
                with path.open('xb') as output:
                    output.write(content)
    print('Verified development-only SMBLibrary 1.5.7 loopback fixture (LGPL-3.0-or-later).')


if __name__ == '__main__':
    main()
