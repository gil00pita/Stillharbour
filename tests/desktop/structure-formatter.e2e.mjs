import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';

describe('Structure Formater — entire generated library', () => {
  it('opens directly on rules and previews every indexed item beyond the 240-row page', async function () {
    this.timeout(120000);
    await mkdir('.context/ui-redesign', { recursive: true });
    const invoke = (command, args) =>
      browser.tauri.execute(
        async ({ core }, command, args) => core.invoke(command, args),
        command,
        args,
      );
    const native = (request) => invoke('library_request', { request });
    const selection = await invoke('test_library_fixture', { verify: false });
    await $('.welcome button.primary').waitForEnabled();
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').click();
    await $('.root-setup input[required]').setValue('Generated whole-library structure');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    // Leave the gallery before adding catalog-only placeholder observations.
    await $('button[aria-label="Structure Formater"]').click();
    await $('button=Preview restructure').waitForDisplayed();
    assert.equal(await $('button=Choose photos').isExisting(), false);
    assert.equal(await $('.asset-check:checked').isExisting(), false);
    const controls = await browser.execute(() => ({
      cards: document.querySelectorAll('.organization-presets input[type="radio"]').length,
      illustrations: document.querySelectorAll('.organization-presets svg[aria-hidden="true"]')
        .length,
      dropdowns: document.querySelectorAll('.structure-page fieldset select').length,
      eventField: [...document.querySelectorAll('.structure-page label')].some((label) =>
        label.textContent.includes('Event name'),
      ),
    }));
    assert.deepEqual(controls, { cards: 8, illustrations: 8, dropdowns: 0, eventField: false });
    assert.deepEqual(await native({ kind: 'organization_protection_status' }), {
      kind: 'organization_protection_status',
      data: { blockers: [] },
    });
    await browser.waitUntil(() =>
      browser.execute(() =>
        document
          .querySelector('.structure-page')
          ?.textContent.includes(
            'No active backup or sync, unresolved jobs or enabled backup schedules found.',
          ),
      ),
    );
    assert.equal(await $('button=Run restructure').isEnabled(), false);
    // The embedded 1.5.1 driver moves radio focus/checked state and fires change,
    // but omits the native activation click that Ark consumes. Complete that
    // activation after asserting the driver's focused radio, without changing
    // application keyboard behavior. This does not certify physical key input.
    const radioKey = async (key) => {
      await browser.keys(key);
      const activation = await browser.execute(() => {
        const input = document.activeElement;
        if (!(input instanceof HTMLInputElement) || input.type !== 'radio' || !input.checked)
          return false;
        input.click();
        return true;
      });
      assert.equal(activation, true);
    };
    await browser.execute(() =>
      document.querySelector('.organization-presets input:checked').focus({ preventScroll: true }),
    );
    await radioKey('ArrowRight');
    await browser.waitUntil(() =>
      browser.execute(() =>
        document
          .querySelector('input[aria-describedby="organization-token-help"]')
          .value.includes('{capture:%m}'),
      ),
    );
    await radioKey('ArrowRight');
    await browser.waitUntil(() =>
      browser.execute(
        () =>
          document.querySelector('input[aria-describedby="organization-token-help"]').value ===
          '{capture:%Y-%m-%d}/{capture:%H%M%S}_{original_stem}.{ext}',
      ),
    );
    const focusSegment = (label) =>
      browser.execute((label) => {
        const group = [...document.querySelectorAll('.segmented-control')].find(
          (element) =>
            document.getElementById(element.getAttribute('aria-labelledby'))?.textContent === label,
        );
        group.querySelector('input:checked').focus({ preventScroll: true });
      }, label);
    const segmentValue = (label) =>
      browser.execute((label) => {
        const group = [...document.querySelectorAll('.segmented-control')].find(
          (element) =>
            document.getElementById(element.getAttribute('aria-labelledby'))?.textContent === label,
        );
        return group.querySelector('[data-part="item"][data-state="checked"] input')?.value;
      }, label);
    await focusSegment('When capture date is missing');
    await radioKey('ArrowRight');
    await browser.waitUntil(
      async () => (await segmentValue('When capture date is missing')) === 'block',
    );
    await radioKey('ArrowLeft');
    await browser.waitUntil(
      async () => (await segmentValue('When capture date is missing')) === 'undated',
    );
    await focusSegment('Filename stem case');
    await radioKey('ArrowRight');
    await browser.waitUntil(async () => (await segmentValue('Filename stem case')) === 'lowercase');
    await radioKey('ArrowRight');
    await browser.waitUntil(async () => (await segmentValue('Filename stem case')) === 'uppercase');
    assert.equal(await invoke('test_scroll_catalog'), 960);
    // Native status must settle before the read-only revision-bound preview.
    await browser.waitUntil(async () =>
      (await native({ kind: 'status' })).data.jobs.every(
        (job) => !['queued', 'running'].includes(job.state),
      ),
    );
    await browser.execute(() => {
      window.__structureRequests = [];
      window.addEventListener('stillharbour:test-library-request', (event) => {
        if (event.detail.kind.startsWith('organization_library_'))
          window.__structureRequests.push(event.detail);
      });
    });
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      await browser.execute(() => {
        document.querySelector('.structure-content').scrollTop = 0;
      });
      await browser.saveScreenshot(`.context/ui-redesign/structure-layouts-rules-${width}.png`);
      await browser.execute(() =>
        document.getElementById('structure-protection-title').scrollIntoView({ block: 'center' }),
      );
      const protectionFit = await browser.execute(() => {
        const section = document.getElementById('structure-protection-title').closest('section');
        return section.scrollWidth <= section.clientWidth;
      });
      assert.equal(protectionFit, true);
      await browser.saveScreenshot(`.context/ui-redesign/structure-protection-${width}.png`);
    }
    await browser.execute(() =>
      [...document.querySelectorAll('.structure-page button')]
        .find((button) => button.textContent.trim() === 'Preview restructure')
        .click(),
    );
    await $('section[aria-label="Proposed library structure"]').waitForDisplayed({
      timeout: 45000,
    });
    const requests = await browser.execute(() => window.__structureRequests);
    assert.equal(requests[0].kind, 'organization_library_start');
    assert.equal('asset_ids' in requests[0], false);
    assert.equal('query' in requests[0], false);
    assert.equal(requests[0].template.event, '');
    assert.equal(requests[0].template.filenameStemCase, 'uppercase');
    assert.equal(
      requests[0].template.pattern,
      '{capture:%Y-%m-%d}/{capture:%H%M%S}_{original_stem}.{ext}',
    );
    assert(
      requests.filter((request) => request.kind === 'organization_library_advance').length >= 4,
    );
    const id = requests.find(
      (request) => request.kind === 'organization_library_advance',
    ).preview_id;
    const first = (await native({ kind: 'organization_library_page', preview_id: id, offset: 0 }))
      .data;
    assert(first.total >= 960);
    assert.equal(first.processed, first.total);
    assert.equal(first.rows.length, 240);
    assert.equal(first.executionAvailable, false);
    assert.equal(await $('button=Run restructure').isEnabled(), false);
    const lastOffset = Math.floor((first.total - 1) / 240) * 240;
    let current = first;
    while (current.offset < lastOffset) {
      await $('button=Next page').waitForEnabled();
      await browser.execute(() =>
        [...document.querySelectorAll('.structure-page button')]
          .find((button) => button.textContent.trim() === 'Next page')
          .click(),
      );
      const expectedOffset = current.offset + 240;
      await browser.waitUntil(() =>
        browser.execute(
          (expectedOffset) =>
            document
              .querySelector('.organization-table caption')
              ?.textContent.includes(`items ${expectedOffset + 1}–`),
          expectedOffset,
        ),
      );
      current = (
        await native({ kind: 'organization_library_page', preview_id: id, offset: expectedOffset })
      ).data;
    }
    assert.equal(current.rows.at(-1).sequence, first.total);
    assert.equal(await $('button=Next page').isEnabled(), false);
    const rendered = await browser.execute(() =>
      [...document.querySelectorAll('.organization-table tbody tr')].map(
        (row) => row.cells[0].querySelector('bdi').textContent,
      ),
    );
    assert.deepEqual(
      rendered,
      current.rows.map((row) => row.sourcePath),
    );
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      const layout = await browser.execute(() => ({
        overflow: document.documentElement.scrollWidth > innerWidth,
        scrollable:
          getComputedStyle(document.querySelector('.structure-content')).overflowY === 'auto',
      }));
      assert.deepEqual(layout, { overflow: false, scrollable: true });
      await browser.execute(() =>
        document
          .querySelector('.organization-table caption')
          .scrollIntoView({ block: 'start', behavior: 'instant' }),
      );
      await browser.saveScreenshot(`.context/ui-redesign/structure-layouts-results-${width}.png`);
    }
    await writeFile(
      '.context/ui-redesign/structure-layouts.json',
      JSON.stringify({
        total: first.total,
        processed: first.processed,
        proposed: first.proposed,
        blocked: first.blocked,
        firstSequence: first.rows[0].sequence,
        lastSequence: current.rows.at(-1).sequence,
        advanceRequests: requests.filter(
          (request) => request.kind === 'organization_library_advance',
        ).length,
        boundedPage: first.rows.length,
        executionAvailable: false,
      }),
    );
    // Test-owned catalog placeholders carry explicit naming observations. They
    // remain unavailable originals; this tests resolution, not source decoding.
    const observations = [];
    for (const [pattern, expectedPrefix] of [
      ['{country}/{city}/{original_stem}.{ext}', 'Portugal/Lagos/'],
      ['{tag}/{subcategory}/{subject}/{original_stem}.{ext}', 'Nature/Birds/Kingfishers/'],
      ['{camera_type}/{brand}/{model}/{original_stem}.{ext}', 'Mirrorless/Sony/A7-IV/'],
      ['{camera}/{file_format}/{original_stem}.{ext}', 'Sony-A7-IV/JPEG/'],
    ]) {
      let preview = (
        await native({
          kind: 'organization_library_start',
          template: { pattern, event: '', missingDate: 'undated' },
        })
      ).data;
      while (!preview.complete)
        preview = (
          await native({ kind: 'organization_library_advance', preview_id: preview.previewId })
        ).data;
      let found = preview.rows.some(
        (row) =>
          row.sourcePath.startsWith('__catalog_fixture__/') &&
          row.targetRelativePath?.startsWith(expectedPrefix),
      );
      for (let offset = 240; !found && offset < preview.total; offset += 240) {
        const page = (
          await native({ kind: 'organization_library_page', preview_id: preview.previewId, offset })
        ).data;
        found = page.rows.some(
          (row) =>
            row.sourcePath.startsWith('__catalog_fixture__/') &&
            row.targetRelativePath?.startsWith(expectedPrefix),
        );
      }
      assert.equal(found, true, pattern);
      observations.push({
        pattern,
        expectedPrefix,
        total: preview.total,
        complete: preview.complete,
      });
      await native({ kind: 'organization_library_discard', preview_id: preview.previewId });
    }
    const chooseCategory = async (value) => {
      await browser.execute((value) => {
        const group = [...document.querySelectorAll('.segmented-control')].find(
          (element) =>
            document.getElementById(element.getAttribute('aria-labelledby'))?.textContent ===
            'Layout type',
        );
        [...group.querySelectorAll('input')].find((input) => input.value === value).click();
      }, value);
      await browser.waitUntil(() =>
        browser.execute((value) => {
          const group = [...document.querySelectorAll('.segmented-control')].find(
            (element) =>
              document.getElementById(element.getAttribute('aria-labelledby'))?.textContent ===
              'Layout type',
          );
          return group.querySelector('[data-state="checked"] input')?.value === value;
        }, value),
      );
    };
    await $('h1').moveTo();
    for (const [category, expectedCards] of [
      ['location', 4],
      ['tags', 6],
      ['camera', 7],
    ]) {
      await chooseCategory(category);
      assert.equal(
        await browser.execute(
          () => document.querySelectorAll('.organization-presets input[type="radio"]').length,
        ),
        expectedCards,
      );
      await browser.setWindowSize(1180, 780);
      await browser.execute(() => (document.querySelector('.structure-content').scrollTop = 0));
      await browser.saveScreenshot(`.context/ui-redesign/structure-layouts-${category}-1180.png`);
    }
    await chooseCategory('custom');
    const builder = await $('input[role="combobox"]');
    await builder.setValue('Country');
    await browser.keys('Enter');
    await browser.waitUntil(() =>
      browser.execute(() =>
        document
          .querySelector('input[aria-describedby="organization-token-help"]')
          .value.includes('/{country}/'),
      ),
    );
    for (const part of ['City', 'Model']) {
      await builder.setValue(part);
      await browser.keys('Enter');
    }
    await browser.setWindowSize(660, 560);
    await browser.execute(() =>
      document.querySelector('.structure-path-input').scrollIntoView({ block: 'center' }),
    );
    const drag = await browser.execute(() => {
      const handles = [...document.querySelectorAll('[data-folder-index]')];
      const from = handles[0].getBoundingClientRect();
      const to = handles.at(-1).getBoundingClientRect();
      const pattern = document.querySelector(
        'input[aria-describedby="organization-token-help"]',
      ).value;
      const parts = pattern.split('/');
      const filename = parts.pop();
      parts.push(parts.shift());
      return {
        from: {
          x: Math.round(from.left + from.width / 2),
          y: Math.round(from.top + from.height / 2),
        },
        to: { x: Math.round(to.left + to.width / 2), y: Math.round(to.top + to.height / 2) },
        wrapped: to.top > from.top,
        pattern,
        expected: [...parts, filename].join('/'),
      };
    });
    assert.equal(drag.wrapped, true, 'The native drag must cross wrapped chip rows');
    await browser.performActions([
      {
        type: 'pointer',
        id: 'folder-chip',
        parameters: { pointerType: 'mouse' },
        actions: [
          { type: 'pointerMove', duration: 0, ...drag.from },
          { type: 'pointerDown', button: 0 },
          { type: 'pointerMove', duration: 180, ...drag.to },
        ],
      },
    ]);
    await $('[data-folder-drag-preview]').waitForExist();
    assert.equal(
      await browser.execute(
        () => document.querySelector('input[aria-describedby="organization-token-help"]').value,
      ),
      drag.pattern,
      'Dragging must not submit intermediate rules',
    );
    await browser.saveScreenshot('.context/ui-redesign/structure-chips-dragging-660.png');
    await browser.performActions([
      {
        type: 'pointer',
        id: 'folder-chip',
        parameters: { pointerType: 'mouse' },
        actions: [{ type: 'pointerUp', button: 0 }],
      },
    ]);
    await browser.releaseActions();
    await browser.waitUntil(() =>
      browser.execute(
        (expected) =>
          document.querySelector('input[aria-describedby="organization-token-help"]').value ===
          expected,
        drag.expected,
      ),
    );
    assert.equal(await $('[data-folder-drag-preview]').isExisting(), false);
    await writeFile('.context/ui-redesign/structure-chip-drag.json', JSON.stringify(drag, null, 2));
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      const geometry = await browser.execute(() => ({
        overflow: document.documentElement.scrollWidth > innerWidth,
        chips: document.querySelectorAll('.structure-path-chip').length,
      }));
      assert.equal(geometry.overflow, false);
      assert(geometry.chips > 0);
      await browser.execute(() =>
        document
          .querySelector('.structure-layout-picker')
          .scrollIntoView({ block: 'start', behavior: 'instant' }),
      );
      await browser.saveScreenshot(`.context/ui-redesign/structure-layouts-custom-${width}.png`);
      await browser.execute(() =>
        document.querySelector('.structure-path-input').scrollIntoView({ block: 'center' }),
      );
      await browser.saveScreenshot(`.context/ui-redesign/structure-chips-${width}.png`);
    }
    await browser.execute(() =>
      [...document.querySelectorAll('.structure-page button')]
        .find((button) => button.textContent.trim() === 'Preview restructure')
        .click(),
    );
    await $('section[aria-label="Proposed library structure"]').waitForDisplayed();
    const customStart = await browser.execute(() =>
      window.__structureRequests
        .filter((request) => request.kind === 'organization_library_start')
        .at(-1),
    );
    assert.equal(customStart.template.pattern, drag.expected);
    const activeId = await browser.execute(
      () =>
        window.__structureRequests
          .filter((request) => request.kind === 'organization_library_advance')
          .at(-1).preview_id,
    );
    await writeFile(
      '.context/ui-redesign/structure-layouts-observations.json',
      JSON.stringify(observations),
    );
    await $('button[aria-label="Settings"]').click();
    await browser.waitUntil(() =>
      browser.execute(
        (activeId) =>
          window.__structureRequests.some(
            (request) =>
              request.kind === 'organization_library_discard' && request.preview_id === activeId,
          ),
        activeId,
      ),
    );
    await browser.waitUntil(async () => {
      try {
        await native({ kind: 'organization_library_page', preview_id: activeId, offset: 0 });
        return false;
      } catch {
        return true;
      }
    });
    await invoke('test_library_fixture', { verify: true });
  });
});
