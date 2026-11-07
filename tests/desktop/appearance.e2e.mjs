import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';

describe('Neutral gallery appearance', () => {
  it('themes the real gallery, saves a thumbnail favorite and preserves generated originals', async function () {
    this.timeout(120000);
    await mkdir('.context/redesign-native/screenshots', { recursive: true });
    const selection = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: false }),
    );
    await $('.welcome button.primary').waitForEnabled();
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').click();
    await $('.root-setup input[required]').setValue('Generated appearance library');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    const photo = await browser.execute(() => {
      const article = document.querySelector('.asset');
      const button = article.querySelector('.photo-favorite');
      return { id: article.dataset.assetId, label: button.getAttribute('aria-label') };
    });
    // Native fixture filenames deliberately include quotes and newlines. Use
    // the stable asset identity rather than treating the display name as CSS.
    const favorite = await $(`[data-asset-id="${photo.id}"] .photo-favorite`);
    await favorite.click();
    await browser.waitUntil(async () =>
      browser.execute(
        (id) =>
          document
            .querySelector(`[data-asset-id="${id}"] .photo-favorite`)
            ?.getAttribute('aria-pressed') === 'true',
        photo.id,
      ),
    );
    assert.equal(await $('.photo-viewer').isExisting(), false);
    const saved = await browser.tauri.execute(
      ({ core }, id) =>
        core.invoke('library_request', { request: { kind: 'detail', asset_id: id } }),
      photo.id,
    );
    assert.equal(saved.data.asset.favorite, true);
    const measurements = [];
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      for (const appearance of ['dark', 'light']) {
        const current = await browser.execute(() =>
          document.documentElement.classList.contains('dark') ? 'dark' : 'light',
        );
        if (current !== appearance)
          await $(`button[aria-label="Switch to ${appearance} appearance"]`).click();
        await browser.waitUntil(() =>
          browser.execute(
            (appearance) => document.documentElement.classList.contains(appearance),
            appearance,
          ),
        );
        const geometry = await browser.execute(() => {
          const header = document.querySelector('.app-page-header');
          const rail = document.querySelector('.sidebar');
          const row = document.querySelector('.asset-row');
          const bounds = header.getBoundingClientRect();
          const tile = document.querySelector('.asset');
          const check = tile.querySelector('.asset-check').getBoundingClientRect();
          const heart = tile.querySelector('.photo-favorite').getBoundingClientRect();
          const checkMark = tile
            .querySelector('.thumbnail-selection > span')
            .getBoundingClientRect();
          const heartMark = tile.querySelector('.photo-favorite > span').getBoundingClientRect();
          return {
            width: innerWidth,
            height: innerHeight,
            appearance: localStorage.getItem('stillharbour-appearance'),
            background: getComputedStyle(document.documentElement).backgroundColor,
            font: getComputedStyle(header).fontFamily,
            gutter: getComputedStyle(row).gap,
            header: bounds.height,
            rail: rail.getBoundingClientRect().width,
            controlsFit: [...header.querySelectorAll('button')].every((button) => {
              const r = button.getBoundingClientRect();
              return (
                r.width === 0 ||
                (r.left >= bounds.left &&
                  r.right <= innerWidth &&
                  r.top >= bounds.top &&
                  r.bottom <= bounds.bottom)
              );
            }),
            executableInlineScripts: document.querySelectorAll(
              'script:not([src]):not([type="application/json"])',
            ).length,
            thumbnailControls: {
              checkboxSize: [check.width, check.height],
              heartSize: [heart.width, heart.height],
              checkboxMark: [checkMark.width, checkMark.height],
              heartMark: [heartMark.width, heartMark.height],
              centreError: tile.hasAttribute('data-narrow-controls')
                ? Math.abs(check.left + check.width / 2 - heart.left - heart.width / 2)
                : Math.abs(check.top + check.height / 2 - heart.top - heart.height / 2),
              buttonBackground: getComputedStyle(tile.querySelector('.photo-favorite'))
                .backgroundColor,
              foreground: getComputedStyle(tile.querySelector('.photo-favorite > span')).color,
              targetsSeparate: [...document.querySelectorAll('.asset')].every((asset) => {
                const check = asset.querySelector('.asset-check').getBoundingClientRect();
                const heart = asset.querySelector('.photo-favorite').getBoundingClientRect();
                return check.right <= heart.left + 1 || check.bottom <= heart.top + 1;
              }),
            },
          };
        });
        measurements.push(geometry);
        await browser.saveScreenshot(
          `.context/redesign-native/screenshots/gallery-${appearance}-${width}.png`,
        );
        assert.equal(geometry.header, 64);
        assert.equal(geometry.gutter, '6px');
        assert.equal(geometry.controlsFit, true, JSON.stringify(geometry));
        assert.ok(geometry.font.includes('Inter Variable'));
        assert.equal(geometry.executableInlineScripts, 0);
        assert.deepEqual(geometry.thumbnailControls.checkboxSize, [36, 36]);
        assert.deepEqual(geometry.thumbnailControls.heartSize, [36, 36]);
        assert.deepEqual(geometry.thumbnailControls.checkboxMark, [28, 28]);
        assert.deepEqual(geometry.thumbnailControls.heartMark, [28, 28]);
        assert.ok(geometry.thumbnailControls.centreError <= 1);
        assert.equal(geometry.thumbnailControls.buttonBackground, 'rgba(0, 0, 0, 0)');
        assert.equal(geometry.thumbnailControls.foreground, 'rgb(248, 248, 250)');
        assert.equal(geometry.thumbnailControls.targetsSeparate, true);
      }
    }
    await browser.setWindowSize(1180, 780);
    if (await $('button[aria-label="Switch to dark appearance"]').isExisting())
      await $('button[aria-label="Switch to dark appearance"]').click();
    const focusAvailable = await browser.execute(() => !document.hidden && document.hasFocus());
    if (focusAvailable) {
      await browser.execute(() => document.querySelector('.asset-open').focus());
      await browser.waitUntil(() =>
        browser.execute(
          () => getComputedStyle(document.querySelector('.asset-check')).opacity === '1',
        ),
      );
    }
    // Background WKWebViews do not expose focus pseudo-classes. The ordinary
    // Select action remains available; record focus evidence separately.
    await $('button=Select').click();
    await $('.asset-check', { strict: false }).click();
    await $('[aria-label="Selected photo actions"]').waitForDisplayed();
    const selectedCheck = await browser.execute(() => {
      const checkbox = document.querySelector('.asset-check:checked');
      const box = checkbox.getBoundingClientRect();
      return {
        opacity: getComputedStyle(checkbox).opacity,
        markOpacity: getComputedStyle(checkbox.nextElementSibling).opacity,
        onTop:
          document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2) ===
          checkbox,
      };
    });
    assert.deepEqual(selectedCheck, { opacity: '1', markOpacity: '1', onTop: true });
    await browser.saveScreenshot('.context/redesign-native/screenshots/selection-dark-1180.png');
    const selectionFits = await browser.execute(() => {
      const box = document.querySelector('.selection-area').getBoundingClientRect();
      return box.left >= 0 && box.right <= innerWidth && box.top >= 64 && box.bottom <= innerHeight;
    });
    assert.equal(selectionFits, true);
    await $('button=Done selecting').click();
    await browser.execute(() => document.querySelectorAll('.asset-open')[3].click());
    await $('button[aria-label="Info"]').waitForDisplayed();
    await $('button[aria-label="Info"]').click();
    await $('.inspector').waitForDisplayed();
    await browser.saveScreenshot('.context/redesign-native/screenshots/inspector-dark-1180.png');
    const viewerGeometry = await browser.execute(() => {
      const button = document.querySelector('.viewer-favorite').getBoundingClientRect();
      const icon = document.querySelector('.t-like-heart').getBoundingClientRect();
      return {
        toolbarTop: document.querySelector('.viewer-toolbar').getBoundingClientRect().top,
        heart: icon.toJSON(),
        heartFits:
          icon.left >= button.left &&
          icon.right <= button.right &&
          icon.top >= button.top &&
          icon.bottom <= button.bottom,
      };
    });
    await writeFile(
      '.context/redesign-native/viewer-appearance.json',
      JSON.stringify(viewerGeometry, null, 2),
    );
    assert.equal(viewerGeometry.heartFits, true, JSON.stringify(viewerGeometry));
    assert.ok(viewerGeometry.toolbarTop >= 0, JSON.stringify(viewerGeometry));
    const audit = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.ok(audit.verifiedFiles > 20);
    await writeFile(
      '.context/redesign-native/appearance.json',
      JSON.stringify({ measurements, focusObserved: focusAvailable, audit }, null, 2),
    );
  });
});
