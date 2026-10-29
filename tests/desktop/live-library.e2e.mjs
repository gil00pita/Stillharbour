import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';
import { mkdir, writeFile, rename, access } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
function manifest(root) {
  return JSON.parse(
    execFileSync('python3', ['scripts/performance/manifest.py', root, '--lossless-stat'], {
      encoding: 'utf8',
      maxBuffer: 2 * 1024 * 1024,
    }),
  );
}
async function native(request) {
  return browser.tauri.execute(
    async ({ core }, request) => core.invoke('library_request', { request }),
    request,
  );
}
async function monitor(action) {
  return browser.tauri.execute(
    async ({ core }, action) => core.invoke('test_performance', { action, rows: null }),
    action,
  );
}
async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}
const filter = {
  version: 1,
  includeGlobs: [],
  excludeGlobs: [],
  excludedFolders: ['Private'],
  extensions: [],
  includeHiddenFiles: false,
  includeHiddenFolders: false,
  minBytes: null,
  maxBytes: null,
  fromDate: null,
  throughDate: null,
};
(process.platform === 'darwin' ? describe : describe.skip)(
  'live read-only library — owned external changes',
  () => {
    it('updates the real Gallery and Folders and exposes retained change history', async () => {
      assert.equal(process.platform, 'darwin');
      await browser.setWindowSize(1180, 780);
      await $('button.primary').waitForEnabled();
      let selection;
      try {
        selection = await browser.tauri.execute(async ({ core }) =>
          core.invoke('test_library_fixture', { verify: false }),
        );
      } catch (error) {
        assert.match(String(error), /A generated fixture already exists/);
        await browser.tauri.execute(async ({ core }) =>
          core.invoke('test_library_fixture', { verify: true }),
        );
        const existing = await monitor('fixture_location');
        assert.equal(typeof existing.selectionId, 'string');
        selection = { id: existing.selectionId };
      }
      assert.equal(
        (await native({ kind: 'status' })).data.roots.length,
        0,
        'Use a fresh catalog or the prior suite’s explicitly forgotten root',
      );
      const { fixtureRoot } = await monitor('fixture_location');
      assert.ok(basename(fixtureRoot).startsWith('stillharbour-generated-'));
      const run = randomUUID();
      const output = resolve('.context/live-library', `native-${run}`);
      await mkdir(output, { recursive: false });
      await writeFile(
        join(output, 'ownership.json'),
        JSON.stringify({ run, kind: 'live-library-native' }),
      );
      const before = manifest(fixtureRoot);
      await writeFile(join(output, 'manifest-before.json'), JSON.stringify(before));
      const original = join(fixtureRoot, 'Coast/Generated-00.jpg');
      const moved = join(fixtureRoot, 'Woodland/Live-moved.jpg');
      const held = join(output, 'held-source.jpg');
      try {
        await native({
          kind: 'save_and_scan',
          selection_id: selection.id,
          label: 'Live source',
          mode: 'referenced',
          filter,
        });
        await $('input[aria-label="Search filenames"]').waitForDisplayed();
        await $('input[aria-label="Search filenames"]').setValue('Generated-00.jpg');
        await $('button[aria-label="Open Generated-00.jpg"]').waitForDisplayed();
        await monitor('monitor');
        await browser.waitUntil(
          async () => {
            const status = (await native({ kind: 'status' })).data;
            return (
              status.monitoring?.some((entry) => entry.mode === 'watching') &&
              !status.jobs.some((job) =>
                ['queued', 'running', 'cancel_requested'].includes(job.state),
              )
            );
          },
          { timeout: 45000 },
        );
        await rename(original, moved); // Simulated external actor; generated fixture only.
        const renamed = manifest(fixtureRoot);
        await writeFile(join(output, 'renamed-before.json'), JSON.stringify(renamed));
        await browser.waitUntil(
          async () => !(await $('button[aria-label="Open Generated-00.jpg"]').isExisting()),
          { timeout: 45000 },
        );
        await $('input[aria-label="Search filenames"]').setValue('Live-moved.jpg');
        await $('button[aria-label="Open Live-moved.jpg"]').waitForDisplayed({ timeout: 45000 });
        assert.deepEqual(manifest(fixtureRoot), renamed);
        await writeFile(join(output, 'renamed-after.json'), JSON.stringify(manifest(fixtureRoot)));
        await $('button[aria-label="Folders"]').click();
        await $('button[aria-label="Open folder Live source"]').click();
        await $('button[aria-label="Open folder Woodland"]').waitForDisplayed();
        await $('button[aria-label="Open folder Woodland"]').click();
        await browser.waitUntil(async () =>
          (await $('.explorer-navigation [aria-current="location"]').getText()).includes(
            'Woodland',
          ),
        );
        await browser.execute(() => {
          const viewport = document.querySelector('.explorer-viewport');
          viewport.scrollTop = viewport.scrollHeight;
        });
        await $('button[aria-label="Open Live-moved.jpg"]').waitForDisplayed();
        await rename(moved, held); // Retain the exact generated object outside the indexed root.
        const absent = manifest(fixtureRoot);
        await writeFile(join(output, 'absent-before.json'), JSON.stringify(absent));
        await browser.waitUntil(
          async () => !(await $('button[aria-label="Open Live-moved.jpg"]').isExisting()),
          { timeout: 45000 },
        );
        assert.deepEqual(manifest(fixtureRoot), absent);
        await writeFile(join(output, 'absent-after.json'), JSON.stringify(manifest(fixtureRoot)));
        await $('button[aria-label="Jobs"]').click();
        await browser.execute(() => {
          const details = document.querySelector('.library-changes');
          details.open = true;
          details.dispatchEvent(new Event('toggle'));
        });
        await browser.waitUntil(async () => {
          const text = await $('.library-change-list').getText();
          return (
            text.includes('Moved') && text.includes('Removed') && text.includes('Live-moved.jpg')
          );
        });
        await browser.execute(() =>
          document.querySelector('.library-change-list').scrollIntoView({ block: 'nearest' }),
        );
        await browser.saveScreenshot('.context/screenshots/live-library-changes-desktop.png');
        await browser.setWindowSize(660, 560);
        assert.equal(
          await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
          true,
        );
        await browser.execute(() =>
          document.querySelector('.library-change-list').scrollIntoView({ block: 'nearest' }),
        );
        await browser.saveScreenshot('.context/screenshots/live-library-changes-minimum.png');
      } finally {
        await monitor('monitor_stop');
        if (await exists(held)) await rename(held, original);
        else if (await exists(moved)) await rename(moved, original);
        const after = manifest(fixtureRoot);
        await writeFile(join(output, 'manifest-after.json'), JSON.stringify(after));
        const restored = (inventory) => ({
          ...inventory,
          entries: inventory.entries.map((entry) => {
            if ((entry.mode & 0o170000) !== 0o040000) return entry;
            return Object.fromEntries(Object.entries(entry).filter(([key]) => key !== 'mtimeNs'));
          }),
        });
        assert.deepEqual(
          restored(after),
          restored(before),
          'External staging restores every file/path/identity; only parent-directory mtimes may differ',
        );
        const changedDirectories = after.entries
          .filter((entry, index) => entry.mtimeNs !== before.entries[index].mtimeNs)
          .map((entry) => ({ id: entry.id, mtimeNs: entry.mtimeNs }));
        assert.ok(
          changedDirectories.every((change) =>
            after.entries.some(
              (entry) => entry.id === change.id && (entry.mode & 0o170000) === 0o040000,
            ),
          ),
        );
        await writeFile(
          join(output, 'restored-source-check.json'),
          JSON.stringify({
            filesAndIdentitiesRestored: true,
            externalDirectoryMtimes: changedDirectories,
          }),
        );
      }
    });
  },
);
