import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';
import { mkdir } from 'node:fs/promises';

describe('Bottom application error notification', () => {
  it('keeps complete errors visible, deduplicates, and dismisses without shifting the workspace', async function () {
    this.timeout(60000);
    await mkdir('.context/error-toast', { recursive: true });
    const picker = $('.welcome button.primary');
    await picker.waitForEnabled({ timeout: 30000 });
    // Existing desktop-test-only IPC interceptor: inject a picker failure,
    // never fill a personal catalog queue or invoke an original-file operation.
    await browser.execute(() => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = ({ request }) => {
        if (request.kind !== 'pick_directory')
          throw new Error('Unexpected generated fixture request');
        throw new Error(
          'Generated notification fixture: the catalog request queue is full (16 waiting requests). This request was not queued. Wait a moment, then try again.',
        );
      };
    });
    const toast = $('[data-scope="toast"][data-part="root"]');
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      const before = await browser.execute(
        () => document.querySelector('.welcome').getBoundingClientRect().top,
      );
      await picker.click();
      await toast.waitForDisplayed();
      const geometry = await browser.execute(() => {
        const node = document.querySelector('[data-scope="toast"][data-part="root"]');
        const rect = node.getBoundingClientRect();
        const description = node.querySelector('[data-part="description"]');
        return {
          count: document.querySelectorAll('[data-scope="toast"][data-part="root"]').length,
          text: description.textContent,
          placement: node.dataset.placement,
          portal: node.closest('.app-shell') === null,
          opacity: getComputedStyle(node).opacity,
          viewportWidth: innerWidth,
          viewportHeight: innerHeight,
          left: rect.left,
          right: rect.right,
          bottom: rect.bottom,
          welcomeTop: document.querySelector('.welcome').getBoundingClientRect().top,
          horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
        };
      });
      assert.equal(geometry.count, 1);
      assert.equal(geometry.placement, 'bottom');
      assert.equal(geometry.portal, true);
      assert.equal(Number(geometry.opacity), 1);
      assert.match(geometry.text, /16 waiting requests/);
      assert.match(geometry.text, /try again\.$/);
      assert.ok(geometry.left >= 15 && geometry.right <= geometry.viewportWidth - 15);
      assert.ok(Math.abs(geometry.viewportHeight - geometry.bottom - 16) <= 2);
      assert.equal(geometry.welcomeTop, before);
      assert.equal(geometry.horizontalOverflow, false);
      for (let i = 0; i < 3; i++) await picker.click();
      // Picker rejection and React's toast update settle after the native click returns.
      await browser.waitUntil(
        async () =>
          browser.execute(
            () => document.querySelectorAll('[data-scope="toast"][data-part="root"]').length === 1,
          ),
        { timeoutMsg: 'The final failed request did not settle to one error notification' },
      );
      assert.equal(
        await browser.execute(
          () => document.querySelectorAll('[data-scope="toast"][data-part="root"]').length,
        ),
        1,
      );
      await browser.saveScreenshot(`.context/error-toast/native-${width}.png`);
      await $('button[aria-label="Dismiss error"]').click();
      await toast.waitForExist({ reverse: true });
    }
    await picker.click();
    await toast.waitForDisplayed();
    await browser.execute(() =>
      document.querySelector('[data-scope="toast"][data-part="root"]').focus(),
    );
    await browser.keys('Escape');
    await toast.waitForExist({ reverse: true });
  });
});
