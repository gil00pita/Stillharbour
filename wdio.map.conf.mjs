import { resolve } from 'node:path';
import { config as foundation } from './wdio.conf.mjs';
const app = resolve(
  `target/debug/stillharbour-map-test${process.platform === 'win32' ? '.exe' : ''}`,
);
export const config = {
  ...foundation,
  specs: ['./tests/desktop/map.e2e.mjs'],
  services: [['@wdio/tauri-service', { ...foundation.services[0][1], appBinaryPath: app }]],
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: app } }],
  outputDir: './.context/wdio-map',
};
