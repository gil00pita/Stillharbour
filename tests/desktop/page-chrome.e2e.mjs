import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';
import { photoControlsReady } from './photo-controls-ready.mjs';

async function movePhotoPointer(point) {
  if (process.platform !== 'win32') {
    // Retain the established non-Windows embedded-driver setup. macOS cursor
    // warping alone does not deliver a native mouse-moved event to WKWebView.
    await browser.performActions([
      {
        type: 'pointer',
        id: 'photo-controls-rest',
        parameters: { pointerType: 'mouse' },
        actions: [{ type: 'pointerMove', duration: 0, ...point }],
      },
    ]);
    await browser.releaseActions();
    return;
  }
  // The embedded driver's pointerMove dispatches DOM events; it does not move
  // the OS cursor or reliably update CSS :hover. Use the instrumented window's
  // native cursor API, with logical coordinates so this also works at high DPI.
  await browser.tauri.execute(
    ({ core }, point) =>
      core.invoke('plugin:window|set_cursor_position', {
        label: 'main',
        value: { Logical: point },
      }),
    point,
  );
}

async function restPhotoControls() {
  // Scan completion can put a new tile under the pointer left by setup. Move
  // both hover and keyboard focus away before measuring the browsing state.
  await browser.execute(() =>
    document
      .querySelector('input[aria-label="Search filenames, dates or places"]')
      .focus({ preventScroll: true }),
  );
  await movePhotoPointer({ x: 20, y: 20 });
  await browser.waitUntil(
    () =>
      browser.execute(() => {
        const checkbox = document.querySelector('.asset-check');
        const tile = checkbox.closest('.asset');
        return (
          !tile.matches(':hover') &&
          !tile.contains(document.activeElement) &&
          getComputedStyle(checkbox).opacity === '0'
        );
      }),
    {
      timeout: 3000,
      timeoutMsg: 'Unselected photo controls did not hide after hover and focus left',
    },
  );
}

async function photoContext(atEdge = false) {
  const anchor = await browser.execute((atEdge) => {
    const photo = document.querySelector('.asset-open');
    const bounds = photo.getBoundingClientRect();
    const point = atEdge
      ? { x: innerWidth - 4, y: innerHeight - 4 }
      : { x: bounds.left + 20, y: bounds.top + 20 };
    photo.dispatchEvent(
      new MouseEvent('contextmenu', {
        bubbles: true,
        cancelable: true,
        button: 2,
        clientX: point.x,
        clientY: point.y,
      }),
    );
    return point;
  }, atEdge);
  await $('[role="menu"]').waitForDisplayed();
  await browser
    .waitUntil(
      async () =>
        browser.execute(
          ({ anchor, atEdge }) => {
            const rect = document.querySelector('[role="menu"]').getBoundingClientRect();
            // Allow Chakra's 4 px alignment inset plus device-pixel rounding.
            if (!atEdge)
              return Math.abs(rect.left - anchor.x) <= 8 && Math.abs(rect.top - anchor.y) <= 8;
            return (
              rect.left >= 0 &&
              rect.top >= 0 &&
              rect.right <= innerWidth &&
              rect.bottom <= innerHeight &&
              Math.abs(rect.right - anchor.x) <= 12 &&
              Math.abs(rect.bottom - anchor.y) <= 12
            );
          },
          { anchor, atEdge },
        ),
      {
        timeout: 5000,
        timeoutMsg: `Photo menu must open at its context point ${JSON.stringify(anchor)} (edge=${atEdge})`,
      },
    )
    .catch(async (error) => {
      const geometry = await browser.execute(() => {
        const menu = document.querySelector('[role="menu"]');
        const positioner = menu.closest('[data-part="positioner"]');
        return {
          hidden: document.hidden,
          focused: document.hasFocus(),
          state: menu.dataset.state,
          rect: menu.getBoundingClientRect().toJSON(),
          positioner: positioner?.getBoundingClientRect().toJSON(),
          style: positioner?.getAttribute('style'),
          viewport: { width: innerWidth, height: innerHeight },
        };
      });
      await writeFile(
        '.context/menu-position/geometry-failure.json',
        JSON.stringify({ anchor, atEdge, geometry }, null, 2),
      );
      await browser.saveScreenshot('.context/menu-position/position-failure.png');
      throw error;
    });
}

