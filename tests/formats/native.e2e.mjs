import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { browser, $, $$ } from '@wdio/globals';

function manifest(root) {
  return JSON.parse(
    execFileSync('python3', ['scripts/performance/manifest.py', root], {
      encoding: 'utf8',
      maxBuffer: 2 * 1024 * 1024,
    }),
  );
}
describe('native common-format previews — generated sources only', () => {
  it('shows a generated HEIC through the real gallery/viewer without changing sources', async () => {
    assert.equal(process.platform, 'darwin');
    await $('button.primary').waitForEnabled();
    const selection = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: false, nativeFormats: true }),
    );
    const { fixtureRoot } = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_performance', { action: 'fixture_location', rows: null }),
    );
    const output = `.context/formats/native-${randomUUID()}`;
    await mkdir(output, { recursive: false });
    const before = manifest(fixtureRoot);
    await writeFile(`${output}/manifest-before.json`, JSON.stringify(before));
    try {
      const picker = await browser.tauri.mock('foundation_request');
      try {
        await picker.mockReturnValue({ kind: 'directory', data: selection });
        await $('button.primary').click();
        await $('.root-setup input[required]').setValue('Generated native formats');
        await $('.folder-actions .primary').click();
      } finally {
        await picker.mockRestore();
      }
      await $('.asset-viewport').waitForDisplayed();
      await $('input[aria-label="Search filenames"]').setValue('Native HEIC');
      await browser.waitUntil(
        async () =>
          browser.execute(
            () =>
              document.querySelectorAll('.asset-open').length === 1 &&
              document
                .querySelector('.asset-open')
                ?.getAttribute('aria-label')
                ?.includes('Native HEIC') &&
              [...document.querySelectorAll('.asset img')].some(
                (image) => image.complete && image.naturalWidth > 0,
              ),
          ),
        { timeout: 45000 },
      );
      assert.equal((await $$('.asset-open')).length, 1);
      await browser.saveScreenshot('.context/screenshots/native-heic-gallery.png');
      await $('.asset-open').click();
      await $('.photo-viewer[open]').waitForDisplayed();
      await browser.waitUntil(
        async () =>
          browser.execute(() => {
            const image = document.querySelector('.photo-stage img');
            return (
              image?.complete &&
              image.naturalWidth > 0 &&
              image.naturalWidth <= 512 &&
              image.naturalHeight <= 512
            );
          }),
        { timeout: 45000 },
      );
      await browser.saveScreenshot('.context/screenshots/native-heic-viewer.png');
      await browser.keys('Escape');
      await $('.asset-viewport').waitForDisplayed();
      assert.equal(await $('input[aria-label="Search filenames"]').getValue(), 'Native HEIC');
      const verified = await browser.tauri.execute(async ({ core }) =>
        core.invoke('test_library_fixture', { verify: true }),
      );
      assert.equal(verified.verifiedFiles, 36);
      const foundation = await browser.tauri.execute(async ({ core }) =>
        core.invoke('foundation_request', { request: { kind: 'status' } }),
      );
      assert.equal(foundation.data.originalMutationsEnabled, false);
      await writeFile(
        `${output}/result.json`,
        JSON.stringify({
          platform: 'macOS WKWebView debug',
          format: 'generated HEIC',
          gallery: true,
          viewer: true,
          sourceEntries: verified.verifiedFiles,
          originalMutationsEnabled: foundation.data.originalMutationsEnabled,
        }),
      );
    } finally {
      const after = manifest(fixtureRoot);
      await writeFile(`${output}/manifest-after.json`, JSON.stringify(after));
      assert.deepEqual(after, before);
    }
  });
});
