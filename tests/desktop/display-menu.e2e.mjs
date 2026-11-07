import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';

async function openDisplay() {
  await browser.execute(() =>
    document.querySelector('button[aria-controls="library-display-panel"]').focus(),
  );
  await $('button[aria-controls="library-display-panel"]').click();
  await $('#library-display-panel[open]').waitForDisplayed();
}
async function setScale(value) {
  await browser.execute((value) => {
    const slider = document.querySelector('#library-display-panel input[type="range"]');
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(slider, value);
    slider.dispatchEvent(new Event('input', { bubbles: true }));
    slider.dispatchEvent(new Event('change', { bubbles: true }));
  }, value);
  await browser.waitUntil(() =>
    browser.execute(
      (value) =>
        document.querySelector('#library-display-panel input[type="range"]').value === value,
      value,
    ),
  );
}

describe('Separate display and filter panels — generated native fixture', () => {
  it('moves view/zoom/fit into Display, preserves query and supports both window sizes', async function () {
    this.timeout(120000);
    const output = '.context/display-menu';
    await mkdir(output, { recursive: true });
    const selection = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: false }),
    );
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').waitForEnabled();
    await $('.welcome button.primary').click();
    await $('.root-setup input[required]').setValue('Generated display menu');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    await $('input[aria-label="Search filenames, dates or places"]').setValue('jpg');
    await $('.asset-row.gallery', { strict: false }).waitForDisplayed();
    assert.equal(
      await browser.execute(
        () => document.querySelector('.app-page-header input[type="range"]') !== null,
      ),
      false,
    );
    assert.equal(
      (await $('button[aria-controls="library-filter-panel"]').getText()).includes('Filter'),
      false,
    );
    await browser.saveScreenshot(`${output}/toolbar.png`);

    await openDisplay();
    await $('#library-display-panel').$('button=Uniform grid').click();
    await setScale('2');
    assert.equal(await $('.asset-row.gallery', { strict: false }).isExisting(), true);
    await $('button=Cancel').click();
    await browser.waitUntil(() =>
      browser.execute(() => !document.querySelector('#library-display-panel')),
    );
    await openDisplay();
    assert.equal(
      await $('#library-display-panel button[aria-pressed="true"]').getText(),
      'Timeline',
    );
    assert.equal(await $('#library-display-panel input[type="range"]').getValue(), '1');
    await $('#library-display-panel').$('button=Uniform grid').click();
    await setScale('2');
    // The embedded driver's option.click() does not change native selects.
    await browser.execute(() => {
      const select = document.querySelector('select[aria-label="Grid image fit"]');
      select.value = 'cover';
      select.dispatchEvent(new Event('change', { bubbles: true }));
    });
    await $('button=Apply display').click();
    await $('.asset-row.grid', { strict: false }).waitForDisplayed();
    assert.equal(
      await $('input[aria-label="Search filenames, dates or places"]').getValue(),
      'jpg',
    );
    const focus = await browser.execute(() => ({
      observed: document.hasFocus(),
      restored: document.activeElement?.getAttribute('aria-controls') === 'library-display-panel',
    }));
    assert.equal(focus.restored, true);

    const geometry = [];
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      for (const appearance of ['light', 'dark']) {
        if (await $(`button[aria-label="Switch to ${appearance} appearance"]`).isExisting())
          await $(`button[aria-label="Switch to ${appearance} appearance"]`).click();
        await openDisplay();
        const metrics = await browser.execute(() => {
          const panel = document.querySelector('#library-display-panel');
          const p = panel.getBoundingClientRect();
          const s = panel.querySelector('input[type="range"]').getBoundingClientRect();
          return {
            width: innerWidth,
            height: innerHeight,
            panel: p.toJSON(),
            scale: s.toJSON(),
            size: panel.querySelector('input[type="range"]').getAttribute('aria-valuetext'),
            fit: panel.querySelector('select').value,
            controlsFit: [...panel.querySelectorAll('button,input,select')].every((e) => {
              const b = e.getBoundingClientRect();
              return (
                b.width === 0 ||
                (b.left >= p.left && b.right <= p.right && b.top >= p.top && b.bottom <= p.bottom)
              );
            }),
          };
        });
        geometry.push({ appearance, ...metrics });
        await browser.saveScreenshot(`${output}/display-${appearance}-${width}.png`);
        assert(
          metrics.panel.left >= 0 &&
            metrics.panel.right <= metrics.width &&
            metrics.panel.bottom <= metrics.height,
        );
        assert(metrics.scale.width > 100);
        assert.equal(metrics.controlsFit, true);
        assert.equal(metrics.size, 'large');
        assert.equal(metrics.fit, 'cover');
        await $('button[aria-label="Close display options"]').click();
      }
    }
    await openDisplay();
    await $('#library-display-panel').$('button=Text list').click();
    assert.equal(await $('#library-display-panel input[type="range"]').isExisting(), false);
    await browser.execute(() => {
      window.__displayRequests = [];
      window.addEventListener('stillharbour:test-library-request', (e) =>
        window.__displayRequests.push(e.detail),
      );
    });
    await $('button=Apply display').click();
    await $('.asset-row.list', { strict: false }).waitForDisplayed();
    assert.equal(
      await browser.execute(
        () =>
          window.__displayRequests.filter((r) => r.kind === 'thumbnails' && r.keys.length > 0)
            .length,
      ),
      0,
    );

    await $('button[aria-controls="library-filter-panel"]').click();
    await $('#library-filter-panel[open]').waitForDisplayed();
    assert.equal(await $('#library-filter-panel').$('button=Uniform grid').isExisting(), false);
    assert.equal(await $('#library-filter-panel input[type="range"]').isExisting(), false);
    await $('button[aria-label="Close filters"]').click();
    const audit = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.ok(audit.verifiedFiles > 20);
    await writeFile(`${output}/receipt.json`, JSON.stringify({ geometry, focus, audit }, null, 2));
  });
});
