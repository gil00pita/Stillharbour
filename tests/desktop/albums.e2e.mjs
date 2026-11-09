import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $, $$ } from '@wdio/globals';

describe('Albums and photo menus · generated library only', () => {
  it('groups scanned tags, creates persistent albums, adds photos and preserves originals', async function () {
    this.timeout(120000);
    await mkdir('.context/albums-native', { recursive: true });
    const invoke = (command, args) =>
      browser.tauri.execute(
        async ({ core }, command, args) => core.invoke(command, args),
        command,
        args,
      );
    const metadata = async (request) =>
      (await invoke('library_request', { request: { kind: 'catalog_metadata', request } })).data
        .data;
    const selection = await invoke('test_library_fixture', { verify: false, tagged: true });
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').click();
    await $('.selected-folder').waitForDisplayed();
    await $('.root-setup input[required]').setValue('Generated albums');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    await browser.waitUntil(
      async () => {
        const albums = await metadata({ kind: 'albums', search: '', offset: 0 });
        return albums.items.find((a) => a.name === 'Outdoors')?.photoCount === 2;
      },
      { timeout: 45000 },
    );
    await browser.waitUntil(
      async () => {
        const status = await invoke('library_request', { request: { kind: 'status' } });
        return status.data.jobs.every(
          (job) => !['queued', 'running', 'cancel_requested'].includes(job.state),
        );
      },
      { timeout: 45000 },
    );
    await $('button[aria-label="Albums / Tags"]').click();
    await $('button[aria-label="Coast, 1 photo, From tags"]').waitForDisplayed();
    await browser.saveScreenshot('.context/albums-native/tag-albums.png');
    const sourceAlbums = await metadata({ kind: 'albums', search: '', offset: 0 });
    assert.equal(sourceAlbums.items.find((a) => a.name === 'Coast').photoCount, 1);
    assert.equal(sourceAlbums.items.find((a) => a.name === 'Outdoors').photoCount, 2);
    await $('button[aria-label="New album"]').click();
    await $('dialog[open] input[required]').setValue('Weekends');
    await $('dialog[open] .asset-check', { strict: false }).waitForDisplayed();
    assert(
      await browser.execute(() =>
        [...document.querySelectorAll('dialog[open] .thumbnail-selection')].every(
          (mark) => getComputedStyle(mark).opacity === '1',
        ),
      ),
      'Album picker selection marks must be visible before choosing any photo.',
    );
    const checks = await $$('dialog[open] .asset-check');
    await checks[0].click();
    await checks[1].click();
    assert(
      await browser.execute(() => {
        const dialog = document.querySelector('dialog[open]').getBoundingClientRect();
        const save = document
          .querySelector('dialog[open] footer button.primary')
          .getBoundingClientRect();
        return save.bottom <= dialog.bottom - 8 && save.top >= dialog.top;
      }),
      'The album save action must remain fully visible.',
    );
    await browser.saveScreenshot('.context/albums-native/album-picker.png');
    await $('dialog[open]').$('button=Create album').click();
    await $('button[aria-label="Weekends, 2 photos"]').waitForDisplayed();
    const custom = (await metadata({ kind: 'collections' })).find((c) => c.name === 'Weekends');
    assert.equal(custom.memberCount, 2);
    await $('button[aria-label="Weekends, 2 photos"]').click();
    await $('button=Add photos').click();
    await $('dialog[open] .asset-check', { strict: false }).waitForDisplayed();
    const more = await $$('dialog[open] .asset-check');
    await more[2].click();
    await $('dialog[open]').$('button=Add photos').click();
    await browser.waitUntil(
      async () =>
        (await metadata({ kind: 'collections' })).find((c) => c.id === custom.id).memberCount === 3,
    );
    await $('button=← All albums').click();
    await $('button[aria-label="New album"]').click();
    await $('dialog[open] input[required]').setValue('weekends');
    await $('dialog[open]').$('button=Create album').click();
    await $('dialog[open] [role="alert"]').waitForDisplayed();
    assert((await $('dialog[open] [role="alert"]').getText()).includes('already exists'));
    await $('dialog[open]').$('button=Cancel').click();
    await $('button[aria-label="Photos"]').click();
    const photo = $('.asset-open', { strict: false });
    await photo.waitForDisplayed();
    // The pinned embedded driver emits secondary pointer events without the
    // browser's contextmenu event. Dispatch that event in the native WebView;
    // ordinary menu buttons and picker actions still use driver input.
    await browser.execute(() => {
      const node = document.querySelector('.asset-open');
      const rect = node.getBoundingClientRect();
      node.dispatchEvent(
        new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          button: 2,
          clientX: rect.left + 20,
          clientY: rect.top + 20,
        }),
      );
    });
    await browser.saveScreenshot('.context/albums-native/right-click.png');
    await $('[role="menuitem"][data-value="add-to-album"]').waitForDisplayed();
    await $('[role="menuitem"][data-value="add-to-album"]').click();
    await $('dialog[open]').$('button*=Weekends').click();
    const countAfter = (await metadata({ kind: 'collections' })).find(
      (c) => c.id === custom.id,
    ).memberCount;
    const openContext = async () =>
      browser.execute(() => {
        const node = document.querySelector('.asset-open');
        const rect = node.getBoundingClientRect();
        node.dispatchEvent(
          new MouseEvent('contextmenu', {
            bubbles: true,
            cancelable: true,
            clientX: rect.left + 20,
            clientY: rect.top + 20,
          }),
        );
      });
    assert.equal(await $('.photo-album-menu-button').isExisting(), false);
    await openContext();
    await $('[role="menuitem"][data-value="add-to-album"]').click();
    await $('dialog[open]').$('button*=Weekends').click();
    assert.equal(
      (await metadata({ kind: 'collections' })).find((c) => c.id === custom.id).memberCount,
      countAfter,
    );
    await openContext();
    await $('[role="menuitem"][data-value="add-to-album"]').click();
    await $('dialog[open]').$('button=+ Create new album').click();
    await $('dialog[open] input[required]').setValue('Little moments');
    assert((await $('dialog[open] footer').getText()).includes('1 photos selected'));
    await $('dialog[open]').$('button=Create album').click();
    await browser.waitUntil(async () =>
      (await metadata({ kind: 'collections' })).some(
        (c) => c.name === 'Little moments' && c.memberCount === 1,
      ),
    );
    await browser.refresh();
    await $('button[aria-label="Albums / Tags"]').waitForDisplayed();
    await $('button[aria-label="Albums / Tags"]').click();
    await $('button[aria-label="Little moments, 1 photo"]').waitForDisplayed();
    for (const [width, height] of [
      [1440, 1000],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      await browser.waitUntil(async () =>
        browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
      );
      await browser.saveScreenshot(`.context/albums-native/albums-${width}.png`);
    }
    await $('button[aria-label="New album"]').click();
    await $('dialog[open]').waitForDisplayed();
    assert(
      await browser.execute(() => {
        const dialog = document.querySelector('dialog[open]').getBoundingClientRect();
        const save = document
          .querySelector('dialog[open] footer button.primary')
          .getBoundingClientRect();
        return (
          save.bottom <= dialog.bottom - 8 && save.left >= dialog.left && save.right <= dialog.right
        );
      }),
      'The narrow album dialog must retain its save action.',
    );
    await browser.saveScreenshot('.context/albums-native/album-picker-660.png');
    await $('dialog[open]').$('button=Cancel').click();
    const originals = await invoke('test_library_fixture', { verify: true });
    assert(originals.verifiedFiles > 0);
    const foundation = await invoke('foundation_request', { request: { kind: 'status' } });
    assert.equal(foundation.data.originalMutationsEnabled, false);
    await writeFile(
      '.context/albums-native/receipt.json',
      JSON.stringify(
        {
          platform: process.platform,
          generatedSourcesVerifiedUnchanged: originals.verifiedFiles,
          originalMutationsEnabled: foundation.data.originalMutationsEnabled,
          customAlbums: (await metadata({ kind: 'collections' }))
            .filter((c) => c.kind === 'album')
            .map((c) => ({ name: c.name, photoCount: c.memberCount })),
          tagAlbums: (await metadata({ kind: 'albums', search: '', offset: 0 })).items
            .filter((c) => c.fromTags)
            .map((c) => ({ name: c.name, photoCount: c.photoCount })),
          sizes: [
            [1440, 1000],
            [660, 560],
          ],
          contextMenuInput:
            'Dispatched contextmenu in the native WebView; embedded driver lacks secondary-click synthesis.',
        },
        null,
        2,
      ),
    );
  });
});
