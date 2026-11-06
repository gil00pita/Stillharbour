import { copyFileSync, constants } from 'node:fs';
import { resolve } from 'node:path';
import { run } from './process.mjs';
run(process.execPath, [
  'scripts/tauri.mjs',
  'build',
  '--debug',
  '--no-bundle',
  '--features',
  'desktop-test',
  '--config',
  'src-tauri/tauri.test.conf.json',
]);
const suffix = process.platform === 'win32' ? '.exe' : '';
// A unique, exclusive copy cannot replace a still-running test executable.
const app = resolve(`target/debug/stillharbour-albums-test-${process.pid}${suffix}`);
copyFileSync(`target/debug/stillharbour${suffix}`, app, constants.COPYFILE_EXCL);
run(process.execPath, ['node_modules/@wdio/cli/bin/wdio.js', 'run', 'wdio.albums.conf.mjs'], {
  env: { STILLHARBOUR_ALBUMS_TEST_BINARY: app },
});
