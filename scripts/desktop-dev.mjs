import { run } from './process.mjs';
import { resolve } from 'node:path';
const port = Number(process.env.CONDUCTOR_PORT ?? 1420);
if (!Number.isInteger(port) || port < 1024 || port > 65535) throw new Error('Invalid dev port');
run(
  process.execPath,
  [
    resolve('scripts/tauri.mjs'),
    'dev',
    '--config',
    JSON.stringify({ build: { devUrl: `http://127.0.0.1:${port}` } }),
  ],
  {
    env: { STILLHARBOUR_PORT: String(port) },
  },
);
