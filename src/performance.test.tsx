import { afterEach, expect, it, vi } from 'vitest';
import {
  diagnosticClock,
  diagnosticPayload,
  diagnosticValue,
  installDiagnostics,
} from './performance';
type Harness = {
  start: () => void;
  stop: () => void;
  read: () => {
    events: { metric: string; value: number; tMs: number }[];
    dropped: number;
    capacity: number;
  };
  drain: () => ReturnType<Harness['read']>;
};
const harness = () => (window as Window & { __stillharbourPerf?: Harness }).__stillharbourPerf;
afterEach(() => {
  harness()?.stop();
  delete (window as Window & { __stillharbourPerf?: Harness }).__stillharbourPerf;
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
});
it('stays off and exposes no harness in ordinary builds', () => {
  installDiagnostics();
  diagnosticValue('layout_ms', 1);
  expect(harness()).toBeUndefined();
  expect(diagnosticClock()).toBeUndefined();
});
it('requires explicit opt-in, drops at the cap and records numeric sizes without private contents', () => {
  vi.stubEnv('MODE', 'desktop-test');
  const clock = vi.spyOn(performance, 'now').mockReturnValue(1000);
  installDiagnostics();
  expect(diagnosticClock()).toBeUndefined();
  harness()!.start();
  // A valid elapsed time can contain a GPS substring. Keep that collision
  // deterministic and verify the permitted structure rather than serialized text.
  clock.mockReturnValue(1051.125);
  const payload = {
    path: '/private/photo.jpg',
    gps: '51.1,0.2',
    credential: 'secret',
  };
  diagnosticPayload('ipc_request_bytes', payload);
  diagnosticValue('layout_ms', Number.NaN);
  diagnosticValue('layout_ms', -1);
  diagnosticValue('/private/photo.jpg' as never, 4);
  for (let index = 0; index < 5000; index++) diagnosticValue('layout_ms', 1);
  const trace = harness()!.read();
  expect(trace.events).toHaveLength(trace.capacity);
  expect(trace.dropped).toBe(5001 - trace.capacity);
  expect(Object.keys(trace).sort()).toEqual(['capacity', 'dropped', 'events']);
  expect(new Set(trace.events.map((event) => Object.keys(event).sort().join(',')))).toEqual(
    new Set(['metric,tMs,value']),
  );
  expect(trace.events[0]).toEqual({
    metric: 'ipc_request_bytes',
    value: new TextEncoder().encode(JSON.stringify(payload)).byteLength,
    tMs: 51.125,
  });
  expect(new Set(trace.events.slice(1).map((event) => event.metric))).toEqual(
    new Set(['layout_ms']),
  );
  expect(new Set(trace.events.slice(1).map((event) => event.value))).toEqual(new Set([1]));
  expect(new Set(trace.events.map((event) => event.tMs))).toEqual(new Set([51.125]));
  harness()!.stop();
  expect(diagnosticClock()).toBeUndefined();
});

it('drains bounded segments while retaining active recording and loss counts', () => {
  vi.stubEnv('MODE', 'desktop-test');
  installDiagnostics();
  const recorder = harness()!;
  recorder.start();
  for (let index = 0; index < 4100; index++) diagnosticValue('ipc_ms', index);
  const first = recorder.drain();
  expect(first.events).toHaveLength(4096);
  expect(first.dropped).toBe(4);
  expect(recorder.read()).toMatchObject({ events: [], dropped: 0 });
  diagnosticValue('ipc_ms', 42);
  const second = recorder.drain();
  expect(second.events).toHaveLength(1);
  expect(second.events[0]!.value).toBe(42);
  expect(second.events[0]!.tMs).toBeGreaterThanOrEqual(first.events.at(-1)!.tMs);
  expect(first.events).toHaveLength(4096);
  recorder.stop();
  diagnosticValue('ipc_ms', 43);
  expect(recorder.drain().events).toHaveLength(0);
});
