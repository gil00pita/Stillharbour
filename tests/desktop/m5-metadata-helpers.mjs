import assert from 'node:assert/strict';
import { browser, $, $$ } from '@wdio/globals';
import { assertPreservedXmpExport } from './metadata-export-evidence.mjs';

export async function native(request) {
  return browser.tauri.execute(
    async ({ core }, request) => core.invoke('library_request', { request }),
    request,
  );
}
export const world = { west: -180, south: -90, east: 180, north: 90 };
export const query = {
  rootId: null,
  folder: '',
  name: '',
  fromDate: '',
  throughDate: '',
  minBytes: '',
  maxBytes: '',
  extension: '',
  favoritesOnly: false,
  tag: '',
  sort: 'capture',
  includeMissing: false,
};
async function catalogMetadata(request) {
  const reply = (await native({ kind: 'catalog_metadata', request })).data;
  return reply.data;
}
export async function reviewCatalogMetadata(directory) {
  // Retain the explicit map handoff; sidebar Photos intentionally resets it.
  await $('.photo-toolbar').waitForDisplayed();
  assert.equal(await $('button[aria-label="Photos"]').getAttribute('aria-current'), 'page');
  await $('button=Add to album').waitForDisplayed();
  await $('button=Add to album').click();
  await $('.catalog-organizer').waitForDisplayed();
  await browser.waitUntil(async () => $('button=Create collection').isEnabled());
  const theme = await browser.execute(() => {
    const button = document.querySelector('.catalog-organizer .chakra-button');
    const input = document.querySelector('.catalog-organizer .chakra-input');
    const select = document.querySelector('.catalog-organizer .chakra-native-select');
    return {
      canvas: getComputedStyle(document.documentElement)
        .getPropertyValue('--stillharbour-colors-bg')
        .trim(),
      button: button ? { fontSize: getComputedStyle(button).fontSize, type: button.type } : null,
      input: input
        ? {
            background: getComputedStyle(input).backgroundColor,
            minHeight: getComputedStyle(input).minHeight,
          }
        : null,
      selectHasArrow: Boolean(
        select && getComputedStyle(select).backgroundImage.includes('data:image/svg+xml'),
      ),
    };
  });
  assert.equal(theme.canvas, '#191b19');
  assert.equal(theme.button.fontSize, '13px');
  assert.equal(theme.input.background, 'rgb(23, 26, 23)');
  assert.equal(theme.input.minHeight, '36px');
  assert.equal(theme.selectHasArrow, true);
  const setField = async (label, value) => {
    await browser.execute(
      (label, value) => {
        const control = [...document.querySelectorAll('.catalog-organizer label')]
          .find((node) => node.textContent.trim().startsWith(label))
          ?.querySelector('input,select');
        if (!control) throw new Error(`Missing catalog control ${label}`);
        const descriptor = Object.getOwnPropertyDescriptor(
          control instanceof HTMLSelectElement
            ? HTMLSelectElement.prototype
            : HTMLInputElement.prototype,
          'value',
        );
        descriptor.set.call(control, value);
        control.dispatchEvent(
          new Event(control instanceof HTMLSelectElement ? 'change' : 'input', { bubbles: true }),
        );
      },
      label,
      value,
    );
  };
  await setField('New collection name', 'Generated album');
  await $('button=Create collection').click();
  await browser.waitUntil(async () => $('button=Add selection to album').isEnabled());
  await $('button=Add selection to album').click();
  let album;
  await browser.waitUntil(async () => {
    const values = await catalogMetadata({ kind: 'collections' });
    album = values.find((value) => value.name === 'Generated album');
    return album?.memberCount === 1;
  });
  await browser.waitUntil(async () => $('button=Create collection').isEnabled());
  await setField('New collection name', 'Generated saved search');
  await setField('Collection type', 'search');
  await $('button=Create collection').click();
  let search;
  await browser.waitUntil(async () => {
    search = (await catalogMetadata({ kind: 'collections' })).find(
      (value) => value.name === 'Generated saved search',
    );
    return Boolean(search);
  });
  assert.equal(search.kind, 'search');
  assert.ok(search.query.mapBounds);
  await browser.waitUntil(async () =>
    $('button=Replace saved filters with current filters').isEnabled(),
  );
  await $('button=Replace saved filters with current filters').click();
  await $('section[aria-label="Review saved search filters"]').waitForDisplayed();
  await browser.saveScreenshot(`${directory}/saved-search-review.png`);
  await browser.setWindowSize(660, 560);
  await browser.saveScreenshot(`${directory}/saved-search-review-minimum.png`);
  await $('button=Keep saved filters').click();
  const unchangedSearch = (await catalogMetadata({ kind: 'collections' })).find(
    (value) => value.id === search.id,
  );
  assert.equal(unchangedSearch.version, search.version, 'Canceling filter review does not write');
  await browser.setWindowSize(1180, 780);
  await browser.waitUntil(async () => $('button=Create tag category').isEnabled());
  await setField('Tag label', 'Generated parent');
  await $('button=Create tag category').click();
  let parent;
  await browser.waitUntil(async () => {
    parent = (await catalogMetadata({ kind: 'tag_categories' })).find(
      (value) => value.label === 'Generated parent',
    );
    return Boolean(parent);
  });
  await browser.waitUntil(async () => $('button=Create tag category').isEnabled());
  await setField('Tag label', 'Fixture tag');
  await setField('Parent category', parent.id);
  await $('button=Create tag category').click();
  await browser.waitUntil(
    async () => (await catalogMetadata({ kind: 'tag_categories' })).length === 2,
  );
  await browser.waitUntil(async () => $('button=Close').isEnabled());
  await browser.saveScreenshot(`${directory}/catalog-organization.png`);
  await browser.setWindowSize(660, 560);
  // The pinned embedded driver's click implementation focuses AFTER click,
  // overriding the application's focus destination. Match real pointer order:
  // focus the action first, then let the actual application handler move focus.
  await browser.execute(() => {
    const button = [
      ...document.querySelectorAll('nav[aria-label="Organization sections"] button'),
    ].find((node) => node.textContent === 'Tag categories');
    if (!button) throw new Error('Missing category navigation action.');
    button.focus();
    button.click();
  });
  await browser.waitUntil(async () =>
    browser.execute(() => document.activeElement.id === 'categories-title'),
  );
  const minimum = await browser.execute(() => {
    const node = document.querySelector('.catalog-organizer');
    const header = node.querySelector('header').getBoundingClientRect();
    const body = node.querySelector('.catalog-organizer-scroll');
    const close = [...node.querySelectorAll('button')]
      .find((button) => button.textContent === 'Close')
      .getBoundingClientRect();
    return {
      width: node.clientWidth,
      contentWidth: node.scrollWidth,
      headerTop: header.top,
      closeBottom: close.bottom,
      scrollTop: body.scrollTop,
      focusedSection: document.activeElement.id,
    };
  });
  assert.ok(
    minimum.headerTop >= 0 && minimum.closeBottom < 560,
    'Title and Close remain visible after scrolling',
  );
  assert.ok(minimum.scrollTop > 0, 'Organization body scrolls independently');
  assert.equal(minimum.focusedSection, 'categories-title');
  minimum.focusAcceptanceAsserted = true;
  assert.ok(minimum.contentWidth <= minimum.width + 1, 'Catalog dialog fits minimum width');
  await browser.saveScreenshot(`${directory}/catalog-minimum.png`);
  await browser.setWindowSize(1180, 780);
  await $('button=Close').click();
  const memberPage = (
    await native({ kind: 'query', query: { ...query, albumId: album.id }, cursor: null, limit: 80 })
  ).data;
  assert.equal(memberPage.rows.length, 1);
  const assetId = memberPage.rows[0][0];
  await native({
    kind: 'annotate',
    selection: { kind: 'ids', ids: [assetId] },
    patch: { favorite: null, rating: 4, tags: ['Fixture tag'] },
  });
  await $('button[aria-label="Albums / Tags"]').click();
  await $('button[aria-label="Catalog"]').click();
  await $('.catalog-organizer').waitForDisplayed();
  await $('button[aria-label="View category Generated parent"]').waitForDisplayed();
  await browser.waitUntil(async () =>
    $('button[aria-label="View category Generated parent"]').isEnabled(),
  );
  await $('button[aria-label="View category Generated parent"]').click();
  await browser.waitUntil(async () => (await $$('.asset-open')).length === 1);
  await $('.asset-open').click();
  await $('button[aria-label="Info"]').waitForDisplayed();
  await $('button[aria-label="Info"]').click();
  await $('section[aria-label="Caption and review"] textarea').waitForDisplayed();
  await $('section[aria-label="Caption and review"] textarea').setValue(
    'Generated native caption · 港',
  );
  await $('section[aria-label="Caption and review"] input[type="checkbox"]').click();
  await $('button[aria-label="Info"]').click();
  await $('button[aria-label="Info"]').click();
  await $('section[aria-label="Caption and review"] textarea').waitForDisplayed();
  assert.equal(
    await $('section[aria-label="Caption and review"] textarea').getValue(),
    'Generated native caption · 港',
  );
  assert.equal(
    await $('section[aria-label="Caption and review"] input[type="checkbox"]').isSelected(),
    true,
  );
  const beforeSave = await catalogMetadata({ kind: 'notes', asset_id: assetId });
  assert.notEqual(
    beforeSave.caption,
    'Generated native caption · 港',
    'Draft retention does not save implicitly',
  );
  await $('button=Save caption & review').click();
  let notes;
  await browser.waitUntil(async () => {
    notes = await catalogMetadata({ kind: 'notes', asset_id: assetId });
    return notes.caption === 'Generated native caption · 港' && notes.rejected;
  });
  await browser.saveScreenshot(`${directory}/catalog-notes.png`);
  await $('button[aria-label="Close photo viewer"]').click();
  const rejected = (
    await native({
      kind: 'query',
      query: {
        ...query,
        rejected: true,
        minRating: 4,
        tag: 'Generated parent',
        tagDescendants: true,
      },
      cursor: null,
      limit: 80,
    })
  ).data;
  assert.equal(rejected.total, 1);
  assert.equal(rejected.rows[0][0], assetId);
  return {
    album,
    search,
    categories: await catalogMetadata({ kind: 'tag_categories' }),
    notes,
    selectedAssetId: assetId,
    rejectedCount: rejected.total,
    minimum,
    theme,
  };
}
export async function finishMetadataExports(directory) {
  if (await $('button=Select').isExisting()) await $('button=Select').click();
  await browser.waitUntil(async () => $('.asset-check').isExisting());
  await browser.execute(() => {
    const checkbox = document.querySelector('.asset-check');
    if (checkbox && !checkbox.checked) checkbox.click();
  });
  const output = await browser.tauri.execute(({ core }) =>
    core.invoke('test_performance', { action: 'organization_export_fixture', rows: null }),
  );
  await browser.execute((selection) => {
    window.__wdio_mocks__ ??= {};
    window.__wdio_mocks__.foundation_request = ({ request }) => {
      if (request.kind === 'pick_directory') return { kind: 'directory', data: selection };
      throw new Error('Only the OS picker is substituted by the generated folder authority.');
    };
  }, output);
  try {
    await $('button=Add to album').click();
    await $('.catalog-organizer').waitForDisplayed();
    await $('summary=Rename, merge, remove & undo tags').click();
    await $('section[aria-label="Tag changes"]').waitForDisplayed();
    await browser.execute(() => {
      const inputs = document.querySelectorAll(
        'section[aria-label="Tag changes"] input:not([type="checkbox"])',
      );
      ['Fixture tag', 'Native reviewed tag'].forEach((value, index) => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(
          inputs[index],
          value,
        );
        inputs[index].dispatchEvent(new Event('input', { bubbles: true }));
      });
    });
    await $('button=Preview tag change').click();
    await $('button=Apply reviewed tag change').waitForDisplayed();
    const previousHistory = new Set(
      (await catalogMetadata({ kind: 'tag_history' })).map((entry) => entry.id),
    );
    await $('button=Apply reviewed tag change').waitForEnabled();
    await $('button=Apply reviewed tag change').click();
    await browser.waitUntil(
      async () =>
        (
          await native({
            kind: 'query',
            query: { ...query, tag: 'Native reviewed tag' },
            cursor: null,
            limit: 80,
          })
        ).data.total === 1,
    );
    let appliedHistory;
    await browser.waitUntil(async () => {
      const fresh = (await catalogMetadata({ kind: 'tag_history' })).filter(
        (entry) => !previousHistory.has(entry.id),
      );
      assert.ok(fresh.length <= 1, 'Only the reviewed fixture tag change adds history');
      appliedHistory = fresh[0];
      return Boolean(appliedHistory);
    });
    const undo = $('section[aria-label="Tag changes"]').$(
      `button=Undo tag change ${appliedHistory.id.slice(0, 8)}`,
    );
    await undo.waitForDisplayed();
    await browser.waitUntil(async () => undo.isEnabled());
    await undo.click();
    await browser.waitUntil(
      async () =>
        (
          await native({
            kind: 'query',
            query: { ...query, tag: 'Fixture tag' },
            cursor: null,
            limit: 80,
          })
        ).data.total === 1,
    );
    await browser.saveScreenshot(`${directory}/tag-review-history.png`);
    await browser.waitUntil(async () => $('button=Choose export folder & preview').isEnabled());
    await $('button=Choose export folder & preview').click();
    await $('button=Save reviewed export').waitForDisplayed();
    assert.equal(await $('button=Save reviewed export').isEnabled(), false);
    await $('.metadata-export-review input[type="checkbox"]').click();
    await browser.saveScreenshot(`${directory}/catalog-export-review.png`);
    await $('button=Save reviewed export').click();
    await browser.waitUntil(
      async () =>
        (await $('section[aria-labelledby="metadata-export-title"]').getText()).includes(
          'new files saved',
        ),
      { timeout: 45000 },
    );
    await browser.execute(() => {
      const select = document.querySelector(
        'section[aria-labelledby="metadata-export-title"] select',
      );
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set.call(select, 'xmp');
      select.dispatchEvent(new Event('change', { bubbles: true }));
    });
    await $('button=Choose export folder & preview').click();
    await $('button=Save reviewed export').waitForDisplayed();
    await $('.metadata-export-review input[type="checkbox"]').click();
    await browser.saveScreenshot(`${directory}/xmp-export-review.png`);
    await $('button=Save reviewed export').click();
    await browser.waitUntil(
      async () =>
        (await $('section[aria-labelledby="metadata-export-title"]').getText()).includes(
          'new files saved',
        ),
      { timeout: 45000 },
    );
    await $('button=Close').click();
    const page = (
      await native({
        kind: 'query',
        query: { ...query, name: 'Zero.jpg' },
        cursor: null,
        limit: 80,
      })
    ).data;
    assert.equal(page.rows.length, 1);
    const retainedReviews = [];
    // Always export the sidecar twice, independent of which photo the UI selected.
    for (let index = 0; index < 2; index++) {
      const retained = (
        await native({
          kind: 'metadata_export_preview',
          export_kind: 'xmp',
          selection: { kind: 'ids', ids: [page.rows[0][0]] },
          expected_count: 1,
          selection_id: output.id,
        })
      ).data;
      await native({
        kind: 'metadata_export_save',
        id: retained.id,
        digest: retained.digest,
        selection_id: output.id,
      });
      retainedReviews.push({ id: retained.id, files: retained.files });
    }
    const audit = await browser.tauri.execute(({ core }) =>
      core.invoke('test_performance', { action: 'metadata_export_verify', rows: null }),
    );
    assert.equal(audit.neighborUnchanged, true);
    const catalog = audit.exports.find((file) => file.name.endsWith('.stillharbour'));
    assert.equal(catalog.detail.schema, 17);
    assert.equal(catalog.detail.nativeAuthorities, 0);
    assert.equal(catalog.detail.credentialScheduleApprovalRows, 0);
    assert.equal(catalog.detail.media, 8);
    for (const retained of retainedReviews) assertPreservedXmpExport(audit, retained);
    return { ...audit, preservedXmpReviews: retainedReviews };
  } finally {
    await browser.execute(() => {
      if (window.__wdio_mocks__) delete window.__wdio_mocks__.foundation_request;
    });
  }
}
