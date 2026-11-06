import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { assertMapNetworkPolicy } from '../map-network-policy.mjs';
import {
  assertProductionFrontend,
  assertProductionGraph,
  regularFiles,
} from '../release-policy.mjs';

function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), 'stillharbour-release-policy-'));
  t.after(() => rmSync(directory, { recursive: true }));
  writeFileSync(join(directory, 'index.html'), '<script type="module" src="/app.js"></script>');
  writeFileSync(join(directory, 'app.js'), 'console.log("Stillharbour")');
  return directory;
}

test('reject missing, empty and source-map-bearing frontend output', (t) => {
  const directory = fixture(t);
  assert.doesNotThrow(() => assertProductionFrontend(directory));
  rmSync(join(directory, 'app.js'));
  assert.throws(() => assertProductionFrontend(directory), /No production JavaScript/);
  writeFileSync(join(directory, 'app.js'), '  ');
  assert.throws(() => assertProductionFrontend(directory), /Empty production JavaScript/);
  writeFileSync(join(directory, 'app.js'), 'export {}');
  writeFileSync(join(directory, 'app.js.map'), '{}');
  assert.throws(() => assertProductionFrontend(directory), /Source map/);
  rmSync(join(directory, 'app.js.map'));
  writeFileSync(join(directory, 'index.html'), '');
  assert.throws(() => assertProductionFrontend(directory), /Empty frontend entry point/);
  rmSync(join(directory, 'index.html'));
  assert.throws(() => assertProductionFrontend(directory), /ENOENT/);
});

test('inspect every nested JavaScript chunk for test authority', (t) => {
  const directory = fixture(t);
  mkdirSync(join(directory, 'assets'));
  writeFileSync(join(directory, 'assets', 'lazy.mjs'), 'window.__stillharbourPerf = {}');
  assert.throws(() => assertProductionFrontend(directory), /Test instrumentation in assets/);
});

test('reject linked build roots, files and directory escapes', (t) => {
  const directory = fixture(t);
  const linkedRoot = join(directory, 'linked-root');
  symlinkSync(directory, linkedRoot, process.platform === 'win32' ? 'junction' : 'dir');
  assert.throws(() => regularFiles(linkedRoot), /regular directory/);
  assert.throws(() => assertProductionFrontend(directory), /Linked release input/);
  rmSync(linkedRoot);
  const linkedFile = join(directory, 'linked-file.js');
  // Windows file symlinks require Developer Mode or privilege. A junction above
  // exercises link refusal there; ordinary-file links are also tested on macOS.
  if (process.platform !== 'win32') {
    symlinkSync(join(directory, 'app.js'), linkedFile);
    assert.throws(() => assertProductionFrontend(directory), /Linked release input/);
  }
});

const graph = (shell = 'default', core = '') =>
  `stillharbour v0.1.0 (/workspace/src-tauri) features=[${shell}]\nstillharbour-core v0.1.0 (/workspace/core) features=[${core}]\n`;

test('production graph refuses enabled fixture/copy/backup features in either owned package', () => {
  assert.doesNotThrow(() => assertProductionGraph(graph()));
  for (const feature of ['fixture-probes', 'desktop-test', 'copy-workflow', 'backup-workflow']) {
    assert.throws(
      () => assertProductionGraph(graph(`default,${feature}`)),
      /Unreviewed production features/,
    );
    assert.throws(
      () => assertProductionGraph(graph('default', feature)),
      /Unreviewed production features/,
    );
  }
});

test('graph validation fails closed on missing inventories, duplicate enabled entries and test servers', () => {
  assert.throws(() => assertProductionGraph(''), /Missing production package/);
  assert.throws(
    () => assertProductionGraph('stillharbour v0.1.0\n'),
    /Missing resolved feature inventory/,
  );
  assert.throws(
    () => assertProductionGraph(graph() + 'stillharbour-core v0.1.0 features=[fixture-probes] (*)'),
    /Unreviewed production features/,
  );
  assert.throws(() => assertProductionGraph(graph() + 'tauri-plugin-wdio v1.5.1'), /Test server/);
});

test('map CSP permits only the compiled NASA imagery path, retaining all other restrictions', () => {
  const {
    app: { security },
  } = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
  assert.doesNotThrow(() => assertMapNetworkPolicy(security));
  for (const directive of ['img-src', 'connect-src', 'script-src', 'default-src']) {
    assert.throws(() =>
      assertMapNetworkPolicy({
        ...security,
        csp: security.csp.replace(directive, `${directive} https:`),
      }),
    );
  }
  assert.throws(() =>
    assertMapNetworkPolicy({
      ...security,
      csp: security.csp.replace('gibs.earthdata.nasa.gov', 'example.org'),
    }),
  );
  assert.throws(() =>
    assertMapNetworkPolicy({
      ...security,
      devCsp: security.devCsp.replace('gibs.earthdata.nasa.gov', 'example.org'),
    }),
  );
});
