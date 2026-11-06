import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';
import { native } from './m5-metadata-helpers.mjs';

async function showMapPhotos() {
  await $('button[aria-label="Map"]').click();
  if (await $('button=Show photos in view').isExisting())
    await $('button=Show photos in view').click();
  await $('button=Show in photos').waitForEnabled();
  assert.equal(await $('button=Show in library').isExisting(), false);
  await $('button=Show in photos').click();
  await $('button[aria-label="Clear Map area filter"]').waitForDisplayed();
}
async function expectOrdinaryPhotos() {
  await $('button[aria-label="Photos"]').click();
  await $('.photo-toolbar').waitForDisplayed();
  assert.equal(await $('button[aria-label="Clear Map area filter"]').isExisting(), false);
  assert.equal(await $('.map-filters').isExisting(), false);
  assert.equal(await $('.photo-toolbar button[aria-label="Filter"]').isExisting(), true);
}

describe('Map to Photos navigation — generated geographic fixture', () => {
  it('transfers only explicit map scope, labels it in search and filters, and clears it without changing text search', async function () {
    this.timeout(90000);
    await mkdir('.context/map-photos', { recursive: true });
    const fixture = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: false, geographic: true }),
    );
    await native({
      kind: 'save_and_scan',
      selection_id: fixture.id,
      label: 'Generated Map Photos',
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
      { timeout: 45000 },
    );
    await $('button[aria-label="Map"]').waitForDisplayed();
    await $('button[aria-label="Map"]').click();
    await expectOrdinaryPhotos();
    await showMapPhotos();
    assert.equal(
      await $('.unified-search').$('button[aria-label="Clear Map area filter"]').isDisplayed(),
      true,
    );
    assert.equal(await $('.photo-toolbar input[type="search"]').getValue(), '');
    await $('.photo-toolbar button[aria-label="Filter (1 active)"]').click();
    await $('#library-filter-panel button[aria-label="Remove Map area filter"]').waitForDisplayed();
    await $('button[aria-label="Close filters"]').click();
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      assert.equal(
        await browser.execute(() => {
          const chip = document.querySelector(
            '.unified-search button[aria-label="Clear Map area filter"]',
          );
          const rect = chip.getBoundingClientRect();
          return (
            rect.width > 0 &&
            rect.right <= innerWidth &&
            chip.contains(
              document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2),
            )
          );
        }),
        true,
      );
      await browser.saveScreenshot(`.context/map-photos/filtered-${width}.png`);
    }
    await browser.setWindowSize(1180, 780);
    await $('.photo-toolbar input[type="search"]').setValue('South.jpg');
    await $('button[aria-label="Clear Map area filter"]').click();
    assert.equal(await $('.photo-toolbar input[type="search"]').getValue(), 'South.jpg');
    assert.equal(await $('.map-filters').isExisting(), false);
    await showMapPhotos();
    await $('button=Return to map').click();
    await expectOrdinaryPhotos();
    await showMapPhotos();
    await expectOrdinaryPhotos();
    const audit = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.equal(audit.verifiedFiles, 9);
    await writeFile(
      '.context/map-photos/receipt.json',
      JSON.stringify(
        {
          audit,
          ordinaryNavigationUnfiltered: true,
          explicitScopeVisibleInSearchAndFilters: true,
          mapClearPreservesFilename: true,
        },
        null,
        2,
      ),
    );
  });
});
