// Isolate the instrumented executable from a concurrently running Tauri dev
// watcher, which can relink the default-feature binary at target/debug.
import { copyFileSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
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
const directory = '.context/m5-native/app';
mkdirSync(directory, { recursive: true });
// Keep Cargo's directory/resource layout while giving the copy its own name.
const output = `target/debug/stillharbour-map-test${suffix}`;
copyFileSync(`target/debug/stillharbour${suffix}`, output);
writeFileSync(
  `${directory}/build.json`,
  JSON.stringify(
    {
      platform: process.platform,
      sha256: createHash('sha256').update(readFileSync(output)).digest('hex'),
      instrumented: true,
    },
    null,
    2,
  ),
);
run(process.execPath, [
  'node_modules/@wdio/cli/bin/wdio.js',
  'run',
  process.argv.includes('--metadata') ? 'wdio.metadata.conf.mjs' : 'wdio.map.conf.mjs',
]);
