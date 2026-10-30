"""Summarize explicitly selected P4 native diagnostics, without upgrading their build scope."""
import argparse
import json
import math
from pathlib import Path
import re
import uuid


def stats(values):
    if not values:
        return None
    ordered = sorted(values)
    return {'n': len(ordered), **{
        f'p{p}': ordered[math.ceil(p * len(ordered) / 100) - 1] for p in (50, 95, 99)
    }, 'max': ordered[-1]}


def read_json(path):
    if path.is_symlink() or path.stat().st_size > 4 * 1024 * 1024:
        raise ValueError('Bounded regular evidence file required')
    return json.loads(path.read_text())


def summarize(folder):
    data = read_json(folder / 'baseline.json')
    if data.get('phase') != 'P4':
        raise ValueError('Only explicit P4 evidence is accepted; historical phases stay separate')
    failures = []
    if data.get('failure'):
        failures.append('native_route_failed')
    before = read_json(folder / 'manifest-before.json')
    after = read_json(folder / 'manifest-after.json')
    if data.get('sourceManifestEqual') is not True or before != after:
        failures.append('source_manifest_mismatch')
    # Lossless transport is required for the new stage, including failed runs.
    if not before['entries'] or any(
        not isinstance(entry[field], str)
        for entry in before['entries'] for field in ('size', 'mtimeNs', 'inode', 'device')
    ):
        failures.append('source_manifest_not_lossless')
    files = data.get('segments', [])
    if not 1 <= len(files) <= 32 or files != [f'segment-{n}.json' for n in range(len(files))]:
        raise ValueError('Expected one bounded sequential segment list')
    frontend, backend, scroll = [], [], []
    frontend_drops = backend_drops = 0
    for file in files:
        segment = read_json(folder / file)
        for trace in (segment['frontend'], segment['backend']['trace']):
            if trace['capacity'] != 4096 or len(trace['events']) > 4096:
                raise ValueError('Diagnostic segment exceeded its fixed bound')
        frontend.extend(segment['frontend']['events'])
        backend.extend(segment['backend']['trace']['events'])
        frontend_drops += segment['frontend']['dropped']
        backend_drops += segment['backend']['trace']['dropped']
        if segment['label'] == 'scroll':
            scroll.append(segment)
    if frontend_drops or backend_drops:
        failures.append('trace_loss')
    if data.get('scrollRouteSteps') != 120 or data.get('scrollRouteMs', 0) < 60000 or len(scroll) != 12:
        failures.append('incomplete_60_second_route')
    residents = [e['value'] for e in frontend if e['metric'] == 'resident_summaries']
    mounts = [e['value'] for e in frontend if e['metric'] == 'mounted_items']
    if not residents or max(residents) > 720:
        failures.append('resident_summary_bound')
    if not mounts:
        failures.append('mounted_observations_missing')
    if data.get('textNonemptyThumbnailRequests') != 0:
        failures.append('text_image_requests')
    if len(data.get('randomJumpCompletionsMs', [])) != 10:
        failures.append('date_jump_route_incomplete')
    if len(data.get('resizeCommandMs', [])) != 4:
        failures.append('resize_route_incomplete')
    if data.get('build', {}).get('profile') != 'debug desktop-test; production release automation remains forbidden':
        failures.append('build_scope_missing')
    if any(not re.fullmatch(r'[0-9a-f]{64}', data.get('build', {}).get(field, ''))
           for field in ('executableSha256', 'cargoLockSha256', 'npmLockSha256')):
        failures.append('build_identity_missing')
    memory = data.get('memory', {})
    samples = memory.get('samples', [])
    return {
        'run': folder.name, 'rows': data.get('fixture', {}).get('rows'),
        'status': 'failed' if failures else 'passed_scoped_native_diagnostics',
        'failures': failures, 'build': data.get('build'), 'environment': data.get('environment'),
        'scrollRouteMs': data.get('scrollRouteMs'), 'segments': len(files),
        'frontendEvents': len(frontend), 'backendEvents': len(backend),
        'frontendDropped': frontend_drops, 'backendDropped': backend_drops,
        'metrics': {metric: stats([e['value'] for e in frontend if e['metric'] == metric])
                    for metric in ('ipc_ms', 'layout_ms', 'react_render_ms', 'raf_interval_ms',
                                   'image_load_ms', 'image_load_to_raf_ms')},
        'dateJumpCommandMs': stats(data.get('randomJumpCompletionsMs', [])),
        'resizeCommandMs': stats(data.get('resizeCommandMs', [])),
        'maxResidentSummaries': max(residents, default=None),
        'maxMountedItems': max(mounts, default=None),
        'textNonemptyThumbnailRequests': data.get('textNonemptyThumbnailRequests'),
        'observedMemory': {
            'samples': len(samples), 'errors': memory.get('errors'), 'dropped': memory.get('dropped'),
            'peakRssBytes': memory.get('peakObservedTreeRssBytes'),
            'firstRssBytes': samples[0]['observedTreeRssBytes'] if samples else None,
            'lastRssBytes': samples[-1]['observedTreeRssBytes'] if samples else None,
            'coverage': memory.get('coverage'),
        },
        'sourceManifestEqual': before == after,
        'releasePresentation': 'not run; instrumented debug UI and rAF cannot pass release presentation',
        'wholeApplicationMemory': 'not established; WebContent/GPU/XPC ownership is incomplete',
        'backupNasHddWindows': 'blocked/unrun; local generated observations do not satisfy these gates',
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-run', action='append', required=True)
    args = parser.parse_args()
    if not 1 <= len(args.native_run) <= 3 or len(set(args.native_run)) != len(args.native_run):
        parser.error('Select one to three distinct native evidence runs explicitly')
    reports = []
    base = Path('.context/performance')
    if Path('.context').is_symlink() or base.is_symlink():
        raise ValueError('Evidence namespace cannot be a link')
    for name in args.native_run:
        match = re.fullmatch(r'native-([0-9a-f-]{36})-(10000|100000|1000000)', name)
        if not match or str(uuid.UUID(match[1])) != match[1] or (base / name).is_symlink():
            parser.error('Expected native-UUID-10000|100000|1000000')
        report = summarize(base / name)
        if report['rows'] != int(match[2]):
            raise ValueError('Evidence rows disagree with selected run name')
        reports.append(report)
    output = Path('.context/p4')
    if output.is_symlink():
        raise ValueError('Report namespace cannot be a link')
    output.mkdir(exist_ok=True)
    path = output / f'report-{uuid.uuid4()}.json'
    with path.open('x') as stream:
        json.dump({'phase': 'P4', 'nativeDiagnostics': reports}, stream, indent=2)
    print(json.dumps({'report': str(path), 'statuses': [r['status'] for r in reports]}))
    if any(report['failures'] for report in reports):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
