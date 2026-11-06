import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';
import { assertViewerControlsOverlay } from './viewer-controls.mjs';

const toastSelector = '[data-scope="toast"][data-part="root"]';
const previewError = 'The requested preview asset is unavailable.';

async function showPickerError(message) {
  await $('button[aria-label="Settings"]').click();
  await browser.execute((message) => {
    window.__wdio_mocks__.foundation_request = () => {
      throw new Error(message);
    };
  }, message);
  await $('button=Add photo folder').click();
  await $(toastSelector).waitForDisplayed();
  assert.equal(await $(`${toastSelector} [data-part="description"]`).getText(), message);
  await $('button[aria-label="Photos"]').click();
  await browser.waitUntil(() =>
    browser.execute(() =>
      [...document.querySelectorAll('.asset img')].some(
        (image) => image.complete && image.naturalWidth > 0,
      ),
    ),
  );
  await browser.execute(() => {
    const tile = [...document.querySelectorAll('.asset')].find((tile) =>
      [...tile.querySelectorAll('img')].some((image) => image.complete && image.naturalWidth > 0),
    );
    tile.querySelector('.asset-open').click();
  });
  await $('.photo-viewer[open]').waitForDisplayed();
}

describe('Viewer controls after an expected generated-preview error', () => {
  it('acknowledges the fixture notification before testing targets and preserves unexpected errors', async function () {
    this.timeout(60000);
    const selection = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: false }),
    );
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').click();
    await $('.selected-folder').waitForDisplayed();
    await $('.root-setup input[required]').setValue('Generated viewer controls');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });

    // Use the existing test-only picker interceptor to reproduce the same
    // application error state seen in CI, without changing files or CSS.
    await showPickerError(previewError);
    const observations = {};
    for (const info of [false, true]) {
      if (info) await $('button[aria-label="Info"]').click();
      for (const [width, height] of [
        [1180, 780],
        [660, 560],
      ]) {
        await browser.setWindowSize(width, height);
        await assertViewerControlsOverlay(observations);
        assert.equal(await $(toastSelector).isExisting(), false);
      }
    }
    assert.deepEqual(observations.viewerPreviewErrorsAcknowledged, [previewError]);
    await $('.viewer-back').click();
    await $('.photo-viewer').waitForExist({ reverse: true });
    await showPickerError('Generated unexpected catalog failure');
    await assert.rejects(
      assertViewerControlsOverlay(),
      /Unexpected application error before viewer control checks/,
    );
    assert.equal(await $(toastSelector).isDisplayed(), true);
    assert.match(await $(toastSelector).getText(), /Generated unexpected catalog failure/);
    await $('button[aria-label="Dismiss error"]').click();
    await $(toastSelector).waitForExist({ reverse: true });
    const verified = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert(verified.verifiedFiles > 20);
  });
});
