import { config as foundation } from './wdio.conf.mjs';
import { resolve } from 'node:path';
const app = resolve(process.env.STILLHARBOUR_PERF_APP ?? 'target/debug/stillharbour');
export const config = {
  ...foundation,
  services: [['@wdio/tauri-service', { ...foundation.services[0][1], appBinaryPath: app }]],
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: app } }],
  specs: ['./tests/performance/gallery.e2e.mjs'],
  mochaOpts: { timeout: 900000 },
  outputDir: './.context/wdio-performance',
};
