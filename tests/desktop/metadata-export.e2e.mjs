import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';
import { native, reviewCatalogMetadata, finishMetadataExports } from './m5-metadata-helpers.mjs';

describe('M5 generated native metadata export', () => {
  it('reviews and undoes tags, exports independent catalog/XMP copies and preserves all sources', async function () {
    this.timeout(120000);
    const directory = '.context/m5-native-metadata';
    await mkdir(directory, { recursive: true });
    try {
      const selection = await browser.tauri.execute(({ core }) =>
        core.invoke('test_library_fixture', { verify: false, geographic: true }),
      );
      await native({
        kind: 'save_and_scan',
        selection_id: selection.id,
        label: 'Generated map library',
        mode: 'referenced',
        filter: {
          version: 1,
          includeGlobs: [],
          excludeGlobs: [],
          excludedFolders: [],
          extensions: ['jpg'],
          includeHiddenFiles: false,
          includeHiddenFolders: false,
          minBytes: null,
          maxBytes: null,
          fromDate: null,
          throughDate: null,
        },
      });
      await browser.waitUntil(
        async () => {
          const status = (await native({ kind: 'status' })).data;
          return (
            status.jobs.length > 0 &&
            status.jobs.every((job) => ['completed', 'partial'].includes(job.state))
          );
        },
        { timeout: 45000, interval: 200 },
      );
      // Establish a bounded map query using its photo-list alternative. This
      // metadata case deliberately asserts no GPU/animation-frame capability.
      await $('button[aria-label="Map"]').click();
      await $('button=Show photos in view').click();
      await browser.waitUntil(async () =>
        (await $('.map-photo-heading p').getText()).startsWith('6 photos'),
      );
      await browser.waitUntil(async () => $('button=Show in library').isEnabled());
      await $('button=Show in library').click();
      await browser.waitUntil(async () =>
        $('.asset-check[aria-label="Select South.jpg"]').isExisting(),
      );
      // Selection setup dispatches the real React input event. The occluded
      // gallery intentionally hides unchecked controls until pointer/focus.
      await browser.execute(() =>
        document.querySelector('.asset-check[aria-label="Select South.jpg"]').click(),
      );
      const metadata = await reviewCatalogMetadata(directory);
      const exports = await finishMetadataExports(directory);
      const audit = await browser.tauri.execute(({ core }) =>
        core.invoke('test_library_fixture', { verify: true }),
      );
      assert.equal(audit.verifiedFiles, 9);
      const foundation = await browser.tauri.execute(({ core }) =>
        core.invoke('foundation_request', { request: { kind: 'status' } }),
      );
      assert.equal(foundation.data.originalMutationsEnabled, false);
      await writeFile(
        `${directory}/metadata.json`,
        JSON.stringify(
          {
            platform: process.platform,
            metadata,
            exports,
            audit,
            originalMutationsEnabled: false,
            graphicsCapabilityAsserted: false,
            nativeFocusCapabilityAsserted: true,
          },
          null,
          2,
        ),
      );
    } catch (error) {
      await browser.saveScreenshot(`${directory}/failure.png`);
      await writeFile(
        `${directory}/failure.json`,
        JSON.stringify({ message: String(error), text: await $('main').getText() }, null, 2),
      );
      throw error;
    }
  });
});
