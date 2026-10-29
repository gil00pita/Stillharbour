import { run } from './process.mjs';
import { resolve } from 'node:path';
run(process.execPath, [resolve('node_modules/@tauri-apps/cli/tauri.js'), ...process.argv.slice(2)]);
