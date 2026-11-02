import { run } from './process.mjs';
import { resolve } from 'node:path';
if (process.argv.slice(2).some((argument) => ['build', 'dev'].includes(argument))) {
  run(process.execPath, [resolve('scripts/backup-helpers.mjs'), '--check']);
}
run(process.execPath, [resolve('node_modules/@tauri-apps/cli/tauri.js'), ...process.argv.slice(2)]);
