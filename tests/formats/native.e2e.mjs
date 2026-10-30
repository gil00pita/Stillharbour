import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { browser, $, $$ } from '@wdio/globals';

function manifest(root) {
  return JSON.parse(
    execFileSync('python3', ['scripts/performance/manifest.py', root], {
      encoding: 'utf8',
      maxBuffer: 2 * 1024 * 1024,
    }),
  );
}
describe('native common-format previews — generated sources only', () => {
  it('opens a full-size HEIC original through the native gallery/viewer without changing sources', async () => {
    assert.equal(process.platform, 'darwin');
    await $('button.primary').waitForEnabled();
    const selection = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: false, nativeFormats: true }),
    );
    const { fixtureRoot } = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_performance', { action: 'fixture_location', rows: null }),
    );
    const output = `.context/formats/native-${randomUUID()}`;
    await mkdir('.context/formats', { recursive: true });
    await mkdir('.context/screenshots', { recursive: true });
    await mkdir(output, { recursive: false });
    const before = manifest(fixtureRoot);
    await writeFile(`${output}/manifest-before.json`, JSON.stringify(before));
    try {
      const picker = await browser.tauri.mock('foundation_request');
      try {
        await picker.mockReturnValue({ kind: 'directory', data: selection });
        await $('button.primary').click();
        await $('.root-setup input[required]').setValue('Generated native formats');
        await $('.folder-actions .primary').click();
      } finally {
        await picker.mockRestore();
      }
      await $('.asset-viewport').waitForDisplayed();
      await $('input[aria-label="Search filenames"]').setValue('Native HEIC');
      await browser.waitUntil(
        async () =>
          browser.execute(
            () =>
              document.querySelectorAll('.asset-open').length === 1 &&
              document
                .querySelector('.asset-open')
                ?.getAttribute('aria-label')
                ?.includes('Native HEIC') &&
              [...document.querySelectorAll('.asset img')].some(
                (image) => image.complete && image.naturalWidth > 0,
              ),
          ),
        { timeout: 45000 },
      );
      assert.equal((await $$('.asset-open')).length, 1);
      try {
        await browser.waitUntil(
          async () =>
            browser.execute(() => {
              const tiny = document.querySelector('.asset .t-skel-skeleton img');
              const clear = document.querySelector('.asset img.t-skel-content');
              return (
                tiny?.complete &&
                tiny.naturalWidth > 0 &&
                tiny.naturalWidth <= 32 &&
                clear?.complete &&
                clear.naturalWidth > 0 &&
                clear.naturalWidth <= 1024 &&
                Boolean(document.querySelector('.asset .t-skel.is-revealed'))
              );
            }),
          { timeout: 45000, timeoutMsg: 'Native tiny-to-clear layers did not finish loading' },
        );
      } catch (error) {
        const layers = await browser.execute(() => ({
          html: document.querySelector('.asset')?.outerHTML,
          images: [...document.querySelectorAll('.asset img')].map((image) => ({
            complete: image.complete,
            width: image.naturalWidth,
            height: image.naturalHeight,
            src: image.src,
          })),
          errors: [...document.querySelectorAll('.preview-error')].map((node) => node.textContent),
        }));
        await writeFile(`${output}/progressive-failure.json`, JSON.stringify(layers));
        await browser.saveScreenshot(`${output}/progressive-failure.png`);
        throw error;
      }
      await browser.waitUntil(
        async () => {
          const status = await browser.tauri.execute(async ({ core }) =>
            core.invoke('library_request', { request: { kind: 'status' } }),
          );
          return status.data.jobs.every(
            (job) => !['queued', 'running', 'cancel_requested'].includes(job.state),
          );
        },
        { timeout: 45000 },
      );
      const beforeJobs = await browser.tauri.execute(async ({ core }) =>
        core.invoke('library_request', { request: { kind: 'status' } }),
      );
      const previousIds = beforeJobs.data.jobs.map((job) => job.id);
      await $('button[aria-label="Jobs"]').click();
      await $('button=Generate thumbnails').click();
      await browser.waitUntil(
        async () => {
          const status = await browser.tauri.execute(async ({ core }) =>
            core.invoke('library_request', { request: { kind: 'status' } }),
          );
          return status.data.jobs.some(
            (job) =>
              job.kind === 'thumbnails' &&
              !previousIds.includes(job.id) &&
              ['completed', 'partial'].includes(job.state),
          );
        },
        { timeout: 45000, timeoutMsg: 'Separate native thumbnail backfill job did not finish' },
      );
      await browser.keys('Escape');

      await browser.saveScreenshot('.context/screenshots/native-heic-original-gallery.png');
      await $('.asset-open').click();
      await $('.photo-viewer[open]').waitForDisplayed();
      await browser.waitUntil(
        async () =>
          browser.execute(() => {
            const image = document.querySelector('.original-image');
            return (
              document.querySelector('.original-photo.has-original') &&
              document.querySelector('.photo-viewer')?.getAttribute('data-motion') === 'open' &&
              image?.complete &&
              image.naturalWidth === 3072 &&
              image.naturalHeight === 2048 &&
              image.src.startsWith('original://') &&
              getComputedStyle(image).opacity === '1' &&
              !document.querySelector('.original-notice')
            );
          }),
        {
          timeout: 45000,
          timeoutMsg: 'HEIC original must load without the preview-fallback warning',
        },
      );
      await browser.execute(async () => {
        await document.querySelector('.original-image').decode();
        await new Promise((resolve) => {
          const timeout = setTimeout(resolve, 1000);
          requestAnimationFrame(() =>
            requestAnimationFrame(() => {
              clearTimeout(timeout);
              resolve();
            }),
          );
        });
      });
      const geometry = await browser.execute(() => {
        const image = document.querySelector('.original-image');
        const fallback = document.querySelector('.original-fallback');
        return {
          source: image.src,
          naturalWidth: image.naturalWidth,
          naturalHeight: image.naturalHeight,
          imageHeight: image.getBoundingClientRect().height,
          imageWidth: image.getBoundingClientRect().width,
          opacity: getComputedStyle(image).opacity,
          fallbackHidden: getComputedStyle(fallback).visibility === 'hidden',
          frameOpacity: getComputedStyle(image.closest('.viewer-image-frame')).opacity,
          frameTransform: getComputedStyle(image.closest('.viewer-image-frame')).transform,
          visibility: document.visibilityState,
        };
      });
      assert.ok(geometry.imageHeight > 100 && geometry.imageWidth > 100);
      assert.equal(geometry.fallbackHidden, true);
      await writeFile(`${output}/viewer-geometry.json`, JSON.stringify(geometry));
      await browser.saveScreenshot('.context/screenshots/native-heic-original.png');
      await browser.keys('Escape');
      await $('.asset-viewport').waitForDisplayed();
      assert.equal(await $('input[aria-label="Search filenames"]').getValue(), 'Native HEIC');
      const verified = await browser.tauri.execute(async ({ core }) =>
        core.invoke('test_library_fixture', { verify: true }),
      );
      assert.equal(verified.verifiedFiles, 36);
      const foundation = await browser.tauri.execute(async ({ core }) =>
        core.invoke('foundation_request', { request: { kind: 'status' } }),
      );
      assert.equal(foundation.data.originalMutationsEnabled, false);
      await writeFile(
        `${output}/result.json`,
        JSON.stringify({
          platform: 'macOS WKWebView debug',
          format: 'generated HEIC',
          gallery: true,
          viewer: true,
          original: true,
          originalDimensions: [3072, 2048],
          sourceEntries: verified.verifiedFiles,
          originalMutationsEnabled: foundation.data.originalMutationsEnabled,
        }),
      );
    } finally {
      const after = manifest(fixtureRoot);
      await writeFile(`${output}/manifest-after.json`, JSON.stringify(after));
      assert.deepEqual(after, before);
    }
  });
});
