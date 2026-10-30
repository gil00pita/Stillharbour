"""Run a new owned phase-labelled release catalog experiment. Never accepts a library path."""
import argparse, hashlib, json, platform, plistlib, shutil, subprocess, threading, time, uuid
from pathlib import Path
from manifest import manifest
from process_stats import Sampler

parser = argparse.ArgumentParser()
parser.add_argument('--rows', type=int, choices=[10000, 100000, 1000000], nargs='+', default=[10000, 100000, 1000000])
parser.add_argument('--phase', choices=['p0','p1','p4'], default='p0')
args = parser.parse_args()
if platform.system() != 'Darwin': raise SystemExit('Native adapter unavailable here; no macOS/Windows result inferred')
if shutil.disk_usage('.').free < 4 * 1024**3: raise SystemExit('At least 4 GiB free required; no cleanup attempted')
run = str(uuid.uuid4())
base = Path('.context/performance')
base.mkdir(parents=True, exist_ok=True)
environment = {'platform': platform.platform(), 'machine': platform.machine(), 'python': platform.python_version(), 'run': run, 'rows': args.rows, 'phase':args.phase}
for label, command in [('macos', ['sw_vers']), ('hardware', ['/usr/sbin/sysctl', '-n', 'machdep.cpu.brand_string', 'hw.memsize', 'hw.ncpu', 'hw.model']), ('rust', [str(Path.home()/'.cargo/bin/rustc'), '-Vv']), ('node', ['node', '--version']), ('npm', ['npm', '--version'])]:
    environment[label] = subprocess.check_output(command, text=True).strip()
environment['webkitFramework'] = {key: value for key, value in plistlib.loads(Path('/System/Library/Frameworks/WebKit.framework/Resources/Info.plist').read_bytes()).items() if key in ['CFBundleVersion', 'CFBundleShortVersionString']}
environment['sha256'] = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in map(Path, ['Cargo.lock', 'package-lock.json', 'target/release/examples/performance_probe', 'target/release/stillharbour'])}
(base/f'{run}-environment.json').write_text(json.dumps(environment, indent=2)+'\n')
for rows in args.rows:
    if shutil.disk_usage('.').free < 4 * 1024**3: raise SystemExit('Space reserve reached; fixtures retained, no cleanup')
    if base.is_symlink() or Path('.context').is_symlink(): raise SystemExit('Diagnostic namespace must not be a link')
    child = subprocess.Popen(['target/release/examples/performance_probe', '--rows', str(rows), '--run', run] + ([f'--{args.phase}'] if args.phase in ('p1', 'p4') else []), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    timed_out = threading.Event()
    def expire():
        timed_out.set()
        child.terminate()
    watchdog = threading.Timer(1800, expire)
    watchdog.daemon = True
    watchdog.start()
    sampler = Sampler(child.pid)
    stopped = threading.Event()
    def sample():
        while not stopped.is_set():
            sampler.sample()
            stopped.wait(.5)
    monitor = threading.Thread(target=sample, daemon=True); monitor.start()
    owned = base/f'{run}-{rows}'
    ready = child.stdout.readline(8 * 1024 * 1024)
    if not ready:
        stopped.set(); monitor.join(); raise SystemExit('Probe failed before handshake; inspect local build/run evidence')
    ready = json.loads(ready)
    assert ready['ready'] and ready['rows'] == rows
    marker = json.loads((owned/'ownership.json').read_text())
    assert marker['run'] == run and marker['rows'] == rows
    before = manifest(owned/'sources', lossless_stat=True)
    (owned/'manifest-before.json').write_text(json.dumps(before, indent=2)+'\n')
    measured_start_ms = (time.monotonic()-sampler.started)*1000
    child.stdin.write('measure\n'); child.stdin.flush(); child.stdin.close()
    raw = child.stdout.readline(8 * 1024 * 1024)
    code = child.wait(timeout=1800)
    stopped.set(); monitor.join(); watchdog.cancel()
    after = manifest(owned/'sources', lossless_stat=True)
    (owned/'manifest-after.json').write_text(json.dumps(after, indent=2)+'\n')
    equal = before == after
    if code or not raw or timed_out.is_set():
        (owned/'failure.json').write_text(json.dumps({'exit': code, 'sourceManifestEqual': equal, 'timedOut': timed_out.is_set()})+'\n')
        raise SystemExit('Probe failed; fixtures/manifests retained without raw error/path logging')
    result = json.loads(raw)
    result.update({'memory': sampler.result(), 'measurementStartMs': measured_start_ms, 'sourceManifestEqual': equal,
                   'sourceEntryCount': len(before['entries']), 'sourceManifestEncoding': 'lossless decimal 64-bit stat fields', 'environment': f'{run}-environment.json', 'cacheProtocol': 'OS cache uncontrolled/warm after seeding; P0 has seven app-connection reopens; P1 separately resets derived summaries and once the disposable connection pager; source and thumbnail cache not purged',
                   'network': 'not instrumented at OS socket level; no network workload requested'})
    trace_failure = False
    if args.phase == 'p4':
        traces = [result['catalog']['trace'], *result['catalog']['traceSegments']]
        trace_failure = any(trace['dropped'] or len(trace['events']) > 4096 for trace in traces)
        if trace_failure:
            result['failure'] = 'P4 catalog trace loss; preserve this failed evidence'
    (owned/'baseline.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({'run':run,'rows':rows,'completed':equal and not trace_failure,'sourceManifestEqual':equal}), flush=True)
    if not equal: raise SystemExit('Source manifest mismatch; stop')
    if trace_failure: raise SystemExit(result['failure'])
