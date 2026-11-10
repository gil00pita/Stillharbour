import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';

async function activateWithFocus(selector) {
  // Match native pointer order; the embedded click adapter focuses after dispatch.
  // This is the same focus-first activation used by the catalog navigation smoke.
  await browser.execute((selector) => {
    const button = document.querySelector(selector);
    button.focus();
    button.click();
  }, selector);
}
async function back() {
  await activateWithFocus('button[aria-label="Back from Jobs"]');
  await browser.waitUntil(async () => $('button[aria-label="Jobs"]').isFocused());
}

describe('Jobs page — generated native fixture', () => {
  it('opens a full page before registration and retains gallery state, counts and native history', async function () {
    this.timeout(90000);
    await $('.welcome button.primary').waitForEnabled();
    await activateWithFocus('button[aria-label="Jobs"]');
    await $('.jobs-page').waitForDisplayed();
    await browser.waitUntil(async () => $('.jobs-page h1').isFocused());
    assert.equal(await $('.jobs-page h1').getText(), 'Jobs');
    await browser.keys('Escape');
    assert.equal(await $('.jobs-page').isDisplayed(), true);
    await back();
    await $('.welcome').waitForDisplayed();

    const selection = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: false }),
    );
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').click();
    await $('.selected-folder').waitForDisplayed();
    await $('.root-setup input[required]').setValue('Generated Jobs page');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    await $('button[aria-label="Select"]').click();
    await $('.asset-check', { strict: false }).waitForEnabled();
    await activateWithFocus('.asset-check');
    const before = await browser.execute(() => {
      const viewport = document.querySelector('.asset-viewport');
      viewport.scrollTop = Math.min(120, viewport.scrollHeight - viewport.clientHeight);
      return {
        top: viewport.scrollTop,
        selected: [...document.querySelectorAll('.asset-check:checked')].map(
          (input) => input.closest('[data-asset-id]').dataset.assetId,
        ),
      };
    });
    await $('button[aria-label="Jobs"]').click();
    await $('.jobs-page').waitForDisplayed();
    assert.equal(await $('button[aria-label="Jobs"]').getAttribute('aria-current'), 'page');
    assert.equal(await $('button[aria-label="Jobs"]').getAttribute('aria-haspopup'), null);
    assert.equal(
      await browser.execute(() => {
        const page = document.querySelector('.jobs-page');
        const workspace = document.querySelector('.workspace-page');
        return (
          Boolean(page.closest('main')) &&
          !page.closest('dialog') &&
          workspace.hidden &&
          workspace.inert
        );
      }),
      true,
    );
    await browser.waitUntil(async () =>
      browser.tauri.execute(async ({ core }) => {
        const status = await core.invoke('library_request', { request: { kind: 'status' } });
        return (
          status.data.jobs.length > 0 &&
          status.data.jobs.every(
            (job) =>
              !['queued', 'running', 'cancel_requested', 'pause_requested', 'paused'].includes(
                job.state,
              ),
          )
        );
      }),
    );
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const button = document.querySelector('button[aria-label="Jobs"]');
        return (
          document.getElementById(button.getAttribute('aria-describedby')).textContent ===
          '0 running jobs'
        );
      }),
    );
    const recent = await $('.jobs-page .recent-jobs');
    await recent.waitForExist();
    assert.equal(await recent.getAttribute('open'), null);
    await recent.$('summary').click();
    await $('.jobs-page .job-row[data-job-kind="scan"]').waitForDisplayed();
    await (await $('.jobs-page .job-row[data-job-kind="scan"]')).$('button=Details').click();
    await $('.job-details').waitForDisplayed();
    for (const [label, width, height] of [
      ['desktop', 1180, 780],
      ['minimum', 660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      assert.equal(
        await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
        true,
      );
      await browser.saveScreenshot(`.context/screenshots/jobs-page-${label}.png`);
    }
    await browser.setWindowSize(1180, 780);
    await back();
    await $('.asset-viewport').waitForDisplayed();
    await browser.waitUntil(async () =>
      browser.execute(
        (before) => document.querySelector('.asset-viewport').scrollTop === before.top,
        before,
      ),
    );
    assert.deepEqual(
      await browser.execute(() =>
        [...document.querySelectorAll('.asset-check:checked')].map(
          (input) => input.closest('[data-asset-id]').dataset.assetId,
        ),
      ),
      before.selected,
    );
    const verified = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert(verified.verifiedFiles > 20);
  });
});
