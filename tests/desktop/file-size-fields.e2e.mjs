import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';

describe('Settings file size field layout', () => {
  it('aligns amount and unit controls with native and bottom-edge select baselines', async () => {
    await $('button[aria-label="Settings"]').click();
    await $('.settings-defaults-form').waitForDisplayed();
    await $('.settings-defaults-form .scan-options summary').click();
    await mkdir('.context/screenshots', { recursive: true });
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      await browser.execute(() =>
        document
          .querySelector('.settings-defaults-form .file-size-controls')
          .scrollIntoView({ block: 'center' }),
      );
      for (const fallback of [false, true]) {
        const geometry = await browser.execute((fallback) => {
          const fields = document.querySelectorAll('.settings-defaults-form .file-size-controls');
          return [...fields].map((field) => {
            const input = field.querySelector('input');
            const select = field.querySelector('select');
            // Layout containment suppresses the intrinsic baseline and exercises
            // the bottom-edge fallback without changing either control's size.
            select.style.contain = fallback ? 'layout' : '';
            const amount = input.getBoundingClientRect();
            const unit = select.getBoundingClientRect();
            const wrapper = select.parentElement.getBoundingClientRect();
            return {
              name: input.name,
              amountHeight: amount.height,
              unitHeight: unit.height,
              wrapperHeight: wrapper.height,
              topOffset: amount.top - unit.top,
              fits: amount.left >= 0 && unit.right <= innerWidth && amount.right <= unit.left,
            };
          });
        }, fallback);
        const evidence = `.context/screenshots/file-size-${width}-${fallback ? 'fallback' : 'native'}`;
        await writeFile(`${evidence}.json`, JSON.stringify(geometry, null, 2));
        await browser.saveScreenshot(`${evidence}.png`);
        assert.equal(geometry.length, 2);
        for (const field of geometry) {
          assert.ok(field.amountHeight >= 36 && field.unitHeight >= 36);
          assert.ok(Math.abs(field.amountHeight - field.unitHeight) <= 1, JSON.stringify(field));
          assert.ok(Math.abs(field.wrapperHeight - field.unitHeight) <= 1, JSON.stringify(field));
          assert.ok(Math.abs(field.topOffset) <= 1, JSON.stringify(field));
          assert.equal(field.fits, true, JSON.stringify(field));
        }
      }
    }
  });
});
