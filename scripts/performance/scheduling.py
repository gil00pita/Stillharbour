"""Own disposable synthetic sources and retain independent before/after evidence."""
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import uuid

from manifest import manifest

REPO = Path(__file__).resolve().parents[2]


def main():
    os.chdir(REPO)
    namespace = REPO / '.context' / 'scheduling'
    for path in [REPO / '.context', namespace]:
        if path.is_symlink():
            raise ValueError('Fixture namespace cannot be a link')
        path.mkdir(exist_ok=True)
    run = str(uuid.uuid4())
    owned = namespace / run
    owned.mkdir()
    sources = owned / 'sources'
    sources.mkdir()
    (owned / 'ownership.json').write_text(json.dumps({
        'run': run, 'kind': 'p3-synthetic-read-contention',
    }))
    for index in range(4):
        with (sources / f'{index}.bin').open('xb') as stream:
            stream.write(bytes([index + 1]) * (256 * 1024))
    binary = REPO / 'target' / 'release' / ('examples/scheduler_probe.exe' if os.name == 'nt' else 'examples/scheduler_probe')
    before = manifest(sources, lossless_stat=True)
    (owned / 'manifest-before.json').write_text(json.dumps(before, indent=2))
    # No shell, arbitrary paths, cleanup or actual backup helper is involved.
    try:
        result = subprocess.run([str(binary), '--run', run], capture_output=True, text=True, timeout=30)
        (owned / 'stdout.json').write_text(result.stdout)
        (owned / 'stderr.txt').write_text(result.stderr)
    finally:
        after = manifest(sources, lossless_stat=True)
        (owned / 'manifest-after.json').write_text(json.dumps(after, indent=2))
    report = {
        'run': run, 'exitCode': result.returncode, 'manifestEqual': before == after,
        'platform': platform.platform(), 'machine': platform.machine(),
        'mode': 'release synthetic read contention; 2 ms simulated helper step',
        'cache': 'OS cache uncontrolled; no warm/cold media claim',
        'binarySha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'lockSha256': hashlib.sha256((REPO / 'Cargo.lock').read_bytes()).hexdigest(),
    }
    if result.returncode == 0:
        trace = json.loads(result.stdout)
        report['traceDropped'] = trace['trace']['dropped']
        report['classes'] = []
        for row in trace['results']:
            summary = {'class': row['class'], 'logicalReadBytes': row['logicalReadBytes']}
            for field in ['waitUs', 'stepUs']:
                values = sorted(row[field])
                summary[field] = {'n': len(values)}
                for p in [50, 95, 99]:
                    summary[field][f'p{p}'] = values[max(0, (p * len(values) + 99) // 100 - 1)]
            report['classes'].append(summary)
    (owned / 'report.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    if result.returncode != 0 or before != after:
        sys.exit(1)


if __name__ == '__main__':
    main()
