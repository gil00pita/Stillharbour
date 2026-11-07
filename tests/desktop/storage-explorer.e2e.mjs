import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';

describe('Storage explorer · generated native observations', () => {
  it('filters and inspects proportional maps while preserving totals and originals', async function () {
    this.timeout(120000);
    await mkdir('.context/storage-native', { recursive: true });
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
    await $('.root-setup input[required]').setValue('Generated storage smoke');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    await $('button[aria-label="Storage"]').click();
    await $('button[aria-label="Storage measurement"]').click();
    await $('dialog[aria-label="Storage measurement"]').waitForDisplayed();
    await $('button=Start storage inspection').click();
    await $('.storage-explorer').waitForExist({ timeout: 45000 });
    await $('dialog[aria-label="Storage measurement"]').$('button=Close').click();
    assert.equal(await $('button[aria-label="Storage measurement"]').isFocused(), true);
    assert.equal(
      await browser.execute(() => {
        const content = document.querySelector('.inspection-content');
        // Scan version/background status may precede the workspace; setup,
        // job details and coverage belong in the closed measurement dialog.
        return (
          Boolean(content?.querySelector(':scope > .storage-explorer')) &&
          !content.querySelector('.inspection-setup, .inspection-job, .inspection-coverage')
        );
      }),
      true,
    );
    const jobs = await invoke('library_request', { request: { kind: 'inspection_jobs' } });
    const job = jobs.data.find((job) => job.input.task.kind === 'storage');
    assert(['completed', 'partial'].includes(job.state), job.message);
    assert.equal(
      await browser.execute(
        () =>
          document.querySelector('.storage-filters').getBoundingClientRect().top <
          document.querySelector('.storage-overview').getBoundingClientRect().top,
      ),
      true,
    );
    const overview = await $('.storage-overview').getText();
    const geometry = () =>
      browser.execute(() =>
        [...document.querySelectorAll('.storage-tile')].map((tile) => [
          tile.style.left,
          tile.style.top,
          tile.style.width,
          tile.style.height,
        ]),
      );
    const before = await geometry();
    assert(before.length > 0);
    const choose = (label, value) =>
      browser.execute(
        (label, value) => {
          const field = [
            ...document.querySelectorAll(
              '.storage-explorer [data-scope="field"][data-part="root"]',
            ),
          ].find((field) => field.querySelector('label')?.textContent === label);
          const select = field.querySelector('select');
          select.value = value;
          select.dispatchEvent(new Event('change', { bubbles: true }));
        },
        label,
        value,
      );
    await choose('Colour mode', 'Age');
    assert.deepEqual(await geometry(), before);
    await choose('Camera', 'Not camera media');
    assert.equal(await $('.storage-overview').getText(), overview);
    await $('.storage-filters').$('button=Reset').click();
    await $('.storage-tile', { strict: false }).click();
    await $('.storage-inspector > h3').waitForDisplayed();
    assert((await $('.storage-inspector').getText()).includes('Last modified'));
    await $('.storage-toolbar').$('button=List').click();
    const rows = await browser.execute(() =>
      [...document.querySelectorAll('.storage-table tbody tr')].map((row) =>
        row.children[1].getAttribute('data-bytes'),
      ),
    );
    assert(rows.length > 0);
    assert(rows.every((bytes, index) => index === 0 || BigInt(bytes) <= BigInt(rows[index - 1])));
    await $('.storage-toolbar').$('button=Map').click();
    for (const [width, height] of [
      [1440, 1000],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      await $('.storage-filters').scrollIntoView({ block: 'start' });
      assert.equal(
        await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
        true,
      );
      await browser.saveScreenshot(`.context/storage-native/storage-${width}.png`);
    }
    await browser.setWindowSize(1180, 780);
    await $('button[aria-label="Storage measurement"]').click();
    await $('dialog[aria-label="Storage measurement"] .inspection-job').waitForDisplayed();
    await browser.keys('Escape');
    await $('dialog[aria-label="Storage measurement"]').waitForExist({ reverse: true });
    assert.equal(await $('button[aria-label="Storage measurement"]').isFocused(), true);
    await $('.storage-toolbar').$('button=Map').click();
    await $('summary=Complete retained folder totals').click();
    await $('.inspection-map').waitForDisplayed();
    const folder = await $('.inspection-map button:not(:disabled)', { strict: false });
    await folder.click();
    await browser.waitUntil(
      async () =>
        (await browser.execute(
          () => document.querySelectorAll('.inspection-breadcrumbs button').length,
        )) === 2,
    );
    const verified = await invoke('test_library_fixture', { verify: true });
    assert(verified.verifiedFiles > 20);
    await writeFile(
      '.context/storage-native/receipt.json',
      JSON.stringify(
        {
          job,
          displayedByteCounts: rows,
          verified,
          geometryPreserved: true,
          physicalTotalsPreserved: true,
          productionMutation: false,
        },
        null,
        2,
      ),
    );
  });
});
