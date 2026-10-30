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
  installDiagnostics();
  expect(diagnosticClock()).toBeUndefined();
  harness()!.start();
  diagnosticPayload('ipc_request_bytes', {
    path: '/private/photo.jpg',
    gps: '51.1,0.2',
    credential: 'secret',
  });
  diagnosticValue('layout_ms', Number.NaN);
  diagnosticValue('layout_ms', -1);
  diagnosticValue('/private/photo.jpg' as never, 4);
  for (let index = 0; index < 5000; index++) diagnosticValue('layout_ms', 1);
  const trace = harness()!.read();
  expect(trace.events).toHaveLength(trace.capacity);
  expect(trace.dropped).toBeGreaterThan(0);
  const encoded = JSON.stringify(trace);
  expect(encoded).not.toContain('/private');
  expect(encoded).not.toContain('secret');
  expect(encoded).not.toContain('51.1');
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
