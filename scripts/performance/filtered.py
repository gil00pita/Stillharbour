"""Measure only previously generated catalog fixtures, retaining independent source evidence."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import uuid

from manifest import manifest
from p4_report import stats


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture-run', required=True)
    parser.add_argument('--rows', type=int, choices=(10000, 100000, 1000000), default=1000000)
    args = parser.parse_args()
    fixture_run = str(uuid.UUID(args.fixture_run))
    repo = Path(__file__).resolve().parents[2]
    fixture = repo / '.context/performance' / f'{fixture_run}-{args.rows}'
    output_parent = repo / '.context/filtered'
    for directory in (repo / '.context', repo / '.context/performance', fixture, fixture / 'sources'):
        if directory.is_symlink() or not directory.is_dir():
            raise ValueError('Ordinary generated fixture directories required')
    marker = fixture / 'ownership.json'
    if marker.is_symlink() or not marker.is_file() or marker.stat().st_size > 4096:
        raise ValueError('Bounded ordinary ownership marker required')
    owner = json.loads(marker.read_text())
    if owner != {'version': 1, 'run': fixture_run, 'rows': args.rows, 'synthetic': True}:
        raise ValueError('Generated fixture ownership mismatch')
    if output_parent.is_symlink():
        raise ValueError('Output namespace cannot be a link')
    output_parent.mkdir(exist_ok=True)
    run = str(uuid.uuid4())
    output = output_parent / run
    output.mkdir()
    (output / 'ownership.json').write_text(json.dumps({'run': run, 'fixtureRun': fixture_run,
                                                     'rows': args.rows, 'kind': 'filtered-catalog'}))
    executable = repo / 'target/release/examples/filtered_probe'
    before = manifest(fixture / 'sources', lossless_stat=True)
    (output / 'manifest-before.json').write_text(json.dumps(before, indent=2))
    try:
        result = subprocess.run([str(executable), '--run', fixture_run, '--rows', str(args.rows)],
                                cwd=repo, capture_output=True, text=True, timeout=120)
        (output / 'stdout.json').write_text(result.stdout)
        (output / 'stderr.txt').write_text(result.stderr)
    finally:
        after = manifest(fixture / 'sources', lossless_stat=True)
        (output / 'manifest-after.json').write_text(json.dumps(after, indent=2))
    report = {'run': run, 'fixtureRun': fixture_run, 'rows': args.rows,
              'exitCode': result.returncode, 'sourceManifestEqual': before == after,
              'host': platform.platform(), 'machine': platform.machine(),
              'sha256': {str(path.relative_to(repo)): hashlib.sha256(path.read_bytes()).hexdigest()
                         for path in (executable, repo / 'Cargo.lock', repo / 'package-lock.json')},
              'cache': 'OS caches uncontrolled; derived summaries reset before each sample; no source scan or cache purge',
              'scope': 'release catalog-only calls; not native UI/physical I/O/full-memory evidence'}
    if result.returncode == 0:
        data = json.loads(result.stdout)
        if data['profile'] != 'release' or data['phase'] != 'P4-filtered' or data['rows'] != args.rows:
            raise ValueError('Unexpected probe build/scope')
        report['sqlite'] = data['results']['sqlite']
        report['scenarios'] = []
        for scenario in data['results']['scenarios']:
            report['scenarios'].append({
                'scope': scenario['scope'], 'matched': scenario['matched'],
                'bucketPlan': scenario['bucketPlan'],
                'bucketPlanScope': 'Unmodified fallback/cold timeline query',
                'pageFirstBucketPlan': scenario.get('pageFirstBucketPlan'),
                'pagePlan': scenario.get('pagePlan'),
                **{field: stats([sample[field] for sample in scenario['samples']])
                   for field in ('pageMs', 'bucketsMs', 'warmBucketsMs')},
                'timelineFirst': {field: stats([sample[field] for sample in scenario['timelineFirst']])
                                  for field in ('pageMs', 'bucketsMs')},
                'candidateProbe': {
                    'plan': scenario['candidateProbe']['plan'],
                    'retained': scenario['candidateProbe']['retained'],
                    'latencyMs': stats(scenario['candidateProbe']['samplesMs']),
                },
            })
    (output / 'report.json').write_text(json.dumps(report, indent=2))
    print(json.dumps({'run': run, 'report': str(output.relative_to(repo) / 'report.json'),
                      'exitCode': result.returncode, 'sourceManifestEqual': before == after}))
    if result.returncode != 0 or before != after:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
