import { resolve } from 'node:path';
import { config as foundation } from './wdio.conf.mjs';
const app = resolve(
  process.env.STILLHARBOUR_ALBUMS_TEST_BINARY ??
    `target/debug/stillharbour-albums-test${process.platform === 'win32' ? '.exe' : ''}`,
);
export const config = {
  ...foundation,
  specs: ['./tests/desktop/albums.e2e.mjs'],
  services: [['@wdio/tauri-service', { ...foundation.services[0][1], appBinaryPath: app }]],
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: app } }],
  outputDir: './.context/wdio-albums',
};
