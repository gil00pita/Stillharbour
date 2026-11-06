import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';

describe('Confirmed favorite heart burst', () => {
  it('animates saved favorites, settles, and can repeat without changing source media', async function () {
    this.timeout(60000);
    const invoke = (command, args) =>
      browser.tauri.execute(
        async ({ core }, command, args) => core.invoke(command, args),
        command,
        args,
      );
    const selection = await invoke('test_library_fixture', { verify: false });
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').click();
    await $('.selected-folder').waitForDisplayed();
    await $('.root-setup input[required]').setValue('Generated heart animation smoke');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          [...document.querySelectorAll('.asset img')].some(
            (image) => image.complete && image.naturalWidth > 0,
          ),
        ),
      { timeout: 45000 },
    );
    const assetId = await browser.execute(() => {
      const tile = [...document.querySelectorAll('.asset')].find((item) =>
        [...item.querySelectorAll('img')].some((image) => image.complete && image.naturalWidth > 0),
      );
      const opener = tile.querySelector('.asset-open');
      const id = opener.closest('.asset').dataset.assetId;
      opener.click();
      return id;
    });
    await $('.photo-viewer[open]').waitForDisplayed();
    const heart = await $('.viewer-favorite');
    assert.equal(await heart.getAttribute('aria-pressed'), 'false');
    await browser.execute(() => {
      const button = document.querySelector('.viewer-favorite');
      window.__heartBursts = [];
      window.__heartObserver = new MutationObserver(() => {
        if (!button.classList.contains('is-animating')) return;
        const shape = button.querySelector('.t-like-heart');
        const ring = button.querySelector('.t-like-ring');
        const spark = button.querySelector('.t-like-spark');
        window.__heartBursts.push({
          hidden: document.visibilityState === 'hidden',
          reduced: matchMedia('(prefers-reduced-motion: reduce)').matches,
          pressed: button.getAttribute('aria-pressed'),
          heart: getComputedStyle(shape).animationName,
          ring: getComputedStyle(ring).animationName,
          sparks: getComputedStyle(spark, '::before').animationName,
          duration: getComputedStyle(shape).animationDuration,
          pairs: button.querySelectorAll('.t-like-spark').length,
        });
      });
      window.__heartObserver.observe(button, { attributes: true, attributeFilter: ['class'] });
    });
    for (let repeat = 0; repeat < 3; repeat++) {
      if (repeat === 2) {
        // Exercise the suppressed path even on a visible local desktop. CI
        // runners may already occlude the WebView; its motion policy still holds.
        await browser.execute(() => {
          window.__heartVisibility = Object.getOwnPropertyDescriptor(document, 'visibilityState');
          Object.defineProperty(document, 'visibilityState', {
            configurable: true,
            get: () => 'hidden',
          });
          document.dispatchEvent(new Event('visibilitychange'));
        });
        await browser.waitUntil(() =>
          browser.execute(() =>
            document.querySelector('.viewer-favorite').classList.contains('without-motion'),
          ),
        );
      }
      await heart.click();
      await browser.waitUntil(async () => (await heart.getAttribute('aria-pressed')) === 'true');
      assert.equal(
        (await invoke('library_request', { request: { kind: 'detail', asset_id: assetId } })).data
          .asset.favorite,
        true,
      );
      await browser.waitUntil(() =>
        browser.execute(
          () => !document.querySelector('.viewer-favorite').classList.contains('is-animating'),
        ),
      );
      const settled = await browser.execute(() => {
        const shape = document.querySelector('.t-like-heart');
        return {
          fill: getComputedStyle(shape.querySelector('path')).fill,
          color: getComputedStyle(shape).color,
          animation: getComputedStyle(shape).animationName,
        };
      });
      assert.equal(settled.fill, settled.color);
      assert.equal(settled.animation, 'none');
      await heart.click();
      await browser.waitUntil(async () => (await heart.getAttribute('aria-pressed')) === 'false');
      assert.equal(
        await browser.execute(() =>
          document.querySelector('.viewer-favorite').classList.contains('is-animating'),
        ),
        false,
      );
    }
    const bursts = await browser.execute(() => {
      window.__heartObserver.disconnect();
      if (window.__heartVisibility)
        Object.defineProperty(document, 'visibilityState', window.__heartVisibility);
      else delete document.visibilityState;
      document.dispatchEvent(new Event('visibilitychange'));
      return window.__heartBursts;
    });
    assert.equal(bursts.length, 3);
    assert.equal(bursts.at(-1).hidden, true);
    for (const { hidden, reduced, ...burst } of bursts) {
      const suppressed = hidden || reduced;
      assert.deepEqual(burst, {
        pressed: 'true',
        heart: suppressed ? 'none' : 't-like-pop',
        ring: suppressed ? 'none' : 't-like-ring',
        sparks: suppressed ? 'none' : 't-like-sparks',
        duration: suppressed ? '0s' : '0.8s',
        pairs: 7,
      });
    }
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      await browser.waitUntil(() =>
        browser.execute(() => {
          const bounds = document.querySelector('.viewer-favorite').getBoundingClientRect();
          return (
            bounds.left >= 0 &&
            bounds.right <= innerWidth &&
            bounds.top >= 0 &&
            bounds.bottom <= innerHeight
          );
        }),
      );
      const geometry = await browser.execute(() => {
        const button = document.querySelector('.viewer-favorite').getBoundingClientRect();
        return {
          width: button.width,
          height: button.height,
          fits:
            button.left >= 0 &&
            button.right <= innerWidth &&
            button.top >= 0 &&
            button.bottom <= innerHeight,
        };
      });
      assert(
        geometry.width >= 44 && geometry.height >= 44 && geometry.fits,
        JSON.stringify(geometry),
      );
      await browser.saveScreenshot(`.context/heart-native-${width}.png`);
    }
    const verified = await invoke('test_library_fixture', { verify: true });
    assert(verified.verifiedFiles > 20);
  });
});
