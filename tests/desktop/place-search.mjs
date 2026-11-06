import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { browser, $, $$ } from '@wdio/globals';
import { native, query } from './m5-metadata-helpers.mjs';

export async function verifyOfflinePlaceSearch(directory) {
  const cityId = 'places-v1:city:2643743';
  const countryId = 'places-v1:country:GBR';
  const search = 'input[placeholder="Search filenames, dates or places…"]';
  const trigger = search;
  await $(trigger).click();
  await $(search).waitForDisplayed();
  await $(search).setValue('London');
  await $('button*=London, England, United Kingdom').waitForDisplayed();
  assert.ok(await $('button*=London, Ontario, Canada').isExisting());
  await browser.saveScreenshot(`${directory}/place-choices.png`);
  await $('button*=London, England, United Kingdom').click();
  await browser.waitUntil(async () => $('.map-marker').isEnabled());
  await $('.map-marker').click();
  await browser.waitUntil(
    async () =>
      (await $$('.map-filmstrip li')).length === 1 &&
      (await $('.map-filmstrip li button').getText()).includes('London.jpg'),
  );
  const city = (
    await native({ kind: 'query', query: { ...query, placeId: cityId }, cursor: null, limit: 80 })
  ).data;
  assert.equal(city.total, 1);
  await $('button=Show in library').click();
  await $('button=Return to map').waitForDisplayed();
  await $('button=Clear location filter').click();
  await browser.waitUntil(
    async () =>
      (await $$('.gallery .asset')).length === 1 &&
      (await $('button[aria-label="Open London.jpg"]').isExisting()),
  );
  await browser.saveScreenshot(`${directory}/place-photos.png`);
  await browser.setWindowSize(660, 560);
  await $(trigger).click();
  await $(search).waitForDisplayed();
  await $('p=Selected: London, England, United Kingdom').waitForDisplayed();
  assert.equal(
    await browser.execute(() => {
      const content = document.querySelector('[data-scope="popover"][data-part="content"]');
      const bounds = content.getBoundingClientRect();
      return (
        document.documentElement.scrollWidth <= window.innerWidth &&
        bounds.width <= window.innerWidth &&
        bounds.height <= window.innerHeight
      );
    }),
    true,
  );
  assert.equal(
    await browser.execute(() =>
      [...document.querySelectorAll('.photo-toolbar button')]
        .filter((button) => button.getBoundingClientRect().width > 0)
        .every((button) => {
          const bounds = button.getBoundingClientRect();
          return (
            bounds.left >= 0 &&
            bounds.right <= innerWidth &&
            bounds.top >= 0 &&
            bounds.bottom <= innerHeight
          );
        }),
    ),
    true,
    'Every toolbar action must remain reachable at the minimum window width',
  );
  await browser.saveScreenshot(`${directory}/place-minimum.png`);
  await $('button=Close place search').click();
  await browser.setWindowSize(1180, 780);
  await $(trigger).click();
  await $(search).setValue('United Kingdom');
  const country = $('//button[.//p[normalize-space(.)="United Kingdom"]]');
  await country.waitForDisplayed();
  await country.click();
  await browser.waitUntil(async () => (await $$('.gallery .asset')).length === 1);
  const frozen = (
    await native({ kind: 'freeze_selection', query: { ...query, placeId: countryId } })
  ).data;
  assert.equal(frozen.count, 1);
  await $(trigger).click();
  await $(search).setValue('London');
  await $('button*=London, Ontario, Canada').waitForDisplayed();
  await $('button*=London, Ontario, Canada').click();
  await browser.waitUntil(
    async () =>
      (await $$('.gallery .asset')).length === 0 &&
      (await $('.empty-results h2').getText()) === 'No matching photos yet',
  );
  await $(trigger).click();
  await $('button=Clear place filter').waitForDisplayed();
  await $('button=Clear place filter').click();
  await browser.waitUntil(async () => (await $$('.gallery .asset')).length > 1);
  // Re-enter Photos through the real map flow so the existing metadata test
  // retains its geographic saved-search scope after our independent clearing test.
  await $('button[aria-label="Map"]').click();
  await $(trigger).click();
  await $('button=Clear place filter').waitForDisplayed();
  await $('button=Clear place filter').click();
  await $('button=Show photos in view').click();
  await browser.waitUntil(async () => $('button=Show in library').isEnabled());
  await $('button=Show in library').click();
  await browser.waitUntil(async () => (await $$('.gallery .asset')).length === 1);
  // Restore one explicit selection for the metadata-review scenario.
  const restoredPhotos = await $$('.asset-check');
  assert.equal(restoredPhotos.length, 1);
  await restoredPhotos[0].waitForEnabled();
  if (!(await restoredPhotos[0].isSelected())) await restoredPhotos[0].click();
  const external = await browser.execute(() =>
    performance
      .getEntriesByType('resource')
      .map((entry) => entry.name)
      .filter(
        (url) =>
          /^https?:/.test(url) &&
          ![
            'tauri.localhost',
            'thumbnail.localhost',
            'original.localhost',
            'ipc.localhost',
            '127.0.0.1',
            'localhost',
          ].includes(new URL(url).hostname),
      ),
  );
  assert.deepEqual(external, []);
  const result = {
    cityId,
    countryId,
    cityCount: city.total,
    countrySelectionCount: frozen.count,
    sameNameCityExcluded: true,
    restoredCollection: true,
    externalResources: external,
    networkEvidenceScope: 'Main WebView resource timing only; not OS/worker interception',
  };
  await writeFile(`${directory}/place-search.json`, JSON.stringify(result, null, 2) + '\n');
  return result;
}
