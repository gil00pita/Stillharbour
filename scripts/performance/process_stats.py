"""Bounded macOS observed-descendant sampler. XPC/GPU ownership is not inferred."""
import ctypes, json, signal, subprocess, sys, time
from pathlib import Path

class Usage(ctypes.Structure):
    _fields_ = [('uuid', ctypes.c_uint8 * 16)] + [(name, ctypes.c_uint64) for name in (
        'user', 'system', 'idle', 'interrupt', 'pageins', 'wired', 'resident', 'footprint',
        'started', 'exited', 'child_user', 'child_system', 'child_idle', 'child_interrupt',
        'child_pageins', 'child_elapsed', 'read', 'written')]

class Sampler:
    def __init__(self, pid):
        self.pid = pid
        self.lib = ctypes.CDLL('/usr/lib/libproc.dylib') if sys.platform == 'darwin' else None
        if self.lib:
            self.lib.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]
            self.lib.proc_pid_rusage.restype = ctypes.c_int
        self.started = time.monotonic()
        self.samples, self.base, self.last = [], {}, {}
        self.root_start, self.errors, self.dropped = None, 0, 0

    def sample(self):
        if not self.lib: return
        raw = subprocess.run(['/bin/ps', '-axo', 'pid=,ppid=,rss='], capture_output=True, text=True, timeout=5)
        rows = [tuple(map(int, line.split())) for line in raw.stdout.splitlines() if len(line.split()) == 3]
        selected = {self.pid}
        for _ in range(16):
            added = {pid for pid, parent, _ in rows if parent in selected}
            if added <= selected: break
            selected |= added
        if len(selected) > 64:
            self.errors += 1
            return
        rss = sum(kib * 1024 for pid, _, kib in rows if pid in selected)
        root_footprint = None
        for pid in selected:
            usage = Usage()
            if self.lib.proc_pid_rusage(pid, 2, ctypes.byref(usage)) != 0:
                self.errors += 1
                continue
            key = (pid, usage.started)
            if pid == self.pid:
                if self.root_start is None: self.root_start = usage.started
                if self.root_start != usage.started: raise RuntimeError('Owned process identity changed')
                root_footprint = usage.footprint
            if key not in self.base and len(self.base) >= 4096:
                self.dropped += 1
                continue
            if key not in self.base: self.base[key] = (usage.read, usage.written) if pid == self.pid else (0, 0)
            self.last[key] = (usage.read, usage.written)
        item = {'tMs': (time.monotonic() - self.started) * 1000, 'observedTreeRssBytes': rss,
                'descendants': len(selected) - 1, 'coreFootprintBytes': root_footprint,
                'observedDiskReadBytes': sum(max(0, self.last[k][0] - self.base[k][0]) for k in self.last),
                'observedDiskWriteBytes': sum(max(0, self.last[k][1] - self.base[k][1]) for k in self.last)}
        if len(self.samples) < 4096: self.samples.append(item)
        else: self.dropped += 1

    def result(self):
        if not self.lib: return {'status': 'blocked', 'reason': 'macOS libproc sampler unavailable', 'samples': []}
        return {'method': 'macOS ps RSS plus libproc rusage_info_v2; 500ms sampling',
                'coverage': 'core and observed descendants; unowned XPC/WebContent/GPU and short-lived children may be missing',
                'physicalIoAttribution': 'process-level only, not per physical endpoint', 'samples': self.samples,
                'errors': self.errors, 'dropped': self.dropped, 'peakObservedTreeRssBytes': max((s['observedTreeRssBytes'] for s in self.samples), default=None)}

if __name__ == '__main__':
    pid, duration = int(sys.argv[1]), min(1800, max(1, float(sys.argv[2])))
    sampler = Sampler(pid)
    deadline = time.monotonic() + duration
    stopped = [False]
    signal.signal(signal.SIGTERM, lambda *_: stopped.__setitem__(0, True))
    while not stopped[0] and time.monotonic() < deadline:
        sampler.sample()
        time.sleep(.5)
    print(json.dumps(sampler.result()))
