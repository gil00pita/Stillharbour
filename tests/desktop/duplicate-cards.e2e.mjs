import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';

// Own disposable native catalog and generated media; never a user's photo library.
describe('Illustrated duplicate matching cards', () => {
  it('selects cards at desktop/minimum sizes and inspects unchanged fixtures', async function () {
    this.timeout(60000);
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
    await $('.root-setup input[required]').setValue('Generated duplicate card smoke');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    await $('button[aria-label="Duplicates"]').click();
    await $('.duplicate-match-options').waitForDisplayed();
    for (const [width, height] of [
      [1440, 1000],
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      const metrics = await browser.execute(() => {
        const cards = [
          ...document.querySelectorAll('.duplicate-match-options .inspection-choice-card'),
        ].slice(0, 3);
        return {
          overflow: document.documentElement.scrollWidth > innerWidth,
          images: cards.map((card) => {
            const image = card.querySelector('img');
            return image.complete && image.naturalWidth > 0 && image.alt === '';
          }),
          bounds: cards.map((card) => ({
            x: card.getBoundingClientRect().x,
            y: card.getBoundingClientRect().y,
            width: card.clientWidth,
          })),
        };
      });
      assert.equal(metrics.overflow, false);
      assert.deepEqual(metrics.images, [true, true, true]);
      assert(metrics.bounds.every((card) => card.width > 200));
      if (width === 660) assert(metrics.bounds[1].y > metrics.bounds[0].y);
      else assert.equal(metrics.bounds[1].y, metrics.bounds[0].y);
      await browser.saveScreenshot(`.context/duplicates-native-${width}.png`);
    }
    // The embedded driver's arrow simulation emits change without the click
    // expected by Ark. Check real browser keyboard behavior separately.
    await $('label:has(input[value="properties"])').click();
    await $('.inspection-rule-options').waitForExist();
    assert(
      await browser.execute(
        () => document.querySelector('.duplicate-match-options > div').clientHeight > 200,
      ),
    );
    await browser.saveScreenshot('.context/duplicates-native-properties.png');
    await $('label:has(input[value="normalized_pixels"])').click();
    await $('.inspection-rule-options').waitForExist({ reverse: true });
    const before = await invoke('library_request', { request: { kind: 'inspection_jobs' } });
    assert.equal(before.data.length, 0);
    await $('label:has(input[value="exact_content"])').click();
    await browser.execute(() =>
      document.querySelector('.duplicate-controls button').scrollIntoView({ block: 'center' }),
    );
    await $('button=Start duplicate inspection').click();
    await browser.waitUntil(
      async () => {
        const jobs = await invoke('library_request', { request: { kind: 'inspection_jobs' } });
        return (
          jobs.data.length === 1 &&
          !['queued', 'running', 'cancel_requested'].includes(jobs.data[0].state)
        );
      },
      { timeout: 30000 },
    );
    const jobs = await invoke('library_request', { request: { kind: 'inspection_jobs' } });
    assert(['completed', 'partial'].includes(jobs.data[0].state), jobs.data[0].message);
    assert(jobs.data[0].filesObserved > 0);
    // Generated fixtures include links/unsupported entries, which remain reported limits.
    assert.equal(jobs.data[0].input.task.options.mode, 'exact_content');
    assert.equal(jobs.data[0].input.task.options.algorithm, 'blake3');
    assert.equal(jobs.data[0].input.scope, 'indexed_media_companions');
    const verified = await invoke('test_library_fixture', { verify: true });
    assert(verified.verifiedFiles > 20);
  });
});
