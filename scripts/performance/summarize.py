"""Summarize explicit phase-matched evidence; never turn missing observations into zeros."""
import argparse, json, math, uuid
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('--catalog-run', required=True)
parser.add_argument('--phase', choices=['P0','P1'], default='P0')
args = parser.parse_args()
run = str(uuid.UUID(args.catalog_run))
base = Path('.context/performance')
def stats(values):
    if not values: return None
    values = sorted(values)
    return {'n': len(values), **{name: values[min(len(values)-1, math.ceil(q*len(values))-1)] for name,q in [('p50',.5),('p95',.95),('p99',.99)]}, 'max': values[-1]}
def memory(value):
    samples = value.get('samples', [])
    return {'sampleCount': len(samples), 'peakObservedRssBytes': value.get('peakObservedTreeRssBytes'), 'firstObservedRssBytes': samples[0]['observedTreeRssBytes'] if samples else None, 'lastObservedRssBytes': samples[-1]['observedTreeRssBytes'] if samples else None, 'lastObservedDiskReadBytes': samples[-1]['observedDiskReadBytes'] if samples else None, 'lastObservedDiskWriteBytes': samples[-1]['observedDiskWriteBytes'] if samples else None, 'errors': value.get('errors'), 'dropped': value.get('dropped'), 'coverage': value.get('coverage')}
summary = {'catalogRun': run, 'phase':args.phase, 'catalog': {}, 'native': {}, 'failures': []}
for rows in [10000,100000,1000000]:
    path = base/f'{run}-{rows}'/'baseline.json'
    data = json.loads(path.read_text())
    if data.get('phase','P0') != args.phase: raise SystemExit('Catalog evidence belongs to another phase')
    assert data['sourceManifestEqual']
    trace = data['catalog']['trace']
    summary['catalog'][rows] = {'metrics': data['catalog']['metrics'], 'freezeOnlyMs': stats([event['elapsed_us']/1000 for event in trace['events'] if event['metric']=='selection_freeze']), 'memory': memory(data['memory']), 'payloadBytes': data['catalog'].get('pageJsonBytes',data['catalog'].get('wire',{}).get('packedBytes')), 'wire':data['catalog'].get('wire'), 'catalogBytes': data['catalog'].get('catalogBytesAfterSelection'), 'sourceEntries': data['sourceEntryCount'], 'media': data['media']['metrics'], 'traceEvents': len(trace['events']), 'traceDropped': trace['dropped']}
for path in sorted(base.glob('native-*/baseline.json'), key=lambda p:p.stat().st_mtime):
    data = json.loads(path.read_text())
    if data.get('phase','P0') != args.phase: continue
    if data.get('failure'):
        summary['failures'].append({'run':path.parent.name,'sourceManifestEqual':data.get('sourceManifestEqual')})
        continue
    if data.get('scrollRouteSteps') != 120: continue
    events=data['frontend']['events']
    rows=data['fixture']['rows']
    summary['native'][rows]={'run':path.parent.name,'scrollMs':data['scrollRouteMs'],'firstGeneratedImageMs':data['freshFixtureSaveScanToFirstDecodedImageMs'],'frontend':{metric:stats([event['value'] for event in events if event['metric']==metric]) for metric in ['layout_ms','react_render_ms','raf_interval_ms','ipc_ms','ipc_request_bytes','ipc_response_bytes','image_load_ms','image_load_to_raf_ms','mounted_items','resident_summaries']},'frontendDropped':data['frontend']['dropped'],'backendEvents':len(data['backend']['trace']['events']),'backendDropped':data['backend']['trace']['dropped'],'scanOverlapSamples':sum(s['runningAtQueryStart'] for s in data['scanOverlapSamples']),'memory':memory(data['memory']),'environment':data.get('environment'),'sourceManifestEqual':data['sourceManifestEqual'],'jumpCommandToTargetMs':stats(data['randomJumpCompletionsMs']),'resizeCommandMs':stats(data['resizeCommandMs'])}
output=base/f'{run}-summary.json'
output.write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({'summary':str(output),'nativeSizes':list(summary['native']),'retainedFailures':len(summary['failures'])}))
