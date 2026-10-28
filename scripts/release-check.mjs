import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { run } from './process.mjs';

const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const capability = JSON.parse(readFileSync('src-tauri/capabilities/foundation.json', 'utf8'));
assert.deepEqual(config.app.security.capabilities, ['foundation']);
assert.deepEqual(capability.permissions, ['allow-foundation-request', 'allow-library-request']);
assert.equal(config.app.withGlobalTauri, undefined);
assert.equal(capability.remote, undefined);
assert.deepEqual(config.bundle.resources, { '../vendor/exiftool/': 'helpers/exiftool/' });
assert.equal(config.bundle.externalBin, undefined);
assert.ok(!config.app.security.csp.includes('unsafe-eval'));
assert.ok(!config.app.security.csp.includes('https:'));
const source = readFileSync('src-tauri/src/main.rs', 'utf8');
assert.ok(source.includes('#[cfg(all(feature = "desktop-test", not(debug_assertions)))]'));
// Confirm the default release dependency graph has no test server or fixture spike.
const graph = run('cargo', ['tree', '-p', 'stillharbour', '-e', 'normal', '--prefix', 'none'], {
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'inherit'],
});
assert.equal(graph.status, 0);
assert.ok(!/tauri-plugin-wdio/.test(graph.stdout));
console.log(
  'Production configuration and default dependency graph exclude desktop-test instrumentation.',
);

const manifest = JSON.parse(readFileSync('vendor/exiftool-manifest.json', 'utf8'));
function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? files(join(directory, entry.name)) : [join(directory, entry.name)],
  );
}
const helperFiles = files('vendor/exiftool');
assert.equal(helperFiles.length, Object.keys(manifest.files).length);
for (const [path, hash] of Object.entries(manifest.files))
  assert.equal(
    createHash('sha256')
      .update(readFileSync(join('vendor/exiftool', path)))
      .digest('hex'),
    hash,
    `ExifTool resource checksum: ${path}`,
  );
const frontend = files('dist')
  .filter((path) => path.endsWith('.js'))
  .map((path) => readFileSync(path, 'utf8'))
  .join('\n');
assert.ok(!frontend.includes('__wdio_mocks__'));
assert.ok(!frontend.includes('stillharbour:test-library-request'));
assert.ok(!frontend.includes('__stillharbourPerf'));
assert.ok(!capability.permissions.includes('allow-test-performance'));
console.log(
  'Vendored ExifTool resources match the reviewed file manifest; production frontend excludes test routes.',
);
