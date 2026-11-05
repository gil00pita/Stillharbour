import assert from 'node:assert/strict';
import { browser, $, $$ } from '@wdio/globals';
import { readFile } from 'node:fs/promises';

export async function verifyNasaSatellite(directory) {
  const { tileBase } = JSON.parse(await readFile('assets/maps/satellite-provider.json', 'utf8'));
  await $('button[aria-label="Map"]').click();
  await $('.map-basemap canvas').waitForDisplayed();
  await browser.waitUntil(async () => $('.map-marker').isEnabled());
  const globe = $('button[aria-label="Whole world"]');
  assert.equal(await globe.getText(), '');
  await browser.execute((base) => {
    window.__satelliteCanvas = document.querySelector('.map-basemap canvas');
    window.__satelliteMarker = document.querySelector('.map-marker');
    window.__nasaRequests = [];
    window.__nasaOffline = false;
    window.__nasaOriginalFetch = window.fetch;
    window.fetch = async (input, init) => {
      const request = new Request(input, init);
      if (!request.url.startsWith(base)) return window.__nasaOriginalFetch(input, init);
      const record = {
        url: request.url,
        method: request.method,
        credentials: request.credentials,
        cache: request.cache,
        referrerPolicy: request.referrerPolicy,
      };
      window.__nasaRequests.push(record);
      if (window.__nasaOffline) throw new TypeError('Generated satellite network failure');
      const result = await window.__nasaOriginalFetch(input, init);
      record.status = result.status;
      return result;
    };
  }, tileBase);
  try {
    assert.equal(await browser.execute(() => window.__nasaRequests.length), 0);
    const beforePhotos = (await $$('.map-filmstrip li')).length;
    await $('button=Satellite').click();
    await browser.waitUntil(
      async () => (await $('.map-view-note').getText()).startsWith('NASA satellite overview'),
      { timeout: 30000 },
    );
    const requests = await browser.execute(() => window.__nasaRequests);
    assert.ok(requests.some((request) => request.status === 200));
    assert.ok(
      requests.every(
        (request) =>
          request.url.startsWith(tileBase) &&
          request.method === 'GET' &&
          request.credentials === 'same-origin' &&
          request.cache === 'no-store' &&
          request.referrerPolicy === 'no-referrer',
      ),
    );
    assert.equal(
      await browser.execute(
        () =>
          window.__satelliteCanvas === document.querySelector('.map-basemap canvas') &&
          window.__satelliteMarker === document.querySelector('.map-marker'),
      ),
      true,
    );
    assert.equal((await $$('.map-filmstrip li')).length, beforePhotos);
    await browser.saveScreenshot(`${directory}/satellite.png`);
    await $('button[aria-label="Whole world"]').click();
    assert.equal(await $('button=Satellite').getAttribute('aria-pressed'), 'true');
    await browser.waitUntil(async () => !(await $('button[aria-label="Zoom out"]').isEnabled()));
    await browser.waitUntil(
      () =>
        browser.execute(
          (base) =>
            window.__nasaRequests.some(
              (request) =>
                (request.url.startsWith(`${base}0/`) ||
                  request.url.startsWith(`${base}1/`) ||
                  request.url.startsWith(`${base}2/`)) &&
                request.status === 200,
            ),
          tileBase,
        ),
      { timeout: 30000 },
    );
    await browser.waitUntil(
      async () => (await $('.map-view-note').getText()).startsWith('NASA satellite overview'),
      { timeout: 30000 },
    );
    await browser.saveScreenshot(`${directory}/satellite-world.png`);
    await browser.setWindowSize(660, 560);
    assert.equal(
      await browser.execute(() => {
        const buttons = [
          ...document.querySelectorAll('.map-world, .map-layers button, .map-zoom button'),
        ];
        return buttons.every((button) => {
          const bounds = button.getBoundingClientRect();
          return (
            bounds.left >= 0 &&
            bounds.right <= innerWidth &&
            bounds.top >= 0 &&
            bounds.bottom <= innerHeight
          );
        });
      }),
      true,
    );
    await browser.saveScreenshot(`${directory}/satellite-minimum.png`);
    await browser.setWindowSize(1180, 780);
    await $('.map-layers').$('button=Map').click();
    assert.equal(await $('.map-layers').$('button=Map').getAttribute('aria-pressed'), 'true');
    assert.ok((await $('.map-attribution').getText()).includes('Natural Earth'));
    const requestCount = await browser.execute(() => window.__nasaRequests.length);
    await $('button[aria-label="Zoom in"]').click();
    await browser.waitUntil(async () => $('.map-marker').isEnabled());
    assert.equal(await browser.execute(() => window.__nasaRequests.length), requestCount);
    await browser.execute(() => {
      window.__nasaOffline = true;
    });
    await $('button=Satellite').click();
    await browser.waitUntil(async () =>
      (await $('.map-view-note').getText()).includes('could not be loaded'),
    );
    assert.equal(await $('.map-layers').$('button=Map').getAttribute('aria-pressed'), 'true');
    assert.equal(await $('.map-basemap canvas').isDisplayed(), true);
    await browser.execute(() => {
      window.__nasaOffline = false;
    });
    await $('button=Satellite').click();
    await browser.waitUntil(
      async () => (await $('.map-view-note').getText()).startsWith('NASA satellite overview'),
      { timeout: 30000 },
    );
    await $('.map-layers').$('button=Map').click();
    return {
      provider: 'NASA GIBS Blue Marble',
      liveTileRequests: requests,
      canvasRetained: true,
      markersRetained: true,
      minimumControlsReachable: true,
      offlineFallbackAndRetry: true,
      networkEvidenceScope: 'Instrumented main-WebView fetch calls; not an OS traffic interception',
    };
  } finally {
    await browser.execute(() => {
      window.fetch = window.__nasaOriginalFetch;
    });
  }
}
