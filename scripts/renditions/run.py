#!/usr/bin/env python3
"""Generated macOS large-image worker diagnostics; never accepts a source library."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import time
import uuid
import zlib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--worker', default='target/debug/stillharbour-media-worker')
args = parser.parse_args()
if platform.system() != 'Darwin':
    raise SystemExit('macOS ImageIO diagnostics only; no Windows evidence')
if shutil.disk_usage('.').free < 2 * 1024**3:
    raise SystemExit('Two GiB reserve required; no cleanup attempted')
workspace = Path(__file__).resolve().parents[2]
worker = (workspace / args.worker).resolve(strict=True)
context = workspace / '.context'
parent = context / 'renditions'
if context.is_symlink() or parent.is_symlink():
    raise SystemExit('Owned fixture namespace must not be a link')
run = str(uuid.uuid4())
owned = parent / run
owned.mkdir(parents=True, exist_ok=False)
sources = owned / 'sources'
sources.mkdir()
(owned / 'ownership.json').write_text(json.dumps({'run': run, 'kind': 'large-rendition-fixture', 'version': 1}) + '\n')

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)

# Asymmetric deterministic pattern, created without an external image service.
w, h = 96, 48
pixels = b''.join(b'\0' + b''.join(bytes((220, 30, 20) if x < w // 2 and y < h // 2 else (20, 180, 40) if y < h // 2 else (20, 40, 210) if x < w // 2 else (230, 190, 20)) for x in range(w)) for y in range(h))
seed = owned / 'seed.png'
seed.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(pixels)) + chunk(b'IEND', b''))
# Fail safely against an older app before using a new dispatch flag: the
# established stdin mode never enters GUI startup. Edge 32 plus the report
# schema identify this rendition worker generation.
with seed.open('rb') as stream:
    handshake = subprocess.run([str(worker), '--media-worker-stdin', '32', '0'], stdin=stream, capture_output=True, timeout=15)
try:
    assert 'input_read_bytes' in json.loads(handshake.stdout)['Ok']
except (ValueError, KeyError, AssertionError):
    raise SystemExit('Build the current rendition worker before this diagnostic; no new worker mode invoked')
for suffix, codec in [('jpg', 'jpeg'), ('png', 'png'), ('tif', 'tiff')]:
    subprocess.run(['/usr/bin/sips', '-z', '6000', '8000', '-s', 'format', codec, str(seed), '--out', str(sources / f'large.{suffix}')], check=True, stdout=subprocess.DEVNULL)
# Camera/export JPEG and PNG frequently carry an ICC profile. These exercise
# the old native fallback's full-source estimate, unlike untagged 48 MP RGB.
for suffix in ('jpg', 'png'):
    subprocess.run(['/usr/bin/sips', '--matchTo', '/System/Library/ColorSync/Profiles/Display P3.icc', str(sources / f'large.{suffix}'), '--out', str(sources / f'large-tagged.{suffix}')], check=True, stdout=subprocess.DEVNULL)
# Well-formed but outside the retained 12,000-pixel bound.
wide = sources / 'over-dimension.png'
wide.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 12001, 1, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(b'\0' + b'\xff\0\0' * 12001)) + chunk(b'IEND', b''))
(sources / 'malformed.png').write_bytes(b'not an image')

def manifest():
    result = {}
    for file in sorted(sources.iterdir()):
        assert file.is_file() and not file.is_symlink() and file.parent == sources
        digest = hashlib.sha256()
        with file.open('rb') as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b''):
                digest.update(block)
        stat = file.stat()
        result[file.name] = {'sha256': digest.hexdigest(), 'bytes': stat.st_size, 'mtimeNs': stat.st_mtime_ns, 'device': stat.st_dev, 'inode': stat.st_ino}
    return result

before = manifest()
(owned / 'manifest-before.json').write_text(json.dumps(before, indent=2) + '\n')
results = []
try:
    for file in sorted(sources.iterdir()):
        for edge in (32, 512):
            started = time.monotonic()
            with file.open('rb') as stream:
                proc = subprocess.run([str(worker), '--media-worker-file', str(edge), '0'], stdin=stream, capture_output=True, timeout=15)
            report = json.loads(proc.stdout)
            success = 'Ok' in report
            assert success == file.name.startswith('large'), (file.name, report)
            if success:
                value = report['Ok']
                assert (value['source_width'], value['source_height']) == (8000, 6000)
                assert (value['thumbnail_width'], value['thumbnail_height']) == (edge, edge * 3 // 4)
                png = bytes(value.pop('png'))
                assert png.startswith(b'\x89PNG\r\n\x1a\n') and len(png) <= 2 * 1024**2
                assert value['input_read_bytes'] <= 512 * 1024**2
                output = owned / f'{file.name}-{edge}.png'
                output.write_bytes(png)
                value['pngBytes'] = len(png)
            results.append({'fixture': file.name, 'edge': edge, 'wallSeconds': time.monotonic() - started, 'exit': proc.returncode, 'result': report})
    legacy = []
    for name in ('large-tagged.jpg', 'large-tagged.png', 'large.tif'):
        with (sources / name).open('rb') as stream:
            proc = subprocess.run([str(worker), '--media-worker-stdin', '512', '0'], stdin=stream, capture_output=True, timeout=15)
        report = json.loads(proc.stdout)
        assert report['Err']['code'] == 'UNSUPPORTED', (name, report)
        legacy.append({'fixture': name, 'result': report})
    (owned / 'legacy-budget-results.json').write_text(json.dumps(legacy, indent=2) + '\n')
    # A writable descriptor must be refused even for a disposable source.
    with (sources / 'large.jpg').open('r+b') as stream:
        denied = subprocess.run([str(worker), '--media-worker-file', '32', '0'], stdin=stream, capture_output=True, timeout=15)
    assert json.loads(denied.stdout)['Err']['code'] == 'UNSUPPORTED'
finally:
    after = manifest()
    (owned / 'manifest-after.json').write_text(json.dumps(after, indent=2) + '\n')
    (owned / 'worker-results.json').write_text(json.dumps(results, indent=2) + '\n')
    assert before == after, 'Source identity/content changed; retain evidence, do not clean up'
print(json.dumps({'run': run, 'cases': len(results), 'sourcesUnchanged': True, 'evidence': str(owned.relative_to(workspace))}))
