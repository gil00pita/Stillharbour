import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';

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
  });
  it('uses context actions without dots or browsing checkboxes', async function () {
    this.timeout(120000);
    assert.equal(await $('.asset-check').isExisting(), false);
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
    await $('button=Done selecting').click();
    assert.equal(await $('.asset-check').isExisting(), false);
    await $('button=Select').click();
    await $('.asset-check', { strict: false }).waitForExist();
    await $('button=Done selecting').click();
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
          contextInput: 'DOM contextmenu dispatched inside native WKWebView',
          nativePointerGestureAsserted: false,
        },
        null,
        2,
      ),
    );
  });
});
