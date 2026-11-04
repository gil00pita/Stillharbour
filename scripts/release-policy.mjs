import assert from 'node:assert/strict';
import { lstatSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';

// Build inputs only: the caller must keep the build tree quiescent. This is
// not authorization to traverse or mutate a photo library.
export function regularFiles(directory) {
  assert.ok(lstatSync(directory).isDirectory(), `Expected a regular directory: ${directory}`);
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    const stat = lstatSync(path);
    assert.ok(!stat.isSymbolicLink(), `Linked release input: ${path}`);
    if (stat.isDirectory()) return regularFiles(path);
    assert.ok(stat.isFile(), `Nonregular release input: ${path}`);
    return [path];
  });
}

export function assertProductionGraph(graph) {
  assert.ok(!/tauri-plugin-wdio/.test(graph), 'Test server in production dependency graph');
  for (const name of ['stillharbour', 'stillharbour-core']) {
    const rows = graph.split('\n').filter((line) => line.startsWith(`${name} v`));
    assert.ok(rows.length > 0, `Missing production package: ${name}`);
    for (const row of rows) {
      const match = / features=\[([^\]]*)\](?: \(\*\))?$/.exec(row);
      assert.ok(match, `Missing resolved feature inventory: ${name}`);
      const features = match[1].split(',').filter(Boolean);
      assert.ok(
        features.every((feature) => feature === 'default'),
        `Unreviewed production features for ${name}: ${features.join(',')}`,
      );
    }
  }
}

export function assertProductionFrontend(directory) {
  const paths = regularFiles(directory);
  assert.ok(
    readFileSync(join(directory, 'index.html'), 'utf8').trim(),
    'Empty frontend entry point',
  );
  const scripts = paths.filter((path) => /\.(?:js|mjs|cjs)$/.test(path));
  assert.ok(scripts.length > 0, 'No production JavaScript found');
  for (const path of paths) {
    assert.ok(
      !path.endsWith('.map'),
      `Source map in production frontend: ${relative(directory, path)}`,
    );
  }
  const markers = [
    '__wdio_mocks__',
    'stillharbour:test-library-request',
    '__stillharbourPerf',
    'stillharbour:rename-lab',
    'test_generated_rename',
    'test_generated_rename_worker',
  ];
  for (const path of scripts) {
    const content = readFileSync(path, 'utf8');
    assert.ok(content.trim(), `Empty production JavaScript: ${relative(directory, path)}`);
    for (const marker of markers)
      assert.ok(
        !content.includes(marker),
        `Test instrumentation in ${relative(directory, path)}: ${marker}`,
      );
  }
}
