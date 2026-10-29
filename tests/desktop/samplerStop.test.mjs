import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { expect, it } from 'vitest';
import { samplerStop } from '../performance/samplerStop.mjs';

it('waits for inherited stdout to close after the sampler process has exited', async () => {
  const report = JSON.stringify({ samples: [1, 2, 3] });
  const lateWriter = `setTimeout(() => process.stdout.write(${JSON.stringify(report)}), 120)`;
  const parent = `
    const { spawn } = require('node:child_process');
    spawn(process.execPath, ['-e', ${JSON.stringify(lateWriter)}], {
      stdio: ['ignore', 1, 'ignore'],
    }).unref();
    process.exit(0);
  `;
  const child = spawn(process.execPath, ['-e', parent], { stdio: ['ignore', 'pipe', 'ignore'] });
  const stop = samplerStop(child);
  const closed = once(child, 'close');
  let output = '';
  child.stdout.setEncoding('utf8');
  child.stdout.on('data', (chunk) => (output += chunk));
  try {
    await once(child, 'exit');
    await stop();
    expect(output).toBe(report);
  } finally {
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
