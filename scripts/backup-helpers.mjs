import { run } from './process.mjs';

// Build tooling only. Network retrieval requires an explicit --download; the
// desktop executable has neither a downloader nor a PATH helper fallback.
const args = process.argv.slice(2);
run(process.platform === 'win32' ? 'python' : 'python3', [
  'scripts/prepare-backup-helpers.py',
  ...args,
]);
