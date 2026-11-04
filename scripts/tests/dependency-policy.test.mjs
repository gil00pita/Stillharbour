import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { resolve } from 'import-meta-resolve';
import { glob } from 'glob';
import JSZip from 'jszip';

// Resolve through the actual pinned Tauri service's utility dependency. A
// patched top-level browser helper must not hide an older nested helper.
const service = import.meta.resolve('@wdio/tauri-service');
const utilities = resolve('@wdio/utils/node', service);
const browserModule = resolve('@puppeteer/browsers', utilities);
const browser = await import(browserModule);
const { unpackArchive, extractZipWithYauzl } = await import(
  new URL('./fileUtil.js', browserModule)
);

function fixture(t) {
  const directory = fs.mkdtempSync(join(tmpdir(), 'stillharbour-dependency-policy-'));
  t.after(() => fs.rmSync(directory, { recursive: true }));
  return directory;
}

test('the browser helper JavaScript extractor rejects escaping ZIP symlinks and retains neighbors', async (t) => {
  const directory = fixture(t);
  const output = join(directory, 'output');
  const outside = join(directory, 'outside');
  fs.mkdirSync(output);
  fs.mkdirSync(outside);
  fs.writeFileSync(join(outside, 'neighbor.txt'), 'retain this exact version');
  // unpackArchive tries native tar/PowerShell/unzip first. Their platform-specific
  // behavior is not the JavaScript fallback's containment guarantee.
  assert.equal(typeof extractZipWithYauzl, 'function');
  for (const target of ['../outside', 'nested/../../outside', outside]) {
    const zip = new JSZip();
    zip.file('escape', target, { unixPermissions: 0o120777 });
    const archive = join(directory, 'hostile.zip');
    fs.writeFileSync(archive, await zip.generateAsync({ type: 'nodebuffer', platform: 'UNIX' }));
    await assert.rejects(
      extractZipWithYauzl(archive, output),
      (error) =>
        error instanceof Error &&
        error.cause instanceof Error &&
        /Zip symlink .*outside of the target directory/.test(error.cause.message),
      'Require the containment rejection, not an unavailable extractor or filesystem error',
    );
    assert.equal(fs.existsSync(join(output, 'escape')), false);
  }
  assert.deepEqual(fs.readdirSync(outside), ['neighbor.txt']);
  assert.equal(fs.readFileSync(join(outside, 'neighbor.txt'), 'utf8'), 'retain this exact version');
  assert.equal(fs.existsSync(join(output, 'escape')), false);
});

test('default and JavaScript browser extractors preserve generated regular files and neighbors', async (t) => {
  const directory = fixture(t);
  const neighbor = join(directory, 'neighbor.txt');
  fs.writeFileSync(neighbor, 'retain this version');
  const zip = new JSZip();
  zip.file('nested/photo-α.txt', 'generated archive content', { unixPermissions: 0o100644 });
  const archive = join(directory, 'regular.zip');
  fs.writeFileSync(archive, await zip.generateAsync({ type: 'nodebuffer', platform: 'UNIX' }));
  for (const [name, extract] of [
    ['default', unpackArchive],
    ['javascript', extractZipWithYauzl],
  ]) {
    const output = join(directory, name);
    fs.mkdirSync(output);
    await extract(archive, output);
    assert.equal(fs.lstatSync(join(output, 'nested', 'photo-α.txt')).isFile(), true);
    assert.equal(
      fs.readFileSync(join(output, 'nested', 'photo-α.txt'), 'utf8'),
      'generated archive content',
    );
    assert.equal(fs.readFileSync(neighbor, 'utf8'), 'retain this version');
  }
});

test('the upgraded browser helper preserves every export used by the pinned WebdriverIO utility', async () => {
  for (const name of [
    'install',
    'canDownload',
    'resolveBuildId',
    'detectBrowserPlatform',
    'computeExecutablePath',
  ])
    assert.equal(typeof browser[name], 'function', name);
  assert.equal(browser.Browser.CHROMEDRIVER, 'chromedriver');
  assert.equal(browser.ChromeReleaseChannel.STABLE, 'stable');
  // Import the consumer as well: ESM/CJS or removed-export breakage must fail
  // without downloading a browser, starting a server or opening a library.
  await import(utilities);
});

test('aborting glob during the first directory read stops descendant traversal', async (t) => {
  const directory = fixture(t);
  for (let index = 0; index < 4; index += 1) {
    fs.mkdirSync(join(directory, `child-${index}`, 'nested'), { recursive: true });
    fs.writeFileSync(join(directory, `child-${index}`, 'nested', 'photo.txt'), 'generated');
  }
  const controller = new AbortController();
  let reads = 0;
  const watchedFs = {
    readdir(path, options, callback) {
      reads += 1;
      fs.readdir(path, options, (error, entries) => {
        if (reads === 1) controller.abort(new Error('generated cancellation'));
        callback(error, entries);
      });
    },
  };
  await assert.rejects(
    glob('**/*', { cwd: directory, fs: watchedFs, signal: controller.signal }),
    /generated cancellation/,
  );
  await new Promise((done) => setTimeout(done, 30));
  assert.equal(reads, 1, 'Cancellation must prevent subsequent directory reads');
});

test('patched glob/minimatch retain native-tooling pattern and CommonJS compatibility', async (t) => {
  const directory = fixture(t);
  fs.mkdirSync(join(directory, 'nested'));
  fs.writeFileSync(join(directory, 'nested', 'photo-α.txt'), 'generated');
  fs.writeFileSync(join(directory, 'nested', 'skip.txt'), 'generated');
  const require = createRequire(service);
  const archiveUtilities = require('archiver-utils');
  const matches = archiveUtilities.file.expand({ cwd: directory, nodir: true }, [
    '**/*.txt',
    '!**/skip.txt',
  ]);
  assert.deepEqual(matches, [join('nested', 'photo-α.txt')]);
  const { Minimatch } = require('minimatch');
  assert.equal(new Minimatch('**/photo-{α,β}.txt').match('nested/photo-α.txt'), true);
  assert.equal(new Minimatch('**/photo-{α,β}.txt').match('nested/skip.txt'), false);
  assert.deepEqual(
    await glob('**/*.txt', { cwd: directory, ignore: '**/skip.txt', nodir: true }),
    matches,
  );
});