async function measureHeader(label) {
  const measured = await browser.execute(() => {
    const header = [...document.querySelectorAll('.app-page-header')].find(
      (e) => e.getBoundingClientRect().height > 0,
    );
    const brand = document.querySelector('.brand');
    const title = header.querySelector('h1');
    const context = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    title.dispatchEvent(context);
    const sidebarContext = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    brand.dispatchEvent(sidebarContext);
    return {
      header: header.getBoundingClientRect().height,
      brand: brand.getBoundingClientRect().height,
      top: header.getBoundingClientRect().top,
      brandTop: brand.getBoundingClientRect().top,
      lineHeight: getComputedStyle(title).lineHeight,
      titleSelect:
        getComputedStyle(title).getPropertyValue('-webkit-user-select') ||
        getComputedStyle(title).userSelect,
      sidebarSelect:
        getComputedStyle(brand.closest('aside')).getPropertyValue('-webkit-user-select') ||
        getComputedStyle(brand.closest('aside')).userSelect,
      titleContextBlocked: context.defaultPrevented,
      sidebarContextBlocked: sidebarContext.defaultPrevented,
      overflow: document.documentElement.scrollWidth > innerWidth,
      controlHeights: [...header.querySelectorAll('button, input[type=search]')]
        .filter((e) => e.getBoundingClientRect().height > 0)
        .map((e) => e.getBoundingClientRect().height),
      search: header.querySelector('.search-input-frame')?.getBoundingClientRect().toJSON(),
      actionsFit: [...header.querySelectorAll('button')]
        .filter((e) => e.getBoundingClientRect().height > 0)
        .every((e) => {
          const bounds = e.getBoundingClientRect();
          const row = header.getBoundingClientRect();
          return (
            bounds.top >= row.top &&
            bounds.bottom <= row.bottom &&
            bounds.right <= innerWidth &&
            e.contains(
              document.elementFromPoint(
                bounds.left + bounds.width / 2,
                bounds.top + bounds.height / 2,
              ),
            )
          );
        }),
    };
  });
  assert.equal(measured.header, measured.brand, `${label}: ${JSON.stringify(measured)}`);
  assert.equal(measured.header, 64, label);
  assert.equal(measured.top, measured.brandTop, label);
  assert.equal(measured.lineHeight, '24px', label);
  assert.equal(measured.titleSelect, 'none', label);
  assert.equal(measured.sidebarSelect, 'none', label);
  assert.equal(measured.titleContextBlocked, true, label);
  assert.equal(measured.sidebarContextBlocked, true, label);
  assert.equal(measured.overflow, false, label);
  assert.equal(measured.actionsFit, true, `${label}: actions must fit inside the header`);
  assert(
    measured.controlHeights.every((height) => height >= 34 && height <= 36),
    `${label}: header controls must share 36 px geometry ${measured.controlHeights}`,
  );
  return { label, ...measured };
}
describe('Page chrome and explicit photo selection — generated native fixture', () => {
  const headers = [];
  before(async function () {
    this.timeout(90000);
    await Promise.all(
      ['.context/chrome-selection', '.context/menu-position', '.context/ui-redesign'].map(
        (directory) => mkdir(directory, { recursive: true }),
      ),
    );
    const selection = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: false }),
    );
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').waitForEnabled();
    headers.push(await measureHeader('Setup'));
    await $('.welcome button.primary').click();
    await $('.selected-folder').waitForDisplayed();
    await $('.root-setup input[required]').setValue('Generated page chrome');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    // Preparation can replace/reorder the first tile while focus is being
    // measured. Wait for this fixture's real background work to settle.
    let preparation;
    try {
      await browser.waitUntil(
        async () => {
          preparation = (
            await browser.tauri.execute(({ core }) =>
              core.invoke('library_request', { request: { kind: 'status' } }),
            )
          ).data;
          return photoControlsReady(preparation);
        },
        { timeout: 45000, timeoutMsg: 'Generated page-chrome preparation did not complete' },
      );
    } finally {
      if (preparation) {
        await writeFile(
          '.context/chrome-selection/preparation.json',
          JSON.stringify(
            {
              ready: photoControlsReady(preparation),
              revision: preparation.revision,
              roots: preparation.roots.map(({ id, availability, preparingPreviews }) => ({
                id,
                availability,
                preparingPreviews,
              })),
              jobs: preparation.jobs.map(({ id, rootId, kind, state, errors }) => ({
                id,
                rootId,
                kind,
                state,
                errors,
              })),
            },
            null,
            2,
          ),
        );
      }
    }
  });
  it('reveals hover/focus selection and retains context actions without dots', async function () {
    this.timeout(120000);
    // Reproduce the CI precondition deliberately: controls must first reveal
    // under native hover, then fully hide when that actual pointer leaves.
    if (process.platform === 'win32') {
      await movePhotoPointer({ x: 20, y: 20 });
      await movePhotoPointer(
        await browser.execute(() => {
          const bounds = document.querySelector('.asset').getBoundingClientRect();
          return { x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2 };
        }),
      );
      await browser.waitUntil(() =>
        browser.execute(() => {
          const checkbox = document.querySelector('.asset-check');
          return (
            checkbox.closest('.asset').matches(':hover') &&
            getComputedStyle(checkbox).opacity === '1'
          );
        }),
      );
    }
    await restPhotoControls();
    const resting = await browser.execute(() => {
      const checkbox = document.querySelector('.asset-check');
      const resting = getComputedStyle(checkbox).opacity;
      checkbox.closest('.asset').querySelector('.asset-open').focus();
      return resting;
    });
    assert.equal(resting, '0');
    await browser.waitUntil(() =>
      browser.execute(
        () => getComputedStyle(document.querySelector('.asset-check')).opacity === '1',
      ),
    );
    assert.equal(await $('.photo-album-menu-button').isExisting(), false);
    assert.equal(await $('button[aria-label="Catalog"]').isExisting(), false);
    assert.equal(await $('button[aria-label="Saved plans"]').isExisting(), false);
    await photoContext();
    await browser.saveScreenshot('.context/menu-position/at-photo.png');
    await photoContext(true);
    await browser.saveScreenshot('.context/menu-position/at-edge.png');
    await photoContext();
    await $('[role="menuitem"][data-value="select-photo"]').click();
    await $('.asset-check:checked').waitForExist();
    await browser.setWindowSize(660, 560);
    headers.push(await measureHeader('Photos selecting 660'));
    const checkboxHit = await browser.execute(() => {
      const checkbox = document.querySelector('.asset-check:checked');
      const bounds = checkbox.getBoundingClientRect();
      return {
        visible: getComputedStyle(checkbox).opacity === '1',
        onTop:
          document.elementFromPoint(
            bounds.left + bounds.width / 2,
            bounds.top + bounds.height / 2,
          ) === checkbox,
      };
    });
    assert.deepEqual(checkboxHit, { visible: true, onTop: true });
    await browser.saveScreenshot('.context/chrome-selection/selection.png');
    await browser.setWindowSize(1180, 780);
    await $('button[aria-label="Done selecting"]').click();
    await restPhotoControls();
    assert.equal(
      await browser.execute(() => getComputedStyle(document.querySelector('.asset-check')).opacity),
      '0',
    );
    await $('button[aria-label="Select"]').click();
    await $('.asset-check', { strict: false }).waitForExist();
    await $('button[aria-label="Done selecting"]').click();
    await photoContext();
    await $('[role="menuitem"][data-value="add-to-album"]').click();
    await $('dialog[open][aria-label="Add to album"]').waitForDisplayed();
    await $('dialog[open]').$('button=Close').click();
  });
  it('aligns titles, search and actions across every destination at both window sizes', async function () {
    this.timeout(120000);
    await browser.keys('Escape');
    for (const width of [1180, 660]) {
      await browser.setWindowSize(width, 780);
      for (const label of [
        'Photos',
        'Albums / Tags',
        'Folders',
        'Map',
        'Duplicates',
        'Integrity',
        'Storage',
        'Backup',
        'Sync',
        'Structure Formater',
        'Settings',
        'Jobs',
      ]) {
        await $(`button[aria-label="${label}"]`).click();
        await browser.waitUntil(async () =>
          browser.execute(
            (label) =>
              [...document.querySelectorAll('.app-page-header')].some(
                (e) =>
                  e.getBoundingClientRect().height > 0 &&
                  e.querySelector('h1')?.textContent.trim() === label,
              ),
            label,
          ),
        );
        headers.push(await measureHeader(`${label} ${width}`));
        await browser.saveScreenshot(
          `.context/ui-redesign/${label.replaceAll(/[^a-zA-Z]/g, '-').toLowerCase()}-${width}.png`,
        );
        if (label === 'Albums / Tags') {
          await $('button[aria-label="Saved plans"]').click();
          await $('button=Close saved plans').waitForDisplayed();
          await $('button=Close saved plans').click();
          await $('button[aria-label="Catalog"]').click();
          await $('.catalog-organizer').waitForDisplayed();
          await $('dialog[open]').$('button=Close').click();
          await browser.saveScreenshot(`.context/chrome-selection/albums-${width}.png`);
        }
      }
      await $('button[aria-label="Back from Jobs"]').click();
    }
    const audit = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert(audit.verifiedFiles > 20);
    await writeFile(
      '.context/chrome-selection/receipt.json',
      JSON.stringify(
        {
          headers,
          audit,
          contextInput: 'DOM contextmenu dispatched inside the native WebView',
          nativePointerHoverAsserted: process.platform === 'win32',
          nativeContextMenuGestureAsserted: false,
        },
        null,
        2,
      ),
    );
  });
});
