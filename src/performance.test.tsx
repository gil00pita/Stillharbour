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
  read: () => { events: { metric: string; value: number }[]; dropped: number; capacity: number };
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
