import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $, $$ } from '@wdio/globals';
import { verifyOfflinePlaceSearch } from './place-search.mjs';
import { verifyNasaSatellite } from './map-satellite.mjs';

import {
  native,
  world,
  query,
  reviewCatalogMetadata,
  finishMetadataExports,
} from './m5-metadata-helpers.mjs';

describe('M5 generated map and catalog metadata desktop slice', () => {
  it('browses offline GPS and saves catalog metadata without changing generated originals', async function () {
    this.timeout(120000);
    const directory = '.context/m5-native';
    await mkdir(directory, { recursive: true });
    try {
      const selection = await browser.tauri.execute(({ core }) =>
        core.invoke('test_library_fixture', { verify: false, geographic: true }),
      );
      await native({
        kind: 'save_and_scan',
        selection_id: selection.id,
        label: 'Generated map library',
        mode: 'referenced',
        filter: {
          version: 1,
          includeGlobs: [],
          excludeGlobs: [],
          excludedFolders: [],
          extensions: ['jpg'],
          includeHiddenFiles: false,
          includeHiddenFolders: false,
          minBytes: null,
          maxBytes: null,
          fromDate: null,
          throughDate: null,
        },
      });
      await browser.waitUntil(
        async () => {
          const status = (await native({ kind: 'status' })).data;
          return (
            status.jobs.length > 0 &&
            status.jobs.every((job) => ['completed', 'partial'].includes(job.state))
          );
        },
        { timeout: 45000, interval: 200 },
      );
      const nativeMap = (await native({ kind: 'map_view', query, bounds: world, zoom: 4 })).data;
      if (nativeMap.located !== 6) {
        const page = (await native({ kind: 'query', query, cursor: null, limit: 80 })).data;
        const ids = page.rows
          ? page.rows.map((row) => row[0])
          : page.assets.map((asset) => asset.id);
        const details = await Promise.all(
          ids.map(async (id) => (await native({ kind: 'detail', asset_id: id })).data),
        );
        await writeFile(
          `${directory}/metadata-failure.json`,
          JSON.stringify({ nativeMap, page, details }, null, 2),
        );
      }
      assert.equal(nativeMap.located, 6);
      assert.equal(nativeMap.unavailable, 2);
      assert.equal(
        nativeMap.clusters.reduce((sum, cluster) => sum + cluster.count, 0),
        6,
      );
      const crossing = (
        await native({
          kind: 'map_view',
          query,
          bounds: { west: 170, east: -170, south: 0, north: 10 },
          zoom: 16,
        })
      ).data;
      assert.equal(crossing.inView, 2);
      const zero = (
        await native({
          kind: 'map_view',
          query,
          bounds: { west: 0, east: 0, south: 0, north: 0 },
          zoom: 16,
        })
      ).data;
      assert.equal(zero.inView, 2);
      assert.equal(zero.clusters.length, 1);
      const west = nativeMap.clusters.find(
        (cluster) => cluster.representative.name === 'South.jpg',
      );
      assert.ok(west);
      assert.ok(
        west.latitude < 0 && west.longitude < 0,
        'EXIF reference signs must survive extraction',
      );
      await $('button[aria-label="Map"]').waitForDisplayed();
      await browser.execute(() => {
        window.__mapRaf = false;
        requestAnimationFrame(() => {
          window.__mapRaf = true;
        });
        window.__mapErrors = [];
        window.__mapWorkers = [];
        const OriginalWorker = window.Worker;
        window.Worker = class extends OriginalWorker {
          constructor(url, options) {
            super(url, options);
            const report = { url: String(url), options, messages: 0, errors: [] };
            window.__mapWorkers.push(report);
            this.addEventListener('message', (event) => {
              report.messages++;
              report.lastMessage = JSON.stringify(event.data).slice(0, 800);
            });
            this.addEventListener('error', (event) => report.errors.push(event.message));
          }
        };
        window.addEventListener('error', (event) => window.__mapErrors.push(event.message));
        window.addEventListener('unhandledrejection', (event) =>
          window.__mapErrors.push(String(event.reason)),
        );
        document.addEventListener('securitypolicyviolation', (event) =>
          window.__mapErrors.push(`${event.violatedDirective}: ${event.blockedURI}`),
        );
      });
      await $('button[aria-label="Map"]').click();
      await $('.map-marker').waitForDisplayed({ timeout: 30000 });
      await browser.waitUntil(async () => $('.map-marker').isEnabled());
      assert.equal((await $$('.map-filmstrip li')).length, 0);
      await browser.execute(() => {
        window.__mapCanvas = document.querySelector('.map-basemap canvas');
      });
      await browser.waitUntil(() =>
        browser.execute(() =>
          [...document.querySelectorAll('.map-marker img')].some(
            (image) => image.complete && image.naturalWidth > 0,
          ),
        ),
      );
      const markerGeometry = await browser.execute(() =>
        [...document.querySelectorAll('.map-marker img')].map((image) => {
          const frame = image.closest('.map-marker-image').getBoundingClientRect();
          const photo = image.getBoundingClientRect();
          const button = image.closest('button').getBoundingClientRect();
          return {
            width: button.width,
            height: button.height,
            contained:
              photo.left >= frame.left - 0.5 &&
              photo.right <= frame.right + 0.5 &&
              photo.top >= frame.top - 0.5 &&
              photo.bottom <= frame.bottom + 0.5,
            clipping: getComputedStyle(image.parentElement).overflow,
          };
        }),
      );
      assert.ok(markerGeometry.length > 0);
      assert.ok(
        markerGeometry.every(
          (marker) =>
            marker.width === 48 &&
            marker.height === 48 &&
            marker.contained &&
            marker.clipping === 'hidden',
        ),
      );
      assert.equal(await $('.map-filter-trigger').getAttribute('aria-expanded'), 'false');
      const searchGeometry = await browser.execute(() => {
        const input = document.querySelector('.map-search-controls input[type="search"]');
        const frame = document.querySelector('.map-search-controls .search-input-frame');
        const icon = frame.querySelector('svg');
        const filter = document.querySelector('.map-filter-trigger');
        const field = frame.getBoundingClientRect();
        const text = input.getBoundingClientRect();
        const glyph = icon.getBoundingClientRect();
        const trigger = filter.getBoundingClientRect();
        return {
          iconInside:
            glyph.width > 0 &&
            glyph.left > field.left &&
            glyph.right < field.right &&
            glyph.top > field.top &&
            glyph.bottom < field.bottom,
          iconBeforeInput: glyph.right <= text.left,
          inputReachable:
            document.elementFromPoint(text.left + text.width / 2, text.top + text.height / 2) ===
            input,
          filterBesideSearch: trigger.left >= field.right && Math.abs(trigger.top - field.top) < 2,
        };
      });
      assert.deepEqual(searchGeometry, {
        iconInside: true,
        iconBeforeInput: true,
        inputReachable: true,
        filterBesideSearch: true,
      });
      await $('.map-filter-trigger').click();
      const filtersSelector = '[role="dialog"][aria-label="Map filters"]';
      await $(filtersSelector).waitForDisplayed();
      await browser.saveScreenshot(`${directory}/toolbar-filters.png`);
      await $('button[aria-label="Close filters"]').click();
      await browser.waitUntil(
        async () => (await $('.map-filter-trigger').getAttribute('aria-expanded')) === 'false',
      );
      assert.equal(
        await browser.execute(
          () => document.activeElement === document.querySelector('.map-filter-trigger'),
        ),
        true,
      );
      await browser.setWindowSize(660, 560);
      await $('.map-filter-trigger').click();
      await $(filtersSelector).waitForDisplayed();
      assert.equal(
        await browser.execute(() => {
          const panel = document
            .querySelector('[role="dialog"][aria-label="Map filters"]')
            .getBoundingClientRect();
          const input = document
            .querySelector('.map-search-controls .search-input-frame')
            .getBoundingClientRect();
          const filter = document.querySelector('.map-filter-trigger').getBoundingClientRect();
          return (
            panel.left >= 0 &&
            panel.right <= innerWidth &&
            panel.top >= 0 &&
            panel.bottom <= innerHeight &&
            input.width > 80 &&
            filter.left >= input.right &&
            filter.right <= innerWidth
          );
        }),
        true,
      );
      await browser.saveScreenshot(`${directory}/toolbar-filters-minimum.png`);
      // A native select can consume Escape before it reaches the popover.
      // Test the popover's keyboard dismissal from its ordinary close button.
      await browser.execute(() => document.querySelector('[aria-label="Close filters"]').focus());
      assert.equal(await $('button[aria-label="Close filters"]').isFocused(), true);
      await browser.keys('Escape');
      await browser.waitUntil(
        async () => (await $('.map-filter-trigger').getAttribute('aria-expanded')) === 'false',
      );
      await browser.setWindowSize(1180, 780);
      await browser.saveScreenshot(`${directory}/toolbar-collapsed.png`);
      assert.equal(await $('.map-basemap canvas').isDisplayed(), true);
      assert.equal(
        await $('.map-attribution').getText(),
        'Made with Natural Earth · Offline world overview',
      );
      const resources = await browser.execute(() =>
        performance.getEntriesByType('resource').map((entry) => entry.name),
      );
      const external = resources.filter(
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
      );
      assert.deepEqual(external, []);
      await browser.execute(() => document.querySelector('.map-basemap canvas').focus());
      await browser.keys('=');
      await browser.waitUntil(async () => {
        const total = await browser.execute(() =>
          [...document.querySelectorAll('.map-marker-count')].reduce(
            (sum, badge) => sum + Number(badge.textContent),
            0,
          ),
        );
        return (
          total > 0 &&
          total < 6 &&
          (await $('.map-marker').isEnabled()) &&
          (await $('button[aria-label="Zoom out"]').isEnabled())
        );
      });
      const keyboardZoomVerified = true;
      assert.equal((await $$('.map-filmstrip li')).length, 0);
      assert.equal(
        await browser.execute(
          () => window.__mapCanvas === document.querySelector('.map-basemap canvas'),
        ),
        true,
      );
      await $('button[aria-label="Whole world"]').click();
      await browser.waitUntil(async () => !(await $('button[aria-label="Zoom out"]').isEnabled()));
      await $('button[aria-label="Zoom in"]').click();
      await browser.waitUntil(async () => {
        const total = await browser.execute(() =>
          [...document.querySelectorAll('.map-marker-count')].reduce(
            (sum, badge) => sum + Number(badge.textContent),
            0,
          ),
        );
        return (
          total > 0 &&
          total < 6 &&
          (await $('.map-marker').isEnabled()) &&
          (await $('button[aria-label="Zoom out"]').isEnabled())
        );
      });
      await $('button[aria-label="Whole world"]').click();
      await browser.waitUntil(
        async () => (await $$('.map-marker')).length === 1 && (await $('.map-marker').isEnabled()),
      );
      await browser.saveScreenshot(`${directory}/overview.png`);
      await $('.map-marker').click();
      await browser.saveScreenshot(`${directory}/after-marker.png`);
      await browser.waitUntil(
        async () => (await $('.map-photo-heading h2').getText()) === 'Selected location',
      );
      await browser.waitUntil(async () => $('button=Show in photos').isEnabled());
      const selectedCount = Number((await $('.map-photo-heading p').getText()).match(/^(\d+)/)[1]);
      await $('button=Show in photos').click();
      await $('button=Return to map').waitForDisplayed();
      assert.ok(await $('button=Clear location filter').isExisting());
      await $('button=Return to map').click();
      await $('.map-page').waitForDisplayed();
      await $('.map-marker').waitForDisplayed();
      await $('button[aria-label="Map filters"]').click();
      await $('button=No usable location (2)').waitForDisplayed();
      await $('button=No usable location (2)').click();
      await browser.waitUntil(
        async () => (await $('.map-photo-heading h2').getText()) === 'No usable location',
      );
      await browser.waitUntil(async () =>
        (await $('.map-photo-heading p').getText()).startsWith('2 photos'),
      );
      assert.equal(await $('.map-geography').isExisting(), false);
      assert.equal(await $('.map-photo-heading p').getText(), '2 photos without usable location');
      await browser.setWindowSize(660, 560);
      await browser.saveScreenshot(`${directory}/minimum.png`);
      await browser.setWindowSize(1180, 780);
      const placeSearch = await verifyOfflinePlaceSearch(directory);
      const metadataEvidence = await reviewCatalogMetadata(directory);
      const exportEvidence = await finishMetadataExports(directory);
      const satelliteEvidence = await verifyNasaSatellite(directory);
      const audit = await browser.tauri.execute(({ core }) =>
        core.invoke('test_library_fixture', { verify: true }),
      );
      assert.equal(audit.verifiedFiles, 9);
      const foundation = await browser.tauri.execute(({ core }) =>
        core.invoke('foundation_request', { request: { kind: 'status' } }),
      );
      assert.equal(foundation.data.originalMutationsEnabled, false);
      await writeFile(
        `${directory}/map.json`,
        JSON.stringify(
          {
            platform: process.platform,
            nativeMap,
            crossing,
            zero,
            selectedCount,
            keyboardZoomVerified,
            searchGeometry,
            filterDropdownVerified: true,
            markerGeometry,
            photosRequireClick: true,
            canvasRetainedAcrossZoom: true,
            placeSearch,
            satelliteEvidence,
            metadataEvidence,
            exportEvidence,
            externalResources: external,
            resourceTimingScope:
              'main WebView only; worker and OS traffic not independently intercepted',
            audit,
            originalMutationsEnabled: false,
          },
          null,
          2,
        ),
      );
    } catch (error) {
      await browser.saveScreenshot(`${directory}/failure.png`);
      await writeFile(
        `${directory}/failure-ui.json`,
        JSON.stringify(
          {
            message: String(error),
            text: await $('main').getText(),
            diagnostics: await browser.execute(() => ({
              errors: window.__mapErrors,
              workers: window.__mapWorkers,
              raf: window.__mapRaf,
              visibility: document.visibilityState,
              map: document.querySelector('.map-basemap')?.innerHTML.slice(0, 10000),
              resources: performance
                .getEntriesByType('resource')
                .map((r) => ({ name: r.name, duration: r.duration, bytes: r.transferSize })),
            })),
          },
          null,
          2,
        ),
      );
      throw error;
    }
  });
});
