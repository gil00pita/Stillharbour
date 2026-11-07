import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';
import { mkdir } from 'node:fs/promises';

describe('Reference timeline navigation through bounded native pages', () => {
  it('navigates during a captured drag, retains the rail across seeks, and follows the actual photo position', async function () {
    this.timeout(120000);
    await mkdir('.context/timeline-reference', { recursive: true });
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
    await $('.root-setup input[required]').setValue('Generated reference timeline');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    // This hook creates catalog-only placeholder records with no preview bytes.
    // Use the real Text list control so the test never requests fake thumbnails.
    await $('button[aria-controls="library-filter-panel"]').click();
    await $('#library-filter-panel[open]').waitForDisplayed();
    await $('#library-filter-panel').$('button=Text list').click();
    await $('button=Apply filters').click();
    await browser.waitUntil(() =>
      browser.execute(() => !document.querySelector('#library-filter-panel[open]')),
    );
    assert.equal(await invoke('test_scroll_catalog'), 960);
    await browser.execute(() => {
      window.__timelineRequests = [];
      window.addEventListener('stillharbour:test-library-request', (event) => {
        if (event.detail.kind === 'seek_index') window.__timelineRequests.push(event.detail);
      });
    });
    await $('input[aria-label="Search filenames, dates or places"]').setValue('Synthetic');
    await browser.waitUntil(() =>
      browser.execute(() => document.querySelector('.date-scrubber')?.max === '959'),
    );
    await browser.setWindowSize(1180, 780);
    const initial = await browser.execute(() => {
      const input = document.querySelector('.date-scrubber');
      window.__timelineInput = input;
      const bounds = input.getBoundingClientRect();
      return {
        x: Math.round(bounds.left + bounds.width / 2),
        hoverY: Math.round(bounds.top + bounds.height * 0.3),
        dragY: Math.round(bounds.top + bounds.height * 0.7),
      };
    });
    await browser.performActions([
      {
        type: 'pointer',
        id: 'reference',
        parameters: { pointerType: 'mouse' },
        actions: [{ type: 'pointerMove', duration: 0, x: initial.x, y: initial.hoverY }],
      },
    ]);
    assert.equal(await browser.execute(() => window.__timelineRequests.length), 0);
    await browser.performActions([
      {
        type: 'pointer',
        id: 'reference',
        parameters: { pointerType: 'mouse' },
        actions: [
          { type: 'pointerMove', duration: 0, x: initial.x, y: initial.dragY },
          { type: 'pointerDown', button: 0 },
        ],
      },
    ]);
    await browser.waitUntil(() => browser.execute(() => window.__timelineRequests.length > 0));
    // This assertion runs while the pointer is still down. Release-only seeking fails it.
    const first = await browser.execute(() => window.__timelineRequests.at(-1));
    const expected = await invoke('library_request', { request: { ...first, limit: 1 } });
    assert.equal(expected.data.rows.length, 1);
    const id = expected.data.rows[0][0];
    await browser.waitUntil(() =>
      browser.execute(
        (id) => document.querySelector('.asset-viewport .asset')?.dataset.assetId === id,
        id,
      ),
    );
    assert.equal(
      await browser.execute(
        () => window.__timelineInput === document.querySelector('.date-scrubber'),
      ),
      true,
    );
    assert.equal(
      await browser.execute(() => document.querySelector('.date-rail').dataset.dragging),
      'true',
    );
    assert(
      await browser.execute(
        () => Number(document.querySelector('.collection-main').dataset.feedCount) <= 720,
      ),
    );
    await browser.performActions([
      {
        type: 'pointer',
        id: 'reference',
        parameters: { pointerType: 'mouse' },
        actions: [
          { type: 'pointerMove', duration: 100, x: initial.x - 80, y: initial.dragY - 100 },
          { type: 'pointerUp', button: 0 },
        ],
      },
    ]);
    await browser.waitUntil(() =>
      browser.execute(() => document.querySelector('.date-rail').dataset.dragging === 'false'),
    );
    await browser.execute(() => document.querySelector('.date-scrubber').focus());
    await browser.keys('End');
    await browser.waitUntil(() =>
      browser.execute(() => document.querySelector('.date-scrubber').value === '959'),
    );
    const lastRequest = await browser.execute(() => window.__timelineRequests.at(-1));
    const lastPage = await invoke('library_request', { request: { ...lastRequest, limit: 1 } });
    await browser.waitUntil(() =>
      browser.execute(
        (id) => document.querySelector('.asset-viewport .asset')?.dataset.assetId === id,
        lastPage.data.rows[0][0],
      ),
    );
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      await browser.performActions([
        {
          type: 'pointer',
          id: 'reference',
          parameters: { pointerType: 'mouse' },
          actions: [{ type: 'pointerMove', duration: 0, x: 20, y: 80 }],
        },
      ]);
      await browser.execute(() =>
        document.querySelector('.date-scrubber').focus({ preventScroll: true }),
      );
      await browser
        .waitUntil(() =>
          browser.execute(
            () => Number(getComputedStyle(document.querySelector('.date-rail')).opacity) >= 0.99,
          ),
        )
        .catch(async (error) => {
          console.log(
            'Timeline visibility diagnostics',
            await browser.execute(() => ({
              hidden: document.hidden,
              active: document.activeElement.className,
              rail: document.querySelector('.date-rail').className,
              opacity: getComputedStyle(document.querySelector('.date-rail')).opacity,
            })),
          );
          throw error;
        });
      const metrics = await browser.execute(() => {
        const input = document.querySelector('.date-scrubber');
        input.focus({ preventScroll: true });
        const bounds = input.getBoundingClientRect();
        const line = document.querySelector('.date-scrubber-track i').getBoundingClientRect();
        const date = document.querySelector('.timeline-current').getBoundingClientRect();
        const expected = bounds.top + (Number(input.value) / Number(input.max)) * bounds.height;
        return {
          width: bounds.width,
          lineWidth: line.width,
          lineHeight: line.height,
          labelHeight: date.height,
          lineError: Math.abs(line.top - expected),
          labelError: Math.abs(date.bottom - expected),
          labelKind: document.querySelector('.timeline-current').getAttribute('aria-label'),
          opacity: getComputedStyle(document.querySelector('.date-rail')).opacity,
          active: document.activeElement.className,
          overflow: document.documentElement.scrollWidth > innerWidth,
        };
      });
      const idle = await browser.execute(() => {
        document.activeElement?.blur();
        const rail = document.querySelector('.date-rail');
        rail.dispatchEvent(new PointerEvent('pointerleave', { bubbles: true }));
        const bounds = rail.getBoundingClientRect();
        const date = rail.querySelector('.timeline-current').getBoundingClientRect();
        const viewport = document.querySelector('.browser-photos').getBoundingClientRect();
        return {
          opacity: getComputedStyle(rail).opacity,
          contained:
            date.left >= bounds.left &&
            date.right <= bounds.right &&
            date.top >= bounds.top &&
            date.bottom <= bounds.bottom,
          separate: date.left >= viewport.right,
        };
      });
      assert.deepEqual(idle, { opacity: '1', contained: true, separate: true });
      assert.equal(metrics.width, 96);
      assert.equal(metrics.lineWidth, 40);
      assert.equal(metrics.lineHeight, 1);
      assert.equal(metrics.labelHeight, 24);
      assert(metrics.lineError <= 1 && metrics.labelError <= 1, JSON.stringify(metrics));
      assert.equal(metrics.overflow, false);
      await browser.saveScreenshot(`.context/timeline-reference/reference-native-${width}.png`);
    }
    await browser.execute(() =>
      document.querySelector('.date-scrubber').focus({ preventScroll: true }),
    );
    await browser.keys('Home');
    await browser.waitUntil(() =>
      browser.execute(() => document.querySelector('.date-scrubber').value === '0'),
    );
    const firstPage = await invoke('library_request', {
      request: { ...lastRequest, index: 0, limit: 1 },
    });
    await browser.waitUntil(() =>
      browser.execute(
        (id) => document.querySelector('.asset-viewport .asset')?.dataset.assetId === id,
        firstPage.data.rows[0][0],
      ),
    );
    const wheel = await browser.execute(() => {
      const input = document.querySelector('.date-scrubber');
      const bounds = input.getBoundingClientRect();
      return {
        x: Math.round(bounds.left + bounds.width / 2),
        y: Math.round(bounds.top + bounds.height * 0.4),
      };
    });
    await browser.performActions([
      {
        type: 'pointer',
        id: 'reference',
        parameters: { pointerType: 'mouse' },
        actions: [
          { type: 'pointerMove', duration: 0, x: wheel.x, y: wheel.y },
          { type: 'pointerDown', button: 0 },
        ],
      },
    ]);
    await browser.waitUntil(() =>
      browser.execute(
        () => Math.abs(Number(document.querySelector('.date-scrubber').value) - 959 * 0.4) < 5,
      ),
    );
    await browser.keys('Escape');
    const cancelled = await browser.execute(() => ({
      index: Number(document.querySelector('.date-scrubber').value),
      requests: window.__timelineRequests.length,
    }));
    await browser.performActions([
      {
        type: 'pointer',
        id: 'reference',
        parameters: { pointerType: 'mouse' },
        actions: [
          { type: 'pointerMove', duration: 0, x: wheel.x - 80, y: wheel.y + 100 },
          { type: 'pointerUp', button: 0 },
        ],
      },
    ]);
    const afterCancel = await browser.execute(() => ({
      index: Number(document.querySelector('.date-scrubber').value),
      requests: window.__timelineRequests,
    }));
    assert(
      Math.abs(afterCancel.index - cancelled.index) <= 5,
      JSON.stringify({ cancelled, afterCancel }),
    );
    assert(
      afterCancel.requests
        .slice(cancelled.requests)
        .every((request) => Math.abs(request.index - cancelled.index) <= 5),
    );
    const verified = await invoke('test_library_fixture', { verify: true });
    assert(verified.verifiedFiles > 20);
  });
});
