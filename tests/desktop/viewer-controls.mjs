import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';

export async function assertViewerControlsOverlay(observations = {}) {
  const notification = $('[data-scope="toast"][data-part="root"]');
  if (await notification.isExisting()) {
    await notification.waitForDisplayed();
    const message = await notification.$('[data-part="description"]').getText();
    // Malformed/unavailable generated previews can leave a persistent error
    // over the bottom toolbar. Acknowledge only that known fixture outcome;
    // unrelated errors must remain visible and fail this scenario.
    assert.equal(
      message,
      'The requested preview asset is unavailable.',
      'Unexpected application error before viewer control checks',
    );
    await notification.$('button[aria-label="Dismiss error"]').click();
    await notification.waitForExist({ reverse: true });
    observations.viewerPreviewErrorsAcknowledged ??= [];
    observations.viewerPreviewErrorsAcknowledged.push(message);
  }
  const before = await browser.execute(() => {
    const viewer = document.querySelector('.photo-viewer');
    const canvas = viewer.querySelector('.viewer-canvas').getBoundingClientRect();
    const frame = viewer.querySelector('.viewer-image-frame').getBoundingClientRect();
    return {
      motion: viewer.dataset.motion,
      canvas: { width: canvas.width, height: canvas.height },
      frame: { width: frame.width, height: frame.height },
    };
  });
  observations.viewerReadiness ??= [];
  observations.viewerReadiness.push(before);
  await browser.waitUntil(
    async () =>
      browser.execute(() => {
        const viewer = document.querySelector('.photo-viewer');
        const canvas = viewer.querySelector('.viewer-canvas').getBoundingClientRect();
        const frame = viewer.querySelector('.viewer-image-frame').getBoundingClientRect();
        return (
          viewer.dataset.motion === 'open' &&
          frame.width > 0 &&
          frame.height > 0 &&
          frame.left >= canvas.left - 1 &&
          frame.right <= canvas.right + 1 &&
          frame.top >= canvas.top - 1 &&
          frame.bottom <= canvas.bottom + 1
        );
      }),
    { timeoutMsg: 'Viewer opening and fitted image geometry did not settle' },
  );
  const geometry = await browser.execute(() => {
    const controls = document.querySelector('.viewer-controls');
    const bar = controls.getBoundingClientRect();
    const photo = document.querySelector('.viewer-photo').getBoundingClientRect();
    const heart = controls.querySelector('.viewer-favorite');
    const buttons = [...controls.querySelectorAll('button')];
    const first = buttons[0].getBoundingClientRect();
    function target(button) {
      const bounds = button.getBoundingClientRect();
      const hit = document.elementFromPoint(
        bounds.left + bounds.width / 2,
        bounds.top + bounds.height / 2,
      );
      return {
        label: button.getAttribute('aria-label'),
        disabled: button.disabled,
        left: bounds.left,
        top: bounds.top,
        width: bounds.width,
        height: bounds.height,
        reachable: button.contains(hit),
        hit: hit && {
          tag: hit.tagName,
          className: hit.getAttribute('class'),
          control: hit.closest('button')?.getAttribute('aria-label') ?? null,
        },
      };
    }
    const buttonBounds = buttons.map(target);
    const navigationBounds = [...document.querySelectorAll('.viewer-previous, .viewer-next')].map(
      target,
    );
    return {
      insidePhoto:
        bar.left >= photo.left &&
        bar.right <= photo.right + 1 &&
        bar.top >= photo.top &&
        bar.bottom <= photo.bottom,
      overlaysCanvas:
        getComputedStyle(controls.parentElement).position === 'absolute' &&
        controls.parentElement.parentElement.classList.contains('viewer-photo'),
      heartBeforeInfo: heart.nextElementSibling?.getAttribute('aria-label') === 'Info',
      singleRow: buttons.every((button) => {
        const bounds = button.getBoundingClientRect();
        return Math.abs(bounds.top + bounds.height / 2 - first.top - first.height / 2) < 1;
      }),
      buttonsReachable: buttonBounds.every((button) => button.reachable),
      navigationReachable: navigationBounds.every((button) => button.reachable),
      buttonBounds,
      navigationBounds,
    };
  });
  const { buttonBounds, navigationBounds, ...checks } = geometry;
  try {
    assert.deepEqual(
      checks,
      {
        insidePhoto: true,
        overlaysCanvas: true,
        heartBeforeInfo: true,
        singleRow: true,
        buttonsReachable: true,
        navigationReachable: true,
      },
      JSON.stringify({ buttonBounds, navigationBounds }),
    );
  } catch (error) {
    await writeFile(
      '.context/screenshots/viewer-controls-failure.json',
      JSON.stringify({ before, ...geometry }, null, 2),
    );
    await browser.saveScreenshot('.context/screenshots/viewer-controls-failure.png');
    throw error;
  }
}
