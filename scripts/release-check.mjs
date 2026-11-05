import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { run } from './process.mjs';
import {
  assertProductionFrontend,
  assertProductionGraph,
  regularFiles,
} from './release-policy.mjs';

const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const mapManifest = JSON.parse(readFileSync('assets/maps/manifest.json', 'utf8'));
const placeManifest = JSON.parse(readFileSync('assets/places/manifest.json', 'utf8'));
for (const [path, hash] of Object.entries(placeManifest.files))
  assert.equal(
    createHash('sha256').update(readFileSync(path)).digest('hex'),
    hash,
    `Bundled place data checksum: ${path}`,
  );
for (const [path, hash] of Object.entries(mapManifest.files))
  assert.equal(
    createHash('sha256').update(readFileSync(path)).digest('hex'),
    hash,
    `Bundled map data checksum: ${path}`,
  );
const capability = JSON.parse(readFileSync('src-tauri/capabilities/foundation.json', 'utf8'));
assert.deepEqual(config.app.security.capabilities, ['foundation']);
assert.deepEqual(capability.permissions, ['allow-foundation-request', 'allow-library-request']);
assert.equal(config.app.withGlobalTauri, undefined);
assert.equal(capability.remote, undefined);
assert.deepEqual(config.bundle.resources, { '../vendor/exiftool/': 'helpers/exiftool/' });
const backupManifest = JSON.parse(readFileSync('vendor/backup-helper-manifest.json', 'utf8'));
for (const [platform, configName] of [
  ['windows-x86_64', 'windows'],
  ['macos-aarch64', 'macos'],
]) {
  const platformConfig = JSON.parse(
    readFileSync(`src-tauri/tauri.${configName}.conf.json`, 'utf8'),
  );
  const artifacts = backupManifest.artifacts.filter((artifact) => artifact.platform === platform);
  assert.equal(artifacts.length, 2);
  assert.deepEqual(artifacts.map((artifact) => artifact.helper).sort(), ['rclone', 'restic']);
  assert.deepEqual(platformConfig, {
    bundle: {
      resources: {
        '../vendor/exiftool/': 'helpers/exiftool/',
        ...Object.fromEntries(
          artifacts.map((artifact) => [
            `../vendor/backup-helpers/${platform}/${artifact.executable}`,
            `helpers/backup/${artifact.executable}`,
          ]),
        ),
        '../vendor/backup-helper-manifest.json': 'helpers/backup/manifest.json',
        '../vendor/backup-licenses/': 'helpers/backup/licenses/',
      },
    },
  });
}
run(process.execPath, ['scripts/backup-helpers.mjs', '--check']);
assert.equal(config.bundle.externalBin, undefined);
assert.ok(!config.app.security.csp.includes('unsafe-eval'));
assert.ok(!config.app.security.csp.includes('https:'));
const source = readFileSync('src-tauri/src/main.rs', 'utf8');
assert.ok(source.includes('#[cfg(all(feature = "desktop-test", not(debug_assertions)))]'));
// Confirm the default release dependency graph has no test server or fixture spike.
const graph = run(
  'cargo',
  [
    'tree',
    '--locked',
    '-p',
    'stillharbour',
    '-e',
    'normal',
    '--prefix',
    'none',
    '--format',
    '{p} features=[{f}]',
  ],
  {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
  },
);
assert.equal(graph.status, 0);
assertProductionGraph(graph.stdout);
console.log(
  'Production configuration and default dependency graph exclude desktop-test instrumentation.',
);

const manifest = JSON.parse(readFileSync('vendor/exiftool-manifest.json', 'utf8'));
const helperFiles = regularFiles('vendor/exiftool');
assert.equal(helperFiles.length, Object.keys(manifest.files).length);
for (const [path, hash] of Object.entries(manifest.files))
  assert.equal(
    createHash('sha256')
      .update(readFileSync(join('vendor/exiftool', path)))
      .digest('hex'),
    hash,
    `ExifTool resource checksum: ${path}`,
  );
assertProductionFrontend('dist');
assert.ok(!capability.permissions.includes('allow-test-generated-rename-worker'));
assert.ok(!capability.permissions.includes('allow-test-generated-rename'));
assert.ok(!capability.permissions.includes('allow-test-performance'));
console.log(
  'Vendored ExifTool resources match the reviewed file manifest; production frontend excludes test routes.',
);
console.log(
  'Preflight only: signing, installed-package contents and release acceptance remain separate gates.',
);
