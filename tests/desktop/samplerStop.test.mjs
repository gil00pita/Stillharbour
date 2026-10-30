import { spawn } from 'node:child_process';
import { EventEmitter, once } from 'node:events';
import { setImmediate as nextTurn } from 'node:timers/promises';
import { expect, it, vi } from 'vitest';
import { samplerStop } from '../performance/samplerStop.mjs';

it('waits for stdout to close after an already exited sampler on every platform', async () => {
  // Model the documented exit/data/close ordering without inheriting a Unix fd
  // into a grandchild: Node's Windows stdio pipes cannot assume that behavior.
  const child = Object.assign(new EventEmitter(), {
    exitCode: null,
    signalCode: null,
    kill: vi.fn(),
    stdout: new EventEmitter(),
  });
  const stop = samplerStop(child);
  child.exitCode = 0;
  child.emit('exit', 0, null);
  const report = JSON.stringify({ samples: [1, 2, 3] });
  let output = '';
  child.stdout.on('data', (chunk) => (output += chunk));
  let stopped = false;
  const stopping = stop().then(() => (stopped = true));
  try {
    await nextTurn();
    expect(stopped).toBe(false);
    expect(child.kill).not.toHaveBeenCalled();
    child.stdout.emit('data', report);
    child.emit('close', 0, null);
    await stopping;
    expect(output).toBe(report);
    expect(stopped).toBe(true);
    await stop();
  } finally {
    child.emit('close', 0, null);
    await stopping;
  }
});

it('collects a real child report completely before teardown on every platform', async () => {
  const report = JSON.stringify({ samples: Array.from({ length: 10000 }, (_, index) => index) });
  const child = spawn(
    process.execPath,
    ['-e', 'process.stdout.end(JSON.stringify({samples:Array.from({length:10000},(_,i)=>i)}))'],
    { stdio: ['ignore', 'pipe', 'pipe'] },
  );
  const stop = samplerStop(child);
  const closed = once(child, 'close');
  let output = '';
  let errors = '';
  child.stdout.setEncoding('utf8');
  child.stdout.on('data', (chunk) => (output += chunk));
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => (errors += chunk));
  try {
    await once(child, 'exit');
    await stop();
    expect(child.exitCode, errors).toBe(0);
    expect(output).toBe(report);
    expect(errors).toBe('');
  } finally {
    await stop();
    await closed;
  }
});

it('terminates only its running sampler and supports repeated teardown', async () => {
  const child = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], {
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  const stop = samplerStop(child);
  await once(child, 'spawn');
  await stop();
  expect(child.exitCode !== null || child.signalCode !== null).toBe(true);
  await stop();
});
