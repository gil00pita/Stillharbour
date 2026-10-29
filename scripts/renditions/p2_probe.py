#!/usr/bin/env python3
"""Owned P2 worker/rendition measurements; never accepts a photo-library path."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import random
import shutil
import struct
import subprocess
import threading
import time
import uuid
import zlib


def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)


def png(width, height, pixels):
    stride = width * 3
    raw = b''.join(b'\0' + pixels[y * stride:(y + 1) * stride] for y in range(height))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker', default='target/debug/stillharbour-media-worker')
    parser.add_argument('--legacy-worker')
    parser.add_argument('--samples', type=int, default=8, choices=range(3, 21))
    parser.add_argument('--build', choices=['debug', 'release'], required=True)
    args = parser.parse_args()
    if platform.system() != 'Darwin':
        raise SystemExit('Native ImageIO measurements require macOS; no Windows claim.')
    workspace = Path(__file__).resolve().parents[2]
    if shutil.disk_usage(workspace).free < 4 * 1024**3:
        raise SystemExit('Four GiB reserve required; no cleanup attempted.')
    parent = workspace / '.context' / 'p2-renditions'
    if (workspace / '.context').is_symlink() or parent.is_symlink():
        raise SystemExit('Owned fixture namespace cannot be a link.')
    run = str(uuid.uuid4())
    owned = parent / run
    sources = owned / 'sources'
    sources.mkdir(parents=True, exist_ok=False)
    (owned / 'ownership.json').write_text(json.dumps({'run': run, 'kind': 'P2-rendition-probe', 'version': 1}))
    worker = (workspace / args.worker).resolve(strict=True)
    legacy = (workspace / args.legacy_worker).resolve(strict=True) if args.legacy_worker else None
    # Incompressible RGB exercises actual PNG/transport budgets, not a tiny solid image.
    rng = random.Random(20261029)
    width, height = 3072, 2048
    (sources / 'noise.png').write_bytes(png(width, height, rng.randbytes(width * height * 3)))
    for suffix, codec in [('jpg', 'jpeg'), ('tif', 'tiff')]:
        subprocess.run(['/usr/bin/sips', '-s', 'format', codec, str(sources / 'noise.png'), '--out', str(sources / f'noise.{suffix}')], check=True, stdout=subprocess.DEVNULL)
    (sources / 'small.png').write_bytes(png(12, 8, bytes([220, 40, 20]) * 12 * 8))
    (sources / 'malformed.png').write_bytes(b'Not an image')
    (sources / 'over-dimension.png').write_bytes(png(12001, 1, bytes([20, 40, 220]) * 12001))

    def manifest():
        return json.loads(subprocess.check_output(['python3', str(workspace / 'scripts/performance/manifest.py'), str(sources)]))

    before = manifest()
    (owned / 'manifest-before.json').write_text(json.dumps(before))
    observations = []
    cases = []

    def invoke(binary, source, edge, version=2, writable=False):
        stop = threading.Event()
        peaks = []
        with source.open('r+b' if writable else 'rb') as stream:
            started = time.perf_counter()
            process = subprocess.Popen([str(binary), '--media-worker-file-v2' if version == 2 else '--media-worker-file', str(edge), '0'], stdin=stream, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            def sample():
                while not stop.is_set():
                    value = subprocess.run(['/bin/ps', '-o', 'rss=', '-p', str(process.pid)], capture_output=True, text=True, timeout=2)
                    if value.returncode == 0 and value.stdout.strip().isdigit():
                        peaks.append(int(value.stdout.strip()) * 1024)
                    stop.wait(0.05)
            sampler = threading.Thread(target=sample)
            sampler.start()
            try:
                output, error = process.communicate(timeout=15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.communicate()
                raise
            finally:
                stop.set()
                sampler.join()
            elapsed = (time.perf_counter() - started) * 1000
        assert process.returncode == 0, f'Worker failed with exit{process.returncode}; no app startup attempted by diagnostic'
        if version == 2:
            assert output[:8] == b'SHRP\0\0\0\x02' and len(output) >= 12
            length = struct.unpack('<I', output[8:12])[0]
            assert 0 < length <= 4096 and 12 + length <= len(output)
            result = json.loads(output[12:12 + length])
            body = output[12 + length:]
            if 'Ok' in result:
                assert len(body) == result['Ok']['png_bytes']
                result['Ok']['png'] = body
            else:
                assert not body
        else:
            result = json.loads(output)
            if 'Ok' in result:
                result['Ok']['png'] = bytes(result['Ok']['png'])
        return result, {'durationMs': elapsed, 'wireBytes': len(output), 'sampledChildPeakRssBytes': max(peaks, default=None), 'rssSamples': len(peaks)}

    try:
        # First use the established byte-input mode, as the existing diagnostic
        # does. Its32px/report schema identifies the fail-closed worker era;
        # never try a newer app mode after an unsuccessful handshake.
        for binary in [worker] + ([legacy] if legacy else []):
            with (sources / 'small.png').open('rb') as stream:
                check = subprocess.run([str(binary), '--media-worker-stdin', '32', '0'], stdin=stream, capture_output=True, timeout=15)
            try:
                known = json.loads(check.stdout)['Ok']
                assert check.returncode == 0 and known['thumbnail_width'] == 12 and 'input_read_bytes' in known
            except (ValueError, KeyError, AssertionError):
                raise SystemExit('Build the current rendition worker first; no newer worker mode invoked')
        # Check known worker framing before any measurements; the app rejects unknown
        # worker modes before GUI/catalog startup, and this runner owns all inputs.
        handshake, _ = invoke(worker, sources / 'small.png', 32)
        assert handshake['Ok']['thumbnail_width'] == 12
        for suffix in ['jpg', 'png', 'tif']:
            source = sources / f'noise.{suffix}'
            reference = None
            if legacy:
                result, measurement = invoke(legacy, source, 512, version=1)
                reference = hashlib.sha256(result['Ok']['png']).hexdigest()
                observations.append({'format': suffix, 'edge': 512, 'protocol': 1, 'phase': 'legacy-worker-cold-process', **measurement})
            for edge in [32, 256, 512, 1024, 2048]:
                for sample in range(args.samples):
                    result, measurement = invoke(worker, source, edge)
                    report = result['Ok']
                    body = report['png']
                    assert body.startswith(b'\x89PNG\r\n\x1a\n')
                    actual = struct.unpack('>II', body[16:24])
                    assert actual == (report['thumbnail_width'], report['thumbnail_height'])
                    assert max(actual) == edge and abs(actual[0] / actual[1] - width / height) < 0.1
                    if reference and edge == 512:
                        assert hashlib.sha256(body).hexdigest() == reference, 'Existing512px output changed'
                    observations.append({'format': suffix, 'edge': edge, 'protocol': 2, 'sample': sample, 'phase': 'worker-cold-process-os-cache-uncontrolled', 'pngBytes': len(body), **measurement})
                cases.append({'format': suffix, 'edge': edge, 'status': 'passed'})
        for name in ['malformed.png', 'over-dimension.png']:
            result, _ = invoke(worker, sources / name, 2048)
            assert 'Err' in result
            cases.append({'case': name, 'status': 'passed', 'errorCode': result['Err']['code']})
        result, _ = invoke(worker, sources / 'small.png', 2048)
        assert result['Ok']['thumbnail_width'] == 12 and result['Ok']['thumbnail_height'] == 8
        cases.append({'case': 'no-upscale', 'status': 'passed'})
        result, _ = invoke(worker, sources / 'small.png', 2048, writable=True)
        assert 'Err' in result
        cases.append({'case': 'read-only-descriptor-required', 'status': 'passed'})
    finally:
        after = manifest()
        (owned / 'manifest-after.json').write_text(json.dumps(after))
        (owned / 'observations.json').write_text(json.dumps(observations, indent=2))
        assert before == after, 'Source manifest changed'
    def percentiles(values):
        import math
        ordered = sorted(values)
        return {f'p{p}': ordered[max(0, math.ceil(len(ordered) * p / 100) - 1)] for p in [50, 95, 99]}
    summaries = []
    for suffix in ['jpg', 'png', 'tif']:
        for edge in [32, 256, 512, 1024, 2048]:
            samples = [row for row in observations if row['protocol'] == 2 and row['format'] == suffix and row['edge'] == edge]
            summaries.append({'format': suffix, 'edge': edge, 'n': len(samples), 'durationMs': percentiles([row['durationMs'] for row in samples]), 'wireBytes': samples[0]['wireBytes'], 'childPeakRssBytes': max((row['sampledChildPeakRssBytes'] or 0 for row in samples), default=0)})
    summary = {'run': run, 'build': args.build, 'platform': platform.platform(), 'workerSha256': hashlib.sha256(worker.read_bytes()).hexdigest(), 'legacySha256': hashlib.sha256(legacy.read_bytes()).hexdigest() if legacy else None, 'cases': cases, 'summaries': summaries, 'manifestUnchanged': True, 'scope': 'Generated worker protocol/decode; each invocation starts a fresh process. No OS cache purge, disk-cache hit test, UI paint, queue or complete process-tree memory claim.'}
    (owned / 'summary.json').write_text(json.dumps(summary, indent=2))
    print(json.dumps({'run': run, 'cases': len(cases), 'samples': len(observations), 'manifestUnchanged': True, 'summary': str((owned / 'summary.json').relative_to(workspace))}))


if __name__ == '__main__':
    main()
