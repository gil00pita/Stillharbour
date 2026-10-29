import assert from 'node:assert/strict';
import { browser, $, $$ } from '@wdio/globals';
import { mkdir, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';

async function native(request) {
  return browser.tauri.execute(
    async ({ core }, request) => core.invoke('library_request', { request }),
    request,
  );
}
async function assertPhotoCheckboxOnTop(id) {
  // HTMLElement.click() in the embedded driver bypasses real pointer hit testing.
  const hit = await browser.execute((id) => {
    const checkbox = document.querySelector(`[data-asset-id="${id}"] .asset-check`);
    const bounds = checkbox.getBoundingClientRect();
    return {
      onTop:
        document.elementFromPoint(
          bounds.left + bounds.width / 2,
          bounds.top + bounds.height / 2,
        ) === checkbox,
      visible: getComputedStyle(checkbox).opacity === '1',
    };
  }, id);
  assert.deepEqual(hit, { onTop: true, visible: true });
}
async function changeSizeUnit(label, unit) {
  // The embedded driver's option click only calls option.click(), which does
  // not perform a native select change in WKWebView. Exercise its change event.
  await browser.execute(
    (label, unit) => {
      const select = document.querySelector(`select[aria-label="${label}"]`);
      select.value = unit;
      select.dispatchEvent(new Event('change', { bubbles: true }));
    },
    label,
    unit,
  );
  assert.equal(await $(`select[aria-label="${label}"]`).getValue(), unit);
}
async function openFilters() {
  await $('button[aria-controls="library-filter-panel"]').click();
  await $('#library-filter-panel[open]').waitForDisplayed();
}
async function galleryView(label) {
  await openFilters();
  await (await $('#library-filter-panel')).$(`button=${label}`).click();
  await $('button=Apply filters').click();
  await browser.waitUntil(async () =>
    browser.execute(() => !document.querySelector('#library-filter-panel[open]')),
  );
}
async function explorerLayoutReady() {
  await browser.waitUntil(async () =>
    browser.execute(() => {
      const viewport = document.querySelector('.explorer-viewport');
      const row = viewport?.querySelector('.explorer-row');
      if (!row) return false;
      const expected = viewport.classList.contains('details')
        ? 1
        : Math.max(1, Math.floor(viewport.clientWidth / 180));
      return getComputedStyle(row).gridTemplateColumns.trim().split(/\s+/).length === expected;
    }),
  );
}
const observations = {};
let generatedSelection;
const expectedExtensions =
  'jpg jpeg jfif heic heif hif png tif tiff webp avif gif bmp dng cr3 cr2 crw nef nrw arw sr2 srf raf orf rw2 raw pef srw x3f 3fr fff iiq rwl'.split(
    ' ',
  );
describe('native read-only library', () => {
  it('connects to a fresh native catalog and rejects renderer path authority', async () => {
    await $('h1').waitForDisplayed();
    await $('button.primary').waitForEnabled();
    assert.equal(await $('h1').getText(), 'Your library');
    const status = await browser.tauri.execute(async ({ core }) =>
      core.invoke('foundation_request', { request: { kind: 'status' } }),
    );
    assert.equal(status.data.catalogSchema, 2);
    assert.equal(await $('.sidebar-version').getText(), `Version ${status.data.version}`);
    assert.equal(
      await browser.execute(() =>
        document.querySelector('.sidebar').textContent.includes('Local by design.'),
      ),
      false,
    );
    assert.equal(status.data.originalMutationsEnabled, false);
    assert.equal(await $('h2').getText(), 'Start by selecting your Photo library folder.');
    for (const [command, request] of [
      ['foundation_request', { kind: 'pick_directory', path: '/not-authorised' }],
      [
        'library_request',
        {
          kind: 'save_and_scan',
          selection_id: 'invented',
          label: 'unapproved',
          mode: 'referenced',
          filter: {},
        },
      ],
    ]) {
      const result = await browser.tauri.execute(
        async ({ core }, command, request) => {
          try {
            await core.invoke(command, { request });
            return 'unexpected success';
          } catch {
            return 'rejected';
          }
        },
        command,
        request,
      );
      assert.equal(result, 'rejected');
    }
    await mkdir('.context/screenshots', { recursive: true });
    await browser.saveScreenshot('.context/screenshots/m1-onboarding.png');
  });
});
const describeMac = process.platform === 'darwin' ? describe : describe.skip;
describeMac('macOS M1 library — generated native fixtures', () => {
  let sourceRoot;
  let sourceManifest;
  const evidence = `.context/gallery/native-${randomUUID()}`;
  after(async () => {
    if (!sourceRoot) return;
    const after = JSON.parse(
      execFileSync('python3', ['scripts/performance/manifest.py', sourceRoot], {
        encoding: 'utf8',
      }),
    );
    await writeFile(`${evidence}/manifest-after.json`, JSON.stringify(after));
    assert.deepEqual(after, sourceManifest);
  });
  it('saves and scans a backend-owned generated library through real Rust IPC', async () => {
    const selection = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: false }),
    );
    generatedSelection = selection;
    sourceRoot = (
      await browser.tauri.execute(async ({ core }) =>
        core.invoke('test_performance', { action: 'fixture_location', rows: null }),
      )
    ).fixtureRoot;
    sourceManifest = JSON.parse(
      execFileSync('python3', ['scripts/performance/manifest.py', sourceRoot], {
        encoding: 'utf8',
      }),
    );
    await mkdir(evidence, { recursive: true });
    await writeFile(`${evidence}/manifest-before.json`, JSON.stringify(sourceManifest));
    const picker = await browser.tauri.mock('foundation_request');
    try {
      await picker.mockReturnValue({ kind: 'directory', data: selection });
      await $('button.primary').click();
      await $('.selected-folder').waitForDisplayed();
      assert.equal(await $('.root-setup input[required]').isFocused(), true);
      await $('.root-setup input[required]').setValue('Generated test library');
      assert.equal((await $$('.welcome')).length, 0);
      assert.equal((await $$('input[name="includeGlobs"]')).length, 0);
      assert.equal(await $('.scan-options summary').getText(), 'Advanced');
      assert.equal(await $('.scan-options').getAttribute('open'), null);
      await $('.scan-options summary').click();
      const preset = await browser.execute(() =>
        [...document.querySelectorAll('.tag-field[data-field="extensions"] li bdi')].map(
          (node) => node.textContent,
        ),
      );
      assert.deepEqual(
        preset,
        expectedExtensions.map((ext) => `.${ext}`),
      );
      await $('input[name="excludedFolders"]').setValue('Private');
      await browser.keys('Enter');
      assert.equal(await $('.mode-field input[role="switch"]').isSelected(), false);
      await $('.mode-field input[role="switch"]').click();
      assert.equal(await $('.mode-field input[role="switch"]').isSelected(), true);
      await $('.mode-field input[role="switch"]').click();
      assert.equal((await $$('input[type="date"]')).length, 0);
      await $('button[aria-label="Remove Private from Excluded folder names"]').waitForDisplayed();
      await $('input[name="extensions"]').setValue('.JPG');
      await browser.keys('Enter');
      assert.equal((await $$('button[aria-label="Remove .jpg from File Extensions"]')).length, 1);
      await $('button[aria-label="Remove .jpg from File Extensions"]').click();
      await $('input[name="extensions"]').setValue('.jpg');
      await browser.keys('Enter');
      const beforeSave = await native({ kind: 'status' });
      assert.equal(beforeSave.data.roots.length, 0);
      await $('input[name="minBytes"]').setValue('1.5');
      await changeSizeUnit('Minimum size unit', 'MB');
      await $('input[name="maxBytes"]').setValue('2');
      await changeSizeUnit('Maximum size unit', 'GB');
      for (const [width, height, size] of [
        [1180, 780, 'desktop'],
        [660, 560, 'minimum'],
      ]) {
        await browser.setWindowSize(width, height);
        const scrolling = await browser.execute(() => {
          const pane = document.querySelector('main');
          const sidebar = document.querySelector('.sidebar');
          const brand = document.querySelector('.brand');
          const navigation = document.querySelector('.sidebar nav');
          const toolbar = document.querySelector('.toolbar');
          const form = document.querySelector('.root-setup');
          const advanced = document.querySelector('.scan-options');
          pane.scrollTop = 0;
          const before = {
            sidebar: sidebar.getBoundingClientRect().top,
            brand: brand.getBoundingClientRect().top,
            navigation: navigation.getBoundingClientRect().top,
            toolbar: toolbar.getBoundingClientRect().top,
          };
          pane.scrollTop = pane.scrollHeight;
          return {
            before,
            after: {
              sidebar: sidebar.getBoundingClientRect().top,
              brand: brand.getBoundingClientRect().top,
              navigation: navigation.getBoundingClientRect().top,
              toolbar: toolbar.getBoundingClientRect().top,
            },
            mainScroll: pane.scrollTop,
            formHeight: form.clientHeight,
            formScrollHeight: form.scrollHeight,
            formWidth: form.clientWidth,
            formScrollWidth: form.scrollWidth,
            advancedHeight: advanced.clientHeight,
            advancedScrollHeight: advanced.scrollHeight,
            pageScroll: window.scrollY,
            sidebarScroll: sidebar.scrollTop,
          };
        });
        assert.ok(scrolling.mainScroll > 0, 'Long setup must scroll in the main pane');
        assert.ok(scrolling.after.toolbar < scrolling.before.toolbar, 'Page content must move');
        for (const part of ['sidebar', 'brand', 'navigation'])
          assert.ok(
            Math.abs(scrolling.before[part] - scrolling.after[part]) < 1,
            `${part} moved while scrolling page content`,
          );
        assert.ok(
          scrolling.formScrollHeight <= scrolling.formHeight + 1,
          'Form must expand instead of gaining a vertical scrollbar',
        );
        assert.ok(
          scrolling.formScrollWidth <= scrolling.formWidth + 1,
          'Form must not gain a horizontal scrollbar',
        );
        assert.ok(
          scrolling.advancedScrollHeight <= scrolling.advancedHeight + 1,
          'Advanced must expand naturally',
        );
        assert.equal(scrolling.pageScroll, 0);
        assert.equal(scrolling.sidebarScroll, 0);
        await browser.saveScreenshot(`.context/screenshots/advanced-setup-scrolled-${size}.png`);
        await browser.execute(() =>
          document.querySelector('.file-size-field').scrollIntoView({ block: 'center' }),
        );
        await browser.saveScreenshot(`.context/screenshots/size-units-${size}.png`);
      }
      await browser.setWindowSize(1180, 980);
      await browser.execute(() =>
        document.querySelector('.root-setup').scrollIntoView({ block: 'start' }),
      );
      await browser.execute(() => {
        document.querySelector('.root-setup').scrollTop = 0;
      });
      await browser.saveScreenshot('.context/screenshots/advanced-setup-desktop.png');
      await browser.setWindowSize(660, 560);
      await browser.execute(() =>
        document.querySelector('.root-setup').scrollIntoView({ block: 'start' }),
      );
      assert.equal(
        await browser.execute(() => document.documentElement.scrollWidth <= window.innerWidth),
        true,
      );
      await browser.execute(() => {
        document.querySelector('.root-setup').scrollTop = 0;
      });
      await browser.saveScreenshot('.context/screenshots/advanced-setup-minimum.png');
      await browser.execute(() =>
        document.querySelector('.scan-options').scrollIntoView({ block: 'start' }),
      );
      await browser.saveScreenshot('.context/screenshots/advanced-setup-minimum-expanded.png');
      await browser.setWindowSize(1180, 780);

      await $('input[name="minBytes"]').setValue('0');
      await changeSizeUnit('Minimum size unit', 'KB');

      await $('.folder-actions .primary').click();
    } finally {
      await picker.mockRestore();
    }
    await $('.asset-viewport').waitForDisplayed();
    // Diagnostic schedule: deterministically exercise completion before opening
    // Jobs, instead of relying on how fast the host scans the fixture.
    if (process.env.STILLHARBOUR_TEST_JOBS_COMPLETED === '1') {
      await browser.waitUntil(
        async () => {
          const status = await native({ kind: 'status' });
          return (
            status.data.jobs.length > 0 &&
            status.data.jobs.some((job) => job.kind === 'thumbnails') &&
            status.data.jobs
              .filter((job) => job.kind !== 'thumbnails')
              .every((job) => job.state === 'completed') &&
            status.data.jobs.every(
              (job) => !['queued', 'running', 'cancel_requested'].includes(job.state),
            )
          );
        },
        { timeout: 45000 },
      );
      await browser.waitUntil(async () =>
        browser.execute(() => {
          const trigger = document.querySelector('button[aria-label="Jobs"]');
          const id = trigger?.getAttribute('aria-describedby');
          return id && document.getElementById(id)?.textContent === '0 running jobs';
        }),
      );
    }
    await $('button[aria-label="Jobs"]').waitForDisplayed();
    await $('button[aria-label="Jobs"]').click();
    let observedProgress;
    await browser.waitUntil(
      async () => {
        const recent = await $('.jobs-dropdown .recent-jobs');
        if ((await recent.isExisting()) && (await recent.getAttribute('open')) === null)
          await recent.$('summary').click();
        // Read one DOM snapshot: the scan may finish between driver requests.
        observedProgress = await browser.execute(() => {
          const bar = document.querySelector(
            '.jobs-dropdown .job-row[data-job-kind="scan"] .job-progress [role="progressbar"]',
          );
          return bar && bar.getBoundingClientRect().height > 0
            ? { value: bar.getAttribute('aria-valuenow'), text: bar.getAttribute('aria-valuetext') }
            : null;
        });
        return Boolean(observedProgress);
      },
      { timeout: 15000, timeoutMsg: 'The real scan has no reachable job progress' },
    );
    assert.ok(
      ['Scanning · total files not yet known', 'Scan complete'].includes(observedProgress.text),
    );
    assert.equal(observedProgress.value, observedProgress.text === 'Scan complete' ? '100' : null);
    if (process.env.STILLHARBOUR_TEST_JOBS_COMPLETED === '1')
      assert.deepEqual(observedProgress, { value: '100', text: 'Scan complete' });
    observations.progressAtOpen = observedProgress;
    assert.equal(await $('header button[aria-label="Jobs"]').getAttribute('aria-expanded'), 'true');
    await browser.saveScreenshot('.context/screenshots/jobs-dropdown-after-save-desktop.png');
    await browser.setWindowSize(660, 560);
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
      true,
    );
    await browser.saveScreenshot('.context/screenshots/jobs-dropdown-after-save-minimum.png');
    await browser.setWindowSize(1180, 780);
    await browser.keys('Escape');
    const scanObservedAt = Date.now();
    let partial = false;
    await browser.waitUntil(
      async () => {
        const status = await native({ kind: 'status' });
        if (
          status.data.jobs.some((job) => job.kind !== 'thumbnails' && job.state === 'running') &&
          status.data.roots.some((root) => root.fileCount > 0)
        )
          partial = true;
        return (
          status.data.jobs.length > 0 &&
          // The scan publishes completion before its separate thumbnail job is
          // enqueued. Do not mistake that observable handoff gap for all done.
          status.data.jobs.some((job) => job.kind === 'thumbnails') &&
          status.data.jobs.every(
            (job) => !['queued', 'running', 'cancel_requested'].includes(job.state),
          )
        );
      },
      { timeout: 45000, interval: 25, timeoutMsg: 'Generated scan did not finish' },
    );
    observations.scanAfterSaveMs = Date.now() - scanObservedAt;
    observations.partialResultsWhileRunning = partial;
    const status = await native({ kind: 'status' });
    assert.equal(status.data.roots.length, 1);
    assert.ok(
      status.data.jobs
        .filter((job) => job.kind !== 'thumbnails')
        .every((job) => job.state === 'completed' && job.errors === 0),
    );
    const thumbnailJob = status.data.jobs.find((job) => job.kind === 'thumbnails');
    assert.equal(thumbnailJob.state, 'partial');
    assert.equal(
      thumbnailJob.indexed + thumbnailJob.skipped + thumbnailJob.errors,
      thumbnailJob.discovered,
    );
    assert.equal(thumbnailJob.errors, 2); // Deliberate malformed JPEG/HEIC fixtures remain explicit.
    assert.equal(status.data.roots[0].fileCount, 20);
    assert.equal(status.data.roots[0].mode, 'managed');
    assert.equal(status.data.roots[0].filter.fromDate, null);
    assert.equal(status.data.roots[0].filter.throughDate, null);
    assert.equal(status.data.roots[0].filter.minBytes, '0');
    assert.equal(status.data.roots[0].filter.maxBytes, '2000000000');
    assert.deepEqual(status.data.roots[0].filter.excludedFolders, ['Private']);
    assert.deepEqual(
      [...status.data.roots[0].filter.extensions].sort(),
      [...expectedExtensions].sort(),
    );
    assert.deepEqual(status.data.roots[0].filter.includeGlobs, []);
    assert.equal(status.data.metadataAvailable, true);
    await $('.asset', { strict: false }).waitForDisplayed();
    await browser.waitUntil(
      async () =>
        (
          await native({
            kind: 'query',
            query: {
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
            },
            cursor: null,
            limit: 1,
          })
        ).data.total === 20 && (await $$('.scan-progress')).length === 0,
    );
    await browser.waitUntil(
      async () =>
        browser.execute(() =>
          [...document.querySelectorAll('.photo-area img')].some((img) => img.naturalWidth > 0),
        ),
      { timeout: 20000, timeoutMsg: 'Native cached-image protocol did not render a preview' },
    );
    assert.equal((await $$('.error')).length, 0);
    await browser.waitUntil(async () => (await $$('.gallery img')).length >= 6, { timeout: 20000 });
    await browser.waitUntil(
      async () =>
        browser.execute(() => {
          const assets = [...document.querySelectorAll('.gallery .asset')];
          return (
            assets.length > 0 &&
            assets.every((asset) => {
              const image = asset.querySelector('.t-skel-content');
              const fallback = asset.querySelector('.preview-fallback');
              return (
                (image?.complete && image.naturalWidth > 0) ||
                (fallback && !fallback.textContent.includes('Preparing preview'))
              );
            })
          );
        }),
      { timeout: 20000, timeoutMsg: 'Visible clear renditions did not settle' },
    );
    await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_performance', { action: 'start', rows: null }),
    );
    let warmScroll;
    let warmTrace;
    try {
      warmScroll = await browser.execute(async () => {
        const viewport = document.querySelector('.asset-viewport');
        const image = document.querySelector('.gallery .t-skel-content');
        const asset = image.closest('.asset');
        const id = asset.dataset.assetId;
        const source = image.getAttribute('src');
        let retained = true;
        for (let step = 0; step < 10; step++) {
          viewport.scrollTop = step % 2 ? 20 : 24;
          await new Promise((resolve) => setTimeout(resolve, 25));
          const current = [...document.querySelectorAll('.gallery .asset')]
            .find((item) => item.dataset.assetId === id)
            ?.querySelector('.t-skel-content');
          retained &&=
            !!current &&
            current.getAttribute('src') === source &&
            current.complete &&
            current.naturalWidth > 32;
        }
        return { retained };
      });
      warmTrace = await browser.tauri.execute(async ({ core }) =>
        core.invoke('test_performance', { action: 'read', rows: null }),
      );
    } finally {
      await browser.tauri.execute(async ({ core }) =>
        core.invoke('test_performance', { action: 'stop', rows: null }),
      );
    }
    assert.equal(warmScroll.retained, true);
    assert.equal(
      warmTrace.trace.events.filter(
        (event) => event.metric === 'read' && event.endpoint === 'source',
      ).length,
      0,
    );
    await writeFile(
      `${evidence}/warm-scroll.json`,
      JSON.stringify({ warmScroll, trace: warmTrace.trace }),
    );
    await browser.saveScreenshot('.context/screenshots/m1-gallery.png');
    assert.ok((await $$('.gallery .asset-check')).length > 0);
    const selectedPhoto = await browser.execute(() => {
      const viewport = document.querySelector('.asset-viewport').getBoundingClientRect();
      const checkbox = [...document.querySelectorAll('.gallery .asset-check')].find((input) => {
        const bounds = input.getBoundingClientRect();
        return bounds.top > viewport.top + 36 && bounds.bottom < viewport.bottom;
      });
      checkbox.click();
      return checkbox.closest('.asset').dataset.assetId;
    });
    await browser.waitUntil(async () =>
      browser.execute(
        (id) =>
          document.querySelector(`[data-asset-id="${id}"]`)?.classList.contains('is-selected'),
        selectedPhoto,
      ),
    );
    await assertPhotoCheckboxOnTop(selectedPhoto);
    await browser.saveScreenshot('.context/screenshots/gallery-checkbox-on-top.png');
    await $('button=Clear selection').click();

    assert.ok((await $$('.date-heading')).length > 0);
    assert.equal((await $$('.gallery .asset-name')).length, 0);
    await $('.date-rail').waitForDisplayed();
    for (const [width, height, size] of [
      [1180, 780, 'desktop'],
      [660, 560, 'minimum'],
    ]) {
      await browser.setWindowSize(width, height);
      await browser.waitUntil(
        async () =>
          browser.execute(() => {
            const viewport = document.querySelector('.asset-viewport');
            return viewport.scrollWidth <= viewport.clientWidth + 1;
          }),
        { timeout: 3000, timeoutMsg: 'Gallery width did not settle after resize' },
      );
      const geometry = await browser.execute(() => {
        const rows = [...document.querySelectorAll('.asset-row.gallery')];
        const viewport = document.querySelector('.asset-viewport');
        return {
          noOverflow:
            document.documentElement.scrollWidth <= innerWidth &&
            viewport.scrollWidth <= viewport.clientWidth + 1,
          noHorizontalGaps: rows.every((row) => {
            const tiles = [...row.querySelectorAll('.asset')].map((tile) =>
              tile.getBoundingClientRect(),
            );
            return tiles
              .slice(1)
              .every((tile, index) => Math.abs(tile.left - tiles[index].right - 4) < 1);
          }),
          noVerticalGaps: rows
            .slice(1)
            .every(
              (row, index) =>
                Math.abs(
                  row.getBoundingClientRect().top - rows[index].getBoundingClientRect().bottom - 4,
                ) < 1,
            ),
        };
      });
      assert.deepEqual(geometry, {
        noOverflow: true,
        noHorizontalGaps: true,
        noVerticalGaps: true,
      });
      await browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        const destination = Math.min(200, viewport.scrollHeight - viewport.clientHeight);
        viewport.scrollTop = Math.abs(viewport.scrollTop - destination) < 1 ? 0 : destination;
      });

      const visibleDate = await browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport').getBoundingClientRect();
        const first = [...document.querySelectorAll('.asset-row')].find(
          (row) => row.getBoundingClientRect().bottom > viewport.top,
        );
        return first?.querySelector('.asset')?.getAttribute('data-date');
      });
      const expectedDate = visibleDate
        ? new Intl.DateTimeFormat(undefined, {
            month: 'short',
            year: 'numeric',
            timeZone: 'UTC',
          }).format(new Date(visibleDate.slice(0, 7) + '-01T00:00:00Z'))
        : 'Undated';
      await browser.waitUntil(async () =>
        browser.execute(
          (expected) => document.querySelector('.timeline-current')?.textContent === expected,
          expectedDate,
        ),
      );
      assert.equal(
        await browser.execute(() => {
          const year = document
            .querySelector('.date-rail [aria-current="date"]')
            .getBoundingClientRect();
          const bubble = document.querySelector('.timeline-current').getBoundingClientRect();
          return (
            bubble.right <= year.left ||
            bubble.left >= year.right ||
            bubble.bottom <= year.top ||
            bubble.top >= year.bottom
          );
        }),
        true,
      );
      const sticky = await browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport').getBoundingClientRect();
        const heading = [...document.querySelectorAll('.date-heading')].find((element) => {
          const parent = element.parentElement.getBoundingClientRect();
          return parent.top <= viewport.top && parent.bottom > viewport.top + 36;
        });
        return {
          aligned: [...document.querySelectorAll('.date-heading')].every(
            (element) => Math.abs(element.getBoundingClientRect().left - viewport.left) < 1,
          ),
          sticky:
            !!heading &&
            getComputedStyle(heading).position === 'sticky' &&
            Math.abs(heading.getBoundingClientRect().top - viewport.top) < 1,
          separateDays: [...document.querySelectorAll('.asset-row.gallery')].every(
            (row) =>
              new Set(
                [...row.querySelectorAll('.asset')].map((asset) => asset.dataset.date.slice(0, 10)),
              ).size <= 1,
          ),
          noFilters: [
            ...document.querySelectorAll('.photo-layers, .photo-layers img, .t-skel-skeleton'),
          ].every((element) => getComputedStyle(element).filter === 'none'),
        };
      });
      assert.deepEqual(sticky, {
        aligned: true,
        sticky: true,
        separateDays: true,
        noFilters: true,
      });
      await browser.saveScreenshot(`.context/screenshots/gallery-dates-sticky-${size}.png`);
      await browser.saveScreenshot(`.context/screenshots/photo-canvas-timeline-${size}.png`);
      await browser.execute(() => {
        document.querySelector('.asset-viewport').scrollTop = 0;
      });
      await browser.saveScreenshot(`.context/screenshots/photo-canvas-overview-${size}.png`);
      await browser.saveScreenshot(`.context/screenshots/balanced-gallery-${size}.png`);
    }
    // A short diagnostic scrollport lets the final sparse date reach the top;
    // the normal desktop/minimum screenshots above retain their actual layout.
    try {
      await browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        const first = document.querySelector('.photo-date-section');
        viewport.style.flex = '0 0 150px';
        viewport.style.height = '150px';
        viewport.scrollTop =
          Number.parseFloat(first.style.top) + Number.parseFloat(first.style.height) - 20;
      });
      await browser.waitUntil(async () =>
        browser.execute(() => document.querySelectorAll('.photo-date-section').length >= 2),
      );
      const handoff = await browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        const sections = [...document.querySelectorAll('.photo-date-section')];
        const [current, next] = sections
          .slice(0, 2)
          .map((section) => section.querySelector('.date-heading'));
        return {
          pushed: current.getBoundingClientRect().top < viewport.getBoundingClientRect().top,
          nextBelow: next.getBoundingClientRect().top > viewport.getBoundingClientRect().top,
          differentDays: current.dataset.day !== next.dataset.day,
          nextStart: Number.parseFloat(sections[1].style.top),
          nextDay: next.dataset.day,
        };
      });
      assert.equal(handoff.pushed && handoff.nextBelow && handoff.differentDays, true);
      await browser.execute((offset) => {
        document.querySelector('.asset-viewport').scrollTop = offset + 20;
      }, handoff.nextStart);
      await browser.waitUntil(async () =>
        browser.execute((day) => {
          const viewport = document.querySelector('.asset-viewport').getBoundingClientRect();
          const heading = [...document.querySelectorAll('.date-heading')].find(
            (element) => element.dataset.day === day,
          );
          return heading && Math.abs(heading.getBoundingClientRect().top - viewport.top) < 1;
        }, handoff.nextDay),
      );
    } finally {
      await browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        viewport.style.removeProperty('flex');
        viewport.style.removeProperty('height');
        viewport.scrollTop = 0;
      });
    }
    await browser.setWindowSize(1180, 780);
    await browser.execute(() => {
      const viewport = document.querySelector('.asset-viewport');
      viewport.scrollTop = viewport.scrollHeight;
    });
    await browser.waitUntil(
      async () =>
        browser.execute(() => {
          const slider = document.querySelector('.date-scrubber');
          const viewport = document.querySelector('.asset-viewport');
          return (
            viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop <= 1 &&
            Number(slider.value) === Number(slider.max)
          );
        }),
      { timeoutMsg: 'The timeline did not reach its exact end at the gallery bottom' },
    );
    assert.equal(
      await browser.execute(() => document.querySelector('.date-scrubber-track i').style.top),
      '100%',
    );
    await browser.saveScreenshot('.context/screenshots/timeline-end.png');
    await browser.execute(() => {
      document.querySelector('.asset-viewport').scrollTop = 0;
    });
    await browser.waitUntil(async () =>
      browser.execute(() => Number(document.querySelector('.date-scrubber').value) === 0),
    );
    const yearBuckets = (
      await native({
        kind: 'date_buckets',
        query: {
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
        },
      })
    ).data;
    const oldest = yearBuckets.at(-1);
    await $(`button[aria-label="Jump to ${oldest.year}, ${oldest.count} photos"]`).click();
    await browser.waitUntil(async () =>
      (await $('.timeline-current').getText()).includes(oldest.year),
    );
    assert.equal(
      await browser.execute(() => document.querySelector('.asset-viewport').scrollTop),
      0,
    );
    const newest = yearBuckets[0];
    await $(`button[aria-label="Jump to ${newest.year}, ${newest.count} photos"]`).click();
    await browser.waitUntil(async () =>
      browser.execute(
        (year) =>
          document.querySelector('.gallery .asset')?.getAttribute('data-date')?.startsWith(year),
        newest.year,
      ),
    );
    const monthIndex = yearBuckets
      .flatMap((bucket) => bucket.months)
      .findIndex((month) => month.period.startsWith(oldest.year));
    await browser.execute((index) => {
      const slider = document.querySelector('input[aria-label="Jump to year or month"]');
      slider.focus();
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(
        slider,
        String(index),
      );
      slider.dispatchEvent(new Event('input', { bubbles: true }));
      slider.dispatchEvent(new KeyboardEvent('keyup', { key: 'ArrowDown', bubbles: true }));
      slider.blur();
    }, monthIndex);
    await browser.waitUntil(async () =>
      browser.execute(
        (year) => document.querySelector('.gallery .asset')?.dataset.date?.startsWith(year),
        oldest.year,
      ),
    );
    await $(`button[aria-label="Jump to ${newest.year}, ${newest.count} photos"]`).click();
    await browser.waitUntil(async () =>
      browser.execute(
        (year) => document.querySelector('.gallery .asset')?.dataset.date?.startsWith(year),
        newest.year,
      ),
    );
    // The JPEG sensor is 600×450 with EXIF orientation 6. Its decoded image
    // and chronological tile must both be portrait, including existing catalogs.
    await $('input[aria-label="Search filenames"]').setValue('Generated-06.jpg');
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const assets = [...document.querySelectorAll('.gallery .asset')];
        return assets.length === 1 && assets[0].querySelector('.t-skel-content')?.naturalWidth > 32;
      }),
    );
    const oriented = await browser.execute(() => {
      const area = document.querySelector('.gallery .photo-area').getBoundingClientRect();
      const image = document.querySelector('.gallery .t-skel-content');
      return {
        width: area.width,
        height: area.height,
        naturalWidth: image.naturalWidth,
        naturalHeight: image.naturalHeight,
      };
    });
    assert.ok(oriented.naturalWidth < oriented.naturalHeight);
    assert.ok(Math.abs(oriented.width - oriented.height * 0.75) < 1, JSON.stringify(oriented));
    assert.ok(
      Math.abs(
        oriented.width - (oriented.height * oriented.naturalWidth) / oriented.naturalHeight,
      ) < 1,
      JSON.stringify(oriented),
    );
    await browser.saveScreenshot('.context/screenshots/gallery-oriented-portrait.png');
    await $('input[aria-label="Search filenames"]').setValue('');
    await browser.waitUntil(async () =>
      browser.execute(() => document.querySelectorAll('.gallery .asset').length > 1),
    );
    observations.scrolling = await browser.execute(async () => {
      const gaps = [];
      const viewport = document.querySelector('.asset-viewport');
      let previous = performance.now();
      await new Promise((resolve) => {
        let frameId;
        const deadline = setTimeout(() => {
          cancelAnimationFrame(frameId);
          resolve();
        }, 2000);
        function frame(now) {
          gaps.push(now - previous);
          previous = now;
          if (viewport) viewport.scrollTop = (gaps.length % 20) * 35;
          if (gaps.length < 60) frameId = requestAnimationFrame(frame);
          else {
            clearTimeout(deadline);
            resolve();
          }
        }
        frameId = requestAnimationFrame(frame);
      });
      if (viewport) viewport.scrollTop = 0;
      gaps.sort((a, b) => a - b);
      return {
        frames: gaps.length,
        p50FrameGapMs: gaps[Math.floor(gaps.length * 0.5)] ?? null,
        p95FrameGapMs: gaps[Math.floor(gaps.length * 0.95)] ?? null,
        maxFrameGapMs: gaps.at(-1) ?? null,
        longTaskApiSupported: PerformanceObserver.supportedEntryTypes.includes('longtask'),
      };
    });
  });

  it('scrolls the gallery from the timeline and completes a drag outside the narrow handle without a container outline', async () => {
    await browser.setWindowSize(1180, 780);
    await browser.execute(() => {
      const viewport = document.querySelector('.asset-viewport');
      viewport.scrollTop = 0;
      const rail = document.querySelector('.date-rail');
      rail.dispatchEvent(new WheelEvent('wheel', { deltaY: 180, bubbles: true, cancelable: true }));
    });
    await browser.waitUntil(async () =>
      browser.execute(() => document.querySelector('.asset-viewport').scrollTop > 0),
    );
    const styles = await browser.execute(() => {
      const viewport = document.querySelector('.asset-viewport');
      viewport.focus({ preventScroll: true });
      return {
        outline: getComputedStyle(viewport).outlineStyle,
        scrollbar: getComputedStyle(viewport).scrollbarWidth,
        webkitScrollbar: getComputedStyle(viewport, '::-webkit-scrollbar').display,
      };
    });
    assert.equal(styles.outline, 'none');
    assert.ok(styles.scrollbar === 'none' || styles.webkitScrollbar === 'none');
    const bounds = await browser.execute(() => {
      const rect = document.querySelector('.date-scrubber').getBoundingClientRect();
      return {
        x: Math.round(rect.left + rect.width / 2),
        top: Math.round(rect.top + 1),
        bottom: Math.round(rect.bottom - 1),
      };
    });
    await browser.performActions([
      {
        type: 'pointer',
        id: 'timeline-mouse',
        parameters: { pointerType: 'mouse' },
        actions: [
          { type: 'pointerMove', x: bounds.x, y: bounds.top, duration: 0 },
          { type: 'pointerDown', button: 0 },
          { type: 'pointerMove', x: bounds.x - 30, y: bounds.bottom, duration: 180 },
          { type: 'pointerUp', button: 0 },
        ],
      },
    ]);
    await browser.releaseActions();
    await browser.waitUntil(async () =>
      browser.execute(() =>
        document.querySelector('.gallery .asset')?.dataset.date?.startsWith('1964'),
      ),
    );
    // A programmatic year click leaves the scrubber focused, just like wheel
    // browsing after using the handle. Focus must not pin an obsolete draft.
    await browser.execute(() => {
      document.querySelector('.date-scrubber').focus({ preventScroll: true });
      document.querySelector('.timeline-year-labels button').click();
    });
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const slider = document.querySelector('.date-scrubber');
        return (
          document.querySelector('.gallery .asset')?.dataset.date?.startsWith('2026') &&
          slider.value === '0'
        );
      }),
    );
    await browser.saveScreenshot('.context/screenshots/gallery-scroll-fixed.png');
  });

  it('keeps Jobs in the header and Settings in the sidebar footer in expanded and collapsed navigation', async () => {
    for (const [width, height, label] of [
      [1180, 780, 'desktop'],
      [660, 560, 'minimum'],
    ]) {
      await browser.setWindowSize(width, height);
      for (const collapsed of [false, true]) {
        const toggle = await $(
          'button[aria-label="' + (collapsed ? 'Collapse navigation' : 'Expand navigation') + '"]',
        );
        if (await toggle.isExisting()) await toggle.click();
        const layout = await browser.execute(() => {
          const sidebar = document.querySelector('.sidebar').getBoundingClientRect();
          const footer = document.querySelector('.sidebar-footer').getBoundingClientRect();
          const jobs = document.querySelector('header button[aria-label="Jobs"]');
          const settings = document.querySelector('.sidebar-footer button[aria-label="Settings"]');
          const version = document.querySelector('.sidebar-version').getBoundingClientRect();
          const toggle = document.querySelector('button.sidebar-toggle');
          const toggleRect = toggle.getBoundingClientRect();
          const header = toggle.closest('header')?.getBoundingClientRect();
          return {
            toggleInHeader: Boolean(header) && !toggle.closest('.sidebar'),
            toggleAtTopEdge:
              Boolean(header) &&
              toggleRect.top >= header.top &&
              toggleRect.bottom <= header.bottom &&
              toggleRect.left >= sidebar.right &&
              toggleRect.left <= sidebar.right + 8,
            toggleBeforeTitle: toggle.nextElementSibling?.tagName === 'H1',
            inside: footer.top >= sidebar.top && footer.bottom <= sidebar.bottom + 1,
            jobsInHeader:
              Boolean(jobs) && !document.querySelector('.sidebar button[aria-label="Jobs"]'),
            aboveVersion: settings.getBoundingClientRect().bottom <= version.top,
            primaryContainsActions: Boolean(
              document.querySelector(
                '.sidebar > nav button[aria-label="Jobs"],.sidebar > nav button[aria-label="Settings"]',
              ),
            ),
            footerTop: footer.top,
          };
        });
        assert.equal(layout.toggleInHeader, true);
        assert.equal(layout.toggleAtTopEdge, true);
        assert.equal(layout.toggleBeforeTitle, true);
        assert.equal(layout.inside, true);
        assert.equal(layout.jobsInHeader, true);
        assert.equal(layout.aboveVersion, true);
        assert.equal(layout.primaryContainsActions, false);
        assert.ok(layout.footerTop > height / 2);
        await browser.waitUntil(async () =>
          browser.execute(() => {
            const viewport = document.querySelector('.asset-viewport');
            const last = viewport?.querySelector('.asset-row .asset:last-child');
            return (
              last &&
              last.getBoundingClientRect().right <= viewport.getBoundingClientRect().right + 1
            );
          }),
        );
        await browser.saveScreenshot(
          `.context/screenshots/sidebar-footer-${label}-${collapsed ? 'collapsed' : 'expanded'}.png`,
        );
      }
    }
    await $('button[aria-label="Jobs"]').click();
    await $('.jobs-dropdown').waitForDisplayed();
    await browser.keys('Escape');
    assert.equal(await $('button[aria-label="Jobs"]').isFocused(), true);
    await $('button[aria-label="Settings"]').click();
    await $('.settings-page').waitForDisplayed();
    await browser.execute(
      () =>
        (document.querySelector('main').scrollTop = document.querySelector('main').scrollHeight),
    );
    assert.equal(
      await browser.execute(() => {
        const footer = document.querySelector('.sidebar-footer').getBoundingClientRect();
        return footer.bottom <= innerHeight && footer.top > innerHeight / 2;
      }),
      true,
    );
    await $('button[aria-label="Photos"]').click();
    await browser.setWindowSize(1180, 780);
    if (await $('button[aria-label="Expand navigation"]').isExisting())
      await $('button[aria-label="Expand navigation"]').click();
  });
  it('puts query and display controls in a keyboard-accessible side panel', async () => {
    assert.equal((await $$('.filter-bar')).length, 0);
    assert.equal((await $$('.photo-toolbar button')).length, 3);
    assert.equal((await $$('.pagination')).length, 0);
    assert.equal((await $$('.view-switch')).length, 0);
    await browser.setWindowSize(1180, 780);
    await browser.execute(() => {
      document.querySelector('.asset-viewport').scrollTop = 0;
    });
    await browser.saveScreenshot('.context/screenshots/filter-canvas-desktop.png');
    await openFilters();
    await $('#library-filter-panel input', { strict: false }).waitForDisplayed();
    await browser.saveScreenshot('.context/screenshots/filter-panel-desktop.png');
    await browser.execute(() => {
      const tag = [...document.querySelectorAll('#library-filter-panel label')]
        .find((label) => label.textContent.trim() === 'Tag')
        .querySelector('input');
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(tag, 'Unsaved');
      tag.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await browser.keys('Escape');
    await browser.waitUntil(async () =>
      browser.execute(() => !document.querySelector('#library-filter-panel[open]')),
    );
    assert.equal(
      await browser.execute(() => document.activeElement?.getAttribute('aria-controls')),
      'library-filter-panel',
    );
    await browser.setWindowSize(660, 560);
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        const row = viewport?.querySelector('.asset-row');
        const tiles = row ? [...row.querySelectorAll('.asset')] : [];
        return (
          tiles.length &&
          tiles.at(-1).getBoundingClientRect().right <= viewport.getBoundingClientRect().right + 1
        );
      }),
    );
    await browser.saveScreenshot('.context/screenshots/filter-canvas-minimum.png');
    await openFilters();
    assert.equal(
      await browser.execute(
        () =>
          [...document.querySelectorAll('#library-filter-panel label')]
            .find((label) => label.textContent.trim() === 'Tag')
            .querySelector('input').value,
      ),
      '',
    );
    assert.equal(
      await browser.execute(
        () =>
          document.querySelector('#library-filter-panel').getBoundingClientRect().right <=
          innerWidth,
      ),
      true,
    );
    assert.equal(
      await browser.execute(
        () => getComputedStyle(document.querySelector('.filter-panel-body')).overflowY,
      ),
      'auto',
    );
    await browser.saveScreenshot('.context/screenshots/filter-panel-minimum.png');
    await browser.execute(() => {
      document.querySelector('.filter-panel-body').scrollTop =
        document.querySelector('.filter-panel-body').scrollHeight;
    });
    await browser.saveScreenshot('.context/screenshots/filter-panel-minimum-bottom.png');
    await $('button[aria-label="Close filters"]').click();
    await browser.setWindowSize(1180, 780);
  });
  it('preserves selection across views and starts no image requests in the text list', async () => {
    await galleryView('Uniform grid');
    await browser.waitUntil(async () =>
      browser.execute(
        () =>
          [...document.querySelectorAll('.asset')].filter(
            (asset) => asset.querySelector('img')?.naturalWidth > 0,
          ).length >= 2,
      ),
    );
    const chosen = await browser.execute(() =>
      [...document.querySelectorAll('.asset')]
        .filter((asset) => asset.querySelector('img')?.naturalWidth > 0)
        .slice(0, 2)
        .map((asset) => asset.dataset.assetId),
    );
    await $(`[data-asset-id="${chosen[0]}"] .asset-check`).click();
    await browser.waitUntil(async () =>
      browser.execute(
        (id) =>
          document.querySelector(`[data-asset-id="${id}"]`)?.classList.contains('is-selected'),
        chosen[0],
      ),
    );
    await assertPhotoCheckboxOnTop(chosen[0]);
    await browser.saveScreenshot('.context/screenshots/grid-checkbox-on-top.png');
    const name = await $(`[data-asset-id="${chosen[0]}"] .asset-check`).getAttribute('aria-label');
    await browser.execute(
      (id) =>
        document
          .querySelector(`[data-asset-id="${id}"] .asset-check`)
          .dispatchEvent(new MouseEvent('click', { bubbles: true, shiftKey: true })),
      chosen[1],
    );
    await browser.waitUntil(async () =>
      (await $('.results-heading p').getText()).includes('2 selected'),
    );
    await browser.execute(() => {
      window.__listRequests = [];
      window.addEventListener('stillharbour:test-library-request', (event) => {
        if (
          event.detail.kind === 'thumbnails' &&
          !document.querySelector('dialog[open]') &&
          document.querySelector('.asset-viewport')?.dataset.viewMode === 'list'
        )
          window.__listRequests.push(event.detail.keys.length);
      });
    });
    await galleryView('Text list');
    await $('.list', { strict: false }).waitForDisplayed();
    await browser.waitUntil(async () => browser.execute(() => window.__listRequests.length > 0));
    assert.equal((await $$('.asset-viewport img')).length, 0);
    assert.equal(
      await browser.execute(
        (label) =>
          [...document.querySelectorAll('.asset-check')].some(
            (input) => input.getAttribute('aria-label') === label && input.checked,
          ),
        name,
      ),
      true,
    );
    assert.deepEqual(await browser.execute(() => window.__listRequests), [0]);
    await browser.saveScreenshot('.context/screenshots/m1-text-list.png');
    await galleryView('Uniform grid');
    assert.equal(
      await browser.execute(
        (label) =>
          [...document.querySelectorAll('.asset-check')].some(
            (input) => input.getAttribute('aria-label') === label && input.checked,
          ),
        name,
      ),
      true,
    );
    await galleryView('Timeline');
  });
  it('saves catalog annotations and shows honest unsupported states', async () => {
    await $('button=Compare').waitForEnabled();
    await $('button=Compare').click();
    await $('.comparison-viewer[open]').waitForDisplayed();
    assert.equal((await $$('.comparison-images figure')).length, 2);
    assert.ok(
      (await $('.comparison-viewer .viewer-status').getText()).includes(
        'does not establish exact duplicates',
      ),
    );
    await browser.saveScreenshot('.context/screenshots/photo-canvas-compare.png');
    await $('.comparison-viewer .viewer-toolbar button').click();
    await $('button=Add tags').click();
    await $('input[name="batchTags"]').setValue('Reviewed');
    await $('button=Save tags to selection').click();
    await browser.waitUntil(async () => (await native({ kind: 'tags' })).data.includes('Reviewed'));
    await browser.saveScreenshot('.context/screenshots/photo-canvas-selection.png');
    await $('button=Clear selection').click();
    const opening = await browser.execute(() => {
      const viewport = document.querySelector('.asset-viewport');
      viewport.scrollTop = Math.min(150, viewport.scrollHeight - viewport.clientHeight);
      const bounds = viewport.getBoundingClientRect();
      const article = [...document.querySelectorAll('.gallery .asset')].find(
        (asset) =>
          asset.querySelector('img') &&
          asset.getBoundingClientRect().bottom > bounds.top &&
          asset.getBoundingClientRect().top < bounds.bottom,
      );
      const button = article.querySelector('.asset-open');
      button.focus({ preventScroll: true });
      const position = viewport.scrollTop;
      button.click();
      return { id: article.dataset.assetId, position };
    });
    await $('.photo-viewer[open]').waitForDisplayed();
    assert.equal(
      await browser.execute(() => {
        const canvas = document.querySelector('.viewer-canvas');
        canvas.focus({ preventScroll: true });
        return getComputedStyle(canvas).outlineStyle;
      }),
      'none',
    );

    await browser.waitUntil(async () =>
      browser.execute(
        () => document.querySelector('.photo-viewer .viewer-zoom-plane img')?.naturalWidth > 0,
      ),
    );
    await browser.saveScreenshot('.context/screenshots/photo-canvas-viewer-desktop.png');
    await browser.execute(() => {
      const range = document.querySelector('input[aria-label="Preview zoom"]');
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(range, '200');
      range.dispatchEvent(new Event('input', { bubbles: true }));
      range.dispatchEvent(new Event('change', { bubbles: true }));
    });
    assert.equal(await $('.viewer-zoom output').getText(), '200% of fit');
    await $('button=Fit').click();
    const initialName = await $('.viewer-toolbar h2').getText();
    await $('.viewer-next').waitForEnabled();
    await $('.viewer-next').click();
    await browser.waitUntil(async () => (await $('.viewer-toolbar h2').getText()) !== initialName);
    await $('.viewer-previous').waitForEnabled();
    await $('.viewer-previous').click();
    await browser.waitUntil(async () => (await $('.viewer-toolbar h2').getText()) === initialName);
    await $('button=Filmstrip').click();
    await $('.viewer-filmstrip').waitForDisplayed();
    observations.viewerOpening = opening;
  });
  it('saves viewer catalog notes, restores position and supports the minimum window', async () => {
    const opening = observations.viewerOpening;
    await $('button=Info').click();
    await $('.inspector').waitForDisplayed();
    console.info('Native viewer information opened');
    const favorite = await $('.inspector input[type="checkbox"]');
    await favorite.click();
    await browser.waitUntil(async () => favorite.isSelected());
    await browser.execute(() => {
      const select = document.querySelector('.inspector select');
      select.value = '4';
      select.dispatchEvent(new Event('change', { bubbles: true }));
    });
    await browser.waitUntil(
      async () => (await native({ kind: 'detail', asset_id: opening.id })).data.asset.rating === 4,
    );
    await $('.inspector input:not([type="checkbox"])').setValue('Generated fixture, 家族');
    await $('.inspector button[type="submit"]').click();
    await browser.waitUntil(async () => {
      const tags = await native({ kind: 'tags' });
      return tags.data.includes('家族');
    });
    console.info('Native viewer catalog notes saved');
    await browser.saveScreenshot('.context/screenshots/m1-inspector.png');
    await $('.inspector .section-heading button').click();
    await $('.viewer-back').click();
    await browser.waitUntil(async () =>
      browser.execute(() => !document.querySelector('.photo-viewer[open]')),
    );
    try {
      await browser.waitUntil(async () =>
        browser.execute(
          (position, id) =>
            document.querySelector('.asset-viewport').scrollTop === position &&
            document.activeElement?.closest('[data-asset-id]')?.dataset.assetId === id,
          opening.position,
          opening.id,
        ),
      );
    } catch (error) {
      console.info(
        'Native gallery return diagnostic',
        JSON.stringify(
          await browser.execute(() => ({
            top: document.querySelector('.asset-viewport').scrollTop,
            focus: document.activeElement?.className,
            id: document.activeElement?.closest('[data-asset-id]')?.dataset.assetId,
            viewportHeight: document.querySelector('.asset-viewport').clientHeight,
            scrollHeight: document.querySelector('.asset-viewport').scrollHeight,
          })),
        ),
        JSON.stringify(opening),
      );
      throw error;
    }
    assert.equal(
      await browser.execute(() => document.querySelector('.asset-viewport').scrollTop),
      opening.position,
    );
    assert.equal(
      await browser.execute(
        () => document.activeElement?.closest('[data-asset-id]')?.dataset.assetId,
      ),
      opening.id,
    );
    console.info('Native viewer restored gallery scroll and focus');
    await $('button[aria-label="Jobs"]').click();
    await $('.recent-jobs > summary').click();
    await $('.job-row[data-job-kind="scan"]').waitForDisplayed();
    const finalJob = (await native({ kind: 'status' })).data.jobs.find(
      (job) => job.kind !== 'thumbnails',
    );
    if (finalJob.state === 'completed') {
      assert.equal(
        await $('.job-row[data-job-kind="scan"] .job-progress [role="progressbar"]').getAttribute(
          'aria-valuenow',
        ),
        '100',
      );
      assert.equal(
        await $('.job-row[data-job-kind="scan"] .job-progress .help').getText(),
        'Scan complete',
      );
    } else {
      assert.equal(['partial', 'canceled', 'failed', 'interrupted'].includes(finalJob.state), true);
      assert.equal(
        (await $$('.job-row[data-job-kind="scan"] .job-progress [role="progressbar"]')).length,
        0,
      );
    }
    await (await $('.job-row[data-job-kind="scan"]')).$('button=Details').click();
    await $('.job-details').waitForDisplayed();
    await browser.saveScreenshot('.context/screenshots/jobs-dropdown-recent-desktop.png');
    await browser.keys('Escape');
    await $('.asset', { strict: false }).waitForDisplayed();
    await galleryView('Timeline');
    await browser.setWindowSize(660, 560);
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth <= window.innerWidth),
      true,
    );
    await browser.saveScreenshot('.context/screenshots/m1-gallery-minimum.png');
    await galleryView('Text list');
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth <= window.innerWidth),
      true,
    );
    await browser.saveScreenshot('.context/screenshots/m1-list-minimum.png');
    await $('button[aria-label="Collapse navigation"]').click();
    assert.equal(
      await browser.execute(() => document.querySelector('.sidebar').getBoundingClientRect().width),
      72,
    );
    await browser.saveScreenshot('.context/screenshots/photo-canvas-collapsed-minimum.png');
    const opener = await $('.asset-open', { strict: false });
    await opener.click();
    await $('.photo-viewer[open]').waitForDisplayed();
    await browser.saveScreenshot('.context/screenshots/photo-canvas-viewer-minimum.png');
    await $('button=Info').click();
    await browser.waitUntil(async () =>
      browser.execute(() => document.activeElement?.textContent === 'Photo details'),
    );
    await $('.inspector .section-heading button').click();
    await $('.viewer-back').click();
    await browser.waitUntil(async () =>
      browser.execute(() => document.activeElement?.classList.contains('asset-open')),
    );
    const verification = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.ok(verification.verifiedFiles >= 26);
    observations.verifiedSourceEntries = verification.verifiedFiles;
    await mkdir('.context/evidence', { recursive: true });
    await writeFile(
      '.context/evidence/m1-native-observations.json',
      JSON.stringify(observations, null, 2) + '\n',
    );
  });
  it('persists Settings defaults and reuses the native library-location workflow', async () => {
    await $('button[aria-label="Settings"]').click();
    await $('.settings-page h1').waitForDisplayed();
    const before = (await native({ kind: 'status' })).data.roots[0];
    const picker = await browser.tauri.mock('foundation_request');
    try {
      await picker.mockReturnValue({ kind: 'directory', data: null });
      await $('.settings-library-actions button:first-child').click();
      await browser.waitUntil(async () =>
        $('.settings-library-actions button:first-child').isEnabled(),
      );
      assert.equal((await native({ kind: 'status' })).data.roots[0].id, before.id);
      await picker.mockReturnValue({ kind: 'directory', data: generatedSelection });
      await $('.settings-library-actions button:first-child').click();
      await $('.settings-library-actions button:first-child').waitForEnabled();
      await browser.waitUntil(
        async () =>
          !(await native({ kind: 'status' })).data.jobs.some((job) =>
            ['queued', 'running', 'cancel_requested'].includes(job.state),
          ),
      );
    } finally {
      await picker.mockRestore();
    }
    assert.equal((await native({ kind: 'status' })).data.roots[0].id, before.id);
    await browser.setWindowSize(1180, 780);
    if (await $('button[aria-label="Expand navigation"]').isExisting())
      await $('button[aria-label="Expand navigation"]').click();
    await browser.execute(() => {
      document.querySelector('main').scrollTop = 0;
    });
    await browser.saveScreenshot('.context/screenshots/settings-desktop.png');
    await $('.settings-defaults-form .scan-options summary').click();
    await $('.settings-defaults-form input[name="minBytes"]').setValue('-1');
    assert.equal(
      await browser.execute(() =>
        document.querySelector('.settings-defaults-form input[name="minBytes"]').checkValidity(),
      ),
      false,
    );
    await $('button=Discard changes').click();
    await $('.settings-defaults-form .scan-options summary').click();
    assert.equal(await $('.settings-defaults-form input[name="minBytes"]').getValue(), '');
    assert.equal(
      await browser.execute(() =>
        document.querySelector('.settings-defaults-form input[name="minBytes"]').checkValidity(),
      ),
      true,
    );
    await $('.settings-defaults-form .scan-options summary').click();
    await $('button=Uniform grid').click();
    await changeSizeUnit('Default sort', 'added');
    await changeSizeUnit('Default library', before.id);
    await changeSizeUnit('Thumbnail size', 'large');
    await $('.settings-defaults-form input[role="switch"]', { strict: false }).click();
    await browser.execute(() => {
      const toggle = [...document.querySelectorAll('.settings-defaults-form label')]
        .find((label) => label.textContent.includes('Read only by default'))
        .querySelector('input');
      toggle.click();
    });
    await $('.settings-defaults-form .scan-options summary').click();
    await browser.execute(() => {
      const toggle = [...document.querySelectorAll('.settings-defaults-form label')]
        .find((label) => label.textContent.includes('Include hidden files'))
        .querySelector('input');
      toggle.click();
    });
    await $('button=Save defaults').click();
    await $('.settings-saved').waitForDisplayed();
    const saved = await browser.tauri.execute(async ({ core }) =>
      core.invoke('foundation_request', { request: { kind: 'get_preferences' } }),
    );
    assert.equal(saved.data.galleryView, 'grid');
    assert.equal(saved.data.thumbnailSize, 'large');
    assert.equal(saved.data.sort, 'added');
    assert.equal(saved.data.defaultRootId, before.id);
    assert.equal(saved.data.rootMode, 'referenced');
    assert.equal(saved.data.scanFilter.includeHiddenFiles, true);
    const invalidRootRejected = await browser.tauri.execute(async ({ core }, preferences) => {
      try {
        await core.invoke('foundation_request', {
          request: {
            kind: 'save_preferences',
            preferences: { ...preferences, defaultRootId: '00000000-0000-4000-8000-000000000000' },
          },
        });
        return false;
      } catch {
        return true;
      }
    }, saved.data);
    assert.equal(invalidRootRejected, true);
    const retained = await browser.tauri.execute(async ({ core }) =>
      core.invoke('foundation_request', { request: { kind: 'get_preferences' } }),
    );
    assert.deepEqual(retained.data, saved.data);

    const after = (await native({ kind: 'status' })).data.roots[0];
    assert.deepEqual(after.filter, before.filter);
    assert.equal(after.mode, before.mode);
    await browser.setWindowSize(660, 560);
    await browser.execute(() => {
      document.querySelector('main').scrollTop = 0;
    });
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
      true,
    );
    assert.equal(
      await browser.execute(() => getComputedStyle(document.querySelector('main')).overflowY),
      'auto',
    );
    const sidebarTop = await browser.execute(
      () => document.querySelector('.sidebar').getBoundingClientRect().top,
    );
    assert.equal(
      await browser.execute(
        () =>
          document.querySelector('main').scrollHeight > document.querySelector('main').clientHeight,
      ),
      true,
    );
    await browser.saveScreenshot('.context/screenshots/settings-minimum.png');
    await browser.execute(() =>
      document
        .querySelector('.settings-defaults-form .scan-options')
        .scrollIntoView({ block: 'start' }),
    );
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
      true,
    );
    await browser.saveScreenshot('.context/screenshots/settings-minimum-defaults.png');
    assert.equal(await browser.execute(() => document.querySelector('main').scrollTop > 0), true);
    assert.equal(
      await browser.execute(() => document.querySelector('.sidebar').getBoundingClientRect().top),
      sidebarTop,
    );
    assert.equal(await browser.execute(() => document.documentElement.scrollTop), 0);

    await $('button[aria-label="Photos"]').click();
    await $('.asset', { strict: false }).waitForDisplayed();
    assert.equal(await $('.asset-viewport').getAttribute('data-view-mode'), 'grid');
    assert.equal(await $('.asset-viewport').getAttribute('data-thumbnail-size'), 'large');
    await openFilters();
    assert.equal(await $('select[aria-label="Sort"]').getValue(), 'added');
    assert.equal(await $('.panel-query-fields > label:first-child select').getValue(), before.id);
    await $('button[aria-label="Close filters"]').click();
    await $('button[aria-label="Settings"]').click();
    const addPicker = await browser.tauri.mock('foundation_request');
    try {
      await addPicker.mockReturnValue({ kind: 'directory', data: generatedSelection });
      await $('button=Add photo folder').click();
      await $('.selected-folder.root-setup').waitForDisplayed();
      assert.equal(await $('.mode-field input').isSelected(), true);
      await $('.root-setup .scan-options summary').click();
      assert.equal(
        await browser.execute(
          () =>
            [...document.querySelectorAll('label')]
              .find((label) => label.textContent.includes('Include hidden files'))
              .querySelector('input').checked,
        ),
        true,
      );
      await $('button=Cancel').click();
    } finally {
      await addPicker.mockRestore();
    }
    const verification = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.equal(verification.verifiedFiles, 32);
  });
  it('edits saved-root scan settings inside Settings without replacing defaults or leaving the page', async () => {
    await browser.setWindowSize(1180, 780);
    if (await $('button[aria-label="Expand navigation"]').isExisting())
      await $('button[aria-label="Expand navigation"]').click();
    await $('button[aria-label="Photos"]').click();
    await openFilters();
    await (await $('#library-filter-panel')).$('button=Scan settings').click();
    await $('.settings-scan-group').waitForDisplayed();
    assert.equal((await $$('.settings-page')).length, 1);
    assert.equal((await $$('dialog[open]')).length, 0);
    assert.equal(await $('button[aria-label="Settings"]').getAttribute('aria-current'), 'page');
    await browser.waitUntil(async () =>
      browser.execute(() => document.activeElement?.id === 'scan-settings-title'),
    );
    const before = (await native({ kind: 'status' })).data.roots[0];
    const defaults = await browser.tauri.execute(async ({ core }) =>
      core.invoke('foundation_request', { request: { kind: 'get_preferences' } }),
    );
    assert.equal(await $('select[aria-label="Library to configure"]').getValue(), before.id);
    assert.equal(
      await $('.inline-scan-settings .mode-field input').isSelected(),
      before.mode === 'referenced',
    );
    await $('.inline-scan-settings .scan-options summary').click();
    await browser.execute(() =>
      document.querySelector('#scan-settings-title').scrollIntoView({ block: 'start' }),
    );
    await browser.saveScreenshot('.context/screenshots/settings-inline-scan-desktop.png');
    await browser.setWindowSize(660, 560);
    await browser.execute(() =>
      document.querySelector('#scan-settings-title').scrollIntoView({ block: 'start' }),
    );
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
      true,
    );
    assert.equal(
      await browser.execute(() => getComputedStyle(document.querySelector('main')).overflowY),
      'auto',
    );
    await browser.saveScreenshot('.context/screenshots/settings-inline-scan-minimum.png');
    await $('.inline-scan-settings .mode-field input').click();
    await $('.inline-scan-settings input[name="excludeGlobs"]').setValue(
      '**/never-match-native-fixture/**',
    );
    await $('button=Save & Rescan').click();
    await $('.settings-scan-group .settings-saved').waitForDisplayed();
    await browser.waitUntil(
      async () =>
        !(await native({ kind: 'status' })).data.jobs.some((job) =>
          ['queued', 'running', 'cancel_requested'].includes(job.state),
        ),
    );
    const after = (await native({ kind: 'status' })).data.roots[0];
    assert.equal(after.id, before.id);
    assert.equal(after.mode, before.mode === 'referenced' ? 'managed' : 'referenced');
    assert.ok(after.filter.excludeGlobs.includes('**/never-match-native-fixture/**'));
    assert.deepEqual(after.filter.extensions, before.filter.extensions);
    const unchangedDefaults = await browser.tauri.execute(async ({ core }) =>
      core.invoke('foundation_request', { request: { kind: 'get_preferences' } }),
    );
    assert.deepEqual(unchangedDefaults.data, defaults.data);
    assert.equal((await $$('.settings-page')).length, 1);
    assert.equal((await $$('form form')).length, 0);
    await $('button[aria-label="Photos"]').click();
    await $('button[aria-label="Settings"]').click();
    assert.equal(
      await $('.inline-scan-settings .mode-field input').isSelected(),
      after.mode === 'referenced',
    );
    await $('.inline-scan-settings .scan-options summary').click();
    assert.equal(
      await browser.execute(() =>
        document
          .querySelector('.inline-scan-settings [data-field="excludeGlobs"]')
          .textContent.includes('never-match-native-fixture'),
      ),
      true,
    );
    await $('button=Save & Rescan').waitForEnabled();
    await browser.execute(() =>
      document
        .querySelector('.inline-scan-settings .folder-actions')
        .scrollIntoView({ block: 'end' }),
    );
    await browser.saveScreenshot('.context/screenshots/settings-inline-scan-minimum-actions.png');
    const verified = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.equal(verified.verifiedFiles, 32);
    await browser.setWindowSize(1180, 780);
  });
  it('streams a bounded catalog window in both directions without a paging footer', async () => {
    await $('button[aria-label="Photos"]').click();
    await galleryView('Text list');
    const seeded = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_scroll_catalog'),
    );
    assert.equal(seeded, 960);
    await $('input[aria-label="Search filenames"]').setValue('Synthetic');
    await openFilters();
    await changeSizeUnit('Sort', 'name');
    await $('button=Apply filters').click();
    await browser.waitUntil(async () =>
      browser.execute(
        () => document.querySelector('.collection-main')?.dataset.feedCount === '240',
      ),
    );
    assert.equal((await $$('.pagination')).length, 0);
    for (let index = 0; index < 3; index++) {
      await browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        viewport.scrollTop = viewport.scrollHeight;
      });
      const expected = Math.min((index + 2) * 240, 720);
      await browser.waitUntil(async () =>
        browser.execute(
          (expected) =>
            Number(document.querySelector('.collection-main').dataset.feedCount) === expected,
          expected,
        ),
      );
    }
    // Reach the final tail after admitting each bounded page. Scrolling inside
    // an intermediate-tail assertion can load the next page before that tail
    // is observed, skipping the assertion's own checkpoint.
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        viewport.scrollTop = viewport.scrollHeight;
        return [...document.querySelectorAll('.list .asset')].some((asset) =>
          asset.textContent.includes('Synthetic 0959'),
        );
      }),
    );
    assert.equal(
      await browser.execute(
        () => Number(document.querySelector('.collection-main').dataset.feedCount) <= 720,
      ),
      true,
    );
    await browser.execute(() => {
      const viewport = document.querySelector('.asset-viewport');
      viewport.scrollTop = 0;
    });
    // Scrollbar-style movement: no wheel or key event may be required to fetch earlier rows.
    await browser.waitUntil(async () =>
      browser.execute(
        () =>
          document.querySelector('.collection-main').dataset.feedCount === '720' &&
          document.querySelector('.asset-viewport').scrollTop >
            document.querySelector('.asset-viewport').clientHeight * 3 &&
          [...document.querySelectorAll('.list .asset')]
            .find(
              (asset) =>
                asset.getBoundingClientRect().bottom >
                document.querySelector('.asset-viewport').getBoundingClientRect().top + 1,
            )
            ?.textContent.includes('Synthetic 0240'),
      ),
    );
    await browser.execute(() => {
      document.querySelector('.asset-viewport').scrollTop = 0;
    });
    await browser.waitUntil(async () =>
      browser.execute(() =>
        document.querySelector('.list .asset')?.textContent.includes('Synthetic 0000'),
      ),
    );
    assert.equal((await $$('.asset-viewport img')).length, 0);
    // The synthetic records share one exact timestamp. The timeline must still
    // move with the viewport and drag before release, without a second scrollbar.
    await openFilters();
    await changeSizeUnit('Sort', 'capture');
    await $('button=Apply filters').click();
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const slider = document.querySelector('.date-scrubber');
        return slider && !slider.disabled && slider.max === '1';
      }),
    );
    await browser.execute(() => {
      const viewport = document.querySelector('.asset-viewport');
      viewport.scrollTop = 0;
      document
        .querySelector('.date-rail')
        .dispatchEvent(new WheelEvent('wheel', { deltaY: 350, bubbles: true, cancelable: true }));
    });
    await browser.waitUntil(async () =>
      browser.execute(
        () =>
          document.querySelector('.asset-viewport').scrollTop > 0 &&
          Number(document.querySelector('.date-scrubber').value) > 0,
      ),
    );
    const bounds = await browser.execute(() => {
      const slider = document.querySelector('.date-scrubber');
      const rect = slider.getBoundingClientRect();
      return {
        x: Math.round(rect.left + rect.width / 2),
        start: Math.round(rect.top + rect.height * 0.25),
        end: Math.round(rect.top + rect.height * 0.65),
      };
    });
    await browser.performActions([
      {
        type: 'pointer',
        id: 'dense-timeline',
        parameters: { pointerType: 'mouse' },
        actions: [
          { type: 'pointerMove', x: bounds.x, y: bounds.start, duration: 0 },
          { type: 'pointerDown', button: 0 },
          { type: 'pointerMove', x: bounds.x, y: bounds.end, duration: 120 },
        ],
      },
    ]);
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        return (
          viewport.scrollTop > viewport.clientHeight &&
          Number(document.querySelector('.date-scrubber').value) > 0.5
        );
      }),
    );
    await browser.performActions([
      {
        type: 'pointer',
        id: 'dense-timeline',
        parameters: { pointerType: 'mouse' },
        actions: [{ type: 'pointerUp', button: 0 }],
      },
    ]);
    await browser.releaseActions();
    assert.equal(
      await browser.execute(() => {
        const viewport = document.querySelector('.asset-viewport');
        const style = getComputedStyle(viewport);
        return (
          style.scrollbarWidth === 'none' ||
          getComputedStyle(viewport, '::-webkit-scrollbar').display === 'none'
        );
      }),
      true,
    );
    assert.ok(
      await browser.execute(
        () => Number(document.querySelector('.collection-main').dataset.feedCount) <= 720,
      ),
    );
    await browser.saveScreenshot('.context/screenshots/timeline-only-scroll.png');
    const verified = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.equal(verified.verifiedFiles, 32);
  });
  it('configures text columns and sorts the native catalog without requesting images', async () => {
    await browser.setWindowSize(1180, 780);
    await $('input[aria-label="Search filenames"]').setValue('Generated');
    await browser.waitUntil(async () =>
      browser.execute(() =>
        document.querySelector('.list .asset-open')?.textContent.includes('Generated'),
      ),
    );
    await browser.execute(() => {
      window.__textListQueries = [];
      window.__textListPreviewRequests = [];
      window.addEventListener('stillharbour:test-library-request', (event) => {
        if (event.detail.kind === 'query') window.__textListQueries.push(event.detail.query);
        if (
          event.detail.kind === 'thumbnails' &&
          document.querySelector('.asset-viewport')?.dataset.viewMode === 'list' &&
          !document.querySelector('dialog[open]')
        )
          window.__textListPreviewRequests.push(event.detail.keys.length);
      });
    });
    assert.deepEqual(
      await browser.execute(() =>
        [...document.querySelectorAll('.text-list-head [role="columnheader"]')].map(
          (cell) => cell.getAttribute('aria-label') || cell.textContent.trim(),
        ),
      ),
      ['Selection', 'Name', 'Modified Date', 'Size', 'Dimensions', 'Folder', 'Status'],
    );
    assert.equal(
      await browser.execute(() =>
        document.querySelector('.text-list-row')?.textContent.includes('filesystem modified'),
      ),
      false,
    );
    await changeSizeUnit('Text list sort', 'modified');
    await browser.waitUntil(async () =>
      browser.execute(() => window.__textListQueries.at(-1)?.sort === 'modified'),
    );
    const query = await browser.execute(() => window.__textListQueries.at(-1));
    const packed = (await native({ kind: 'query', query, cursor: null, limit: 240 })).data;
    assert.equal(packed.format, 1);
    const expected = packed.rows.map((row) => ({ id: row[0], modifiedAt: row[12] }));
    assert.ok(expected.length > 0);
    await browser.waitUntil(async () =>
      browser.execute(
        (id) => document.querySelector('.text-list-row')?.dataset.assetId === id,
        expected[0].id,
      ),
    );
    assert.equal(await $('.text-list-head [aria-sort="descending"]').getText(), 'Modified Date');
    assert.equal(
      await $(`[data-asset-id="${expected[0].id}"] [data-column="modified"]`).getAttribute('title'),
      expected[0].modifiedAt || 'Unknown',
    );
    await browser.saveScreenshot('.context/screenshots/text-columns-desktop.png');
    await $('.text-list-head [data-sort="name"]').click();
    await browser.waitUntil(async () =>
      browser.execute(() => window.__textListQueries.at(-1)?.sort === 'name'),
    );
    assert.equal(await $('select[aria-label="Text list sort"]').getValue(), 'name');
    assert.equal(await $('.text-list-head [aria-sort="ascending"]').getText(), 'Name');
    await $('.text-list-columns summary').click();
    await $('input[name="text-column-modified"]').click();
    await $('input[name="text-column-capture"]').click();
    await $('input[name="text-column-source"]').click();
    assert.equal((await $$('.text-list-head [data-sort="modified"]')).length, 0);
    assert.equal((await $$('.text-list-head [data-sort="capture"]')).length, 1);
    assert.equal((await $$('.text-list-row [data-column="source"]')).length > 0, true);
    await browser.saveScreenshot('.context/screenshots/text-columns-menu-desktop.png');
    await browser.keys('Escape');
    assert.equal(await $('.text-list-columns summary').isFocused(), true);
    assert.equal(await $('.text-list-columns').getAttribute('open'), null);
    await galleryView('Uniform grid');
    await galleryView('Text list');
    assert.equal((await $$('.text-list-head [data-sort="capture"]')).length, 1);
    assert.equal((await $$('.text-list-head [data-sort="modified"]')).length, 0);
    await browser.setWindowSize(660, 560);
    await browser.waitUntil(async () =>
      browser.execute(() => {
        const pane = document.querySelector('.text-list-scroll');
        return pane.clientWidth < 660 && pane.scrollWidth > pane.clientWidth;
      }),
    );
    const aligned = await browser.execute(() => {
      const pane = document.querySelector('.text-list-scroll');
      pane.scrollLeft = pane.scrollWidth;
      const table = document.querySelector('.text-list-table');
      const heads = [...document.querySelectorAll('.text-list-head [role="columnheader"]')];
      const cells = [...document.querySelector('.text-list-row').children];
      return {
        noPageOverflow: document.documentElement.scrollWidth <= innerWidth,
        sameWidth:
          Math.abs(
            table.getBoundingClientRect().width -
              document.querySelector('.asset-viewport').getBoundingClientRect().width,
          ) < 2,
        offsets: heads.map((head, index) =>
          Math.abs(head.getBoundingClientRect().left - cells[index].getBoundingClientRect().left),
        ),
      };
    });
    assert.equal(aligned.noPageOverflow, true);
    assert.equal(aligned.sameWidth, true);
    assert.ok(aligned.offsets.every((offset) => offset < 2));
    await browser.saveScreenshot('.context/screenshots/text-columns-minimum.png');
    await $('.text-list-columns summary').click();
    await browser.saveScreenshot('.context/screenshots/text-columns-menu-minimum.png');
    await browser.execute(() =>
      document.querySelector('.text-list-columns button').scrollIntoView({ block: 'nearest' }),
    );
    await $('button=Reset columns').click();
    await browser.keys('Escape');
    assert.equal((await $$('.text-list-head [data-sort="modified"]')).length, 1);
    assert.equal((await $$('.asset-viewport img')).length, 0);
    assert.equal(
      await browser.execute(() => window.__textListPreviewRequests.every((size) => size === 0)),
      true,
    );
    const verified = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.equal(verified.verifiedFiles, 32);
    await browser.setWindowSize(1180, 780);
  });
  it('browses indexed folders as an explorer and opens photos or file details in place', async () => {
    await browser.setWindowSize(1180, 780);
    const before = (await native({ kind: 'status' })).data.roots[0];
    await native({
      kind: 'scan',
      root_id: before.id,
      filter: { ...before.filter, extensions: [...new Set([...before.filter.extensions, 'xmp'])] },
    });
    await browser.waitUntil(
      async () =>
        !(await native({ kind: 'status' })).data.jobs.some((job) =>
          ['queued', 'running', 'cancel_requested'].includes(job.state),
        ),
    );
    const jobs = (await native({ kind: 'status' })).data.jobs.length;
    await $('button[aria-label="Folders"]').click();
    await $('button[aria-label="Open folder ' + before.label + '"]').waitForDisplayed();
    await explorerLayoutReady();
    await browser.saveScreenshot('.context/screenshots/folder-explorer-libraries.png');
    await $('button[aria-label="Open folder ' + before.label + '"]').click();
    await $('button[aria-label="Open folder Coast"]').waitForDisplayed();
    assert.equal((await $$('button[aria-label="Open Generated-00.jpg"]')).length, 0);
    await browser.saveScreenshot('.context/screenshots/folder-explorer-root.png');
    const rootPage = (
      await native({
        kind: 'browse_directory',
        root_id: before.id,
        directory_id: 'd',
        name: '',
        cursor: null,
        limit: 240,
      })
    ).data;
    const coast = rootPage.entries.find(
      (entry) => entry.kind === 'folder' && entry.directory.name === 'Coast',
    ).directory;
    await browser.execute(() => {
      window.folderRequests = [];
      window.folderImageRequestsInDetails = 0;
      window.folderListener = (event) => {
        window.folderRequests.push(event.detail);
        if (
          event.detail.kind === 'thumbnails' &&
          event.detail.keys.length &&
          document.querySelector('.explorer-viewport.details') &&
          !document.querySelector('.photo-viewer[open]')
        )
          window.folderImageRequestsInDetails++;
      };
      addEventListener('stillharbour:test-library-request', window.folderListener);
    });
    await $('button[aria-label="Open folder Coast"]').click();
    await $('button[aria-label="Open Generated-00.jpg"]').waitForDisplayed();
    await explorerLayoutReady();
    await browser.waitUntil(async () =>
      browser.execute(() =>
        [
          ...document.querySelectorAll(
            '.explorer-viewport button[aria-label="Open Generated-00.jpg"] img',
          ),
        ].some((image) => image.complete && image.naturalWidth > 0),
      ),
    );
    await browser.saveScreenshot('.context/screenshots/folder-explorer-coast-desktop.png');
    await $('button[aria-label="Open Generated-00.jpg"]').click();
    await $('.photo-viewer[open]').waitForDisplayed();
    await browser.waitUntil(async () =>
      browser.execute(
        (id) =>
          window.folderRequests.some(
            (request) => request.kind === 'neighbors' && request.query.directoryId === id,
          ),
        coast.id,
      ),
    );
    await $('.viewer-back').click();
    await browser.waitUntil(async () =>
      browser.execute(
        () => document.activeElement?.getAttribute('aria-label') === 'Open Generated-00.jpg',
      ),
    );
    await $('button=Details').click();
    await $('.explorer-viewport.details').waitForDisplayed();
    await browser.waitUntil(async () =>
      browser.execute(() =>
        window.folderRequests.some(
          (request) => request.kind === 'thumbnails' && !request.keys.length,
        ),
      ),
    );
    assert.equal((await $$('.explorer-viewport img')).length, 0);
    assert.equal(await browser.execute(() => window.folderImageRequestsInDetails), 0);
    await $('button[aria-label="Open Generated-00.xmp"]').click();
    await $('.file-details[open]').waitForDisplayed();
    assert.ok((await $('.file-details').getText()).includes('does not open external applications'));
    await $('.file-details button').click();
    await browser.setWindowSize(660, 560);
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
      true,
    );
    await browser.saveScreenshot('.context/screenshots/folder-explorer-coast-minimum-details.png');
    await $('button=Icons').click();
    await explorerLayoutReady();
    await browser.waitUntil(async () =>
      browser.execute(() =>
        [
          ...document.querySelectorAll(
            '.explorer-viewport button[aria-label="Open Generated-00.jpg"] img',
          ),
        ].some((image) => image.complete && image.naturalWidth > 0),
      ),
    );
    await browser.saveScreenshot('.context/screenshots/folder-explorer-coast-minimum.png');
    await $('button=Up').click();
    await $('button[aria-label="Open folder Coast"]').waitForDisplayed();
    await $('button=Back').click();
    await $('button[aria-label="Open Generated-00.jpg"]').waitForDisplayed();
    assert.equal((await native({ kind: 'status' })).data.jobs.length, jobs);
    const verified = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.equal(verified.verifiedFiles, 32);
    await browser.execute(() =>
      removeEventListener('stillharbour:test-library-request', window.folderListener),
    );
    await browser.setWindowSize(1180, 780);
  });
  it('clears previews and confirms library removal without changing generated originals', async () => {
    await $('button[aria-label="Settings"]').click();
    const before = (await native({ kind: 'status' })).data;
    const root = before.roots[0];
    await $('button[aria-label="Remove library ' + root.label + '"]').click();
    await $('button=Remove library and its previews').waitForEnabled();
    assert.equal((await native({ kind: 'status' })).data.roots.length, before.roots.length);
    assert.ok(
      (await $('.maintenance-review').getText()).includes('catalog favorites, ratings, tags'),
    );
    await browser.keys('Escape');
    try {
      await browser.waitUntil(async () =>
        browser.execute(() => document.activeElement?.textContent === 'Remove library'),
      );
    } catch (error) {
      const focus = await browser.execute(() => ({
        reviewOpen: Boolean(document.querySelector('.maintenance-review')),
        jobsOpen: Boolean(document.querySelector('.jobs-dropdown')),
        openerFocused:
          document.activeElement?.matches('button[aria-label^="Remove library "]') ?? false,
        focusInReview: Boolean(document.activeElement?.closest('.maintenance-review')),
        focusedTag: document.activeElement?.tagName,
        visibility: document.visibilityState,
      }));
      await writeFile(
        '.context/evidence/jobs-dropdown-maintenance-focus.json',
        JSON.stringify(focus),
      );
      throw error;
    }
    await $('button[aria-label="Remove library ' + root.label + '"]').click();
    await $('button=Remove library and its previews').waitForEnabled();
    await $('button=Cancel').click();
    const query = {
      rootId: root.id,
      folder: '',
      name: 'Generated-00.jpg',
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
    const packed = (await native({ kind: 'query', query, cursor: null, limit: 1 })).data;
    assert.equal(packed.format, 1);
    const assetId = packed.rows[0][0];
    const notesBefore = (await native({ kind: 'detail', asset_id: assetId })).data.asset;
    await $('button=Clear preview cache').click();
    await $('button=Clear all cached previews').waitForEnabled();
    assert.ok(
      Number((await $('.maintenance-review').getText()).match(/(\d+) cached previews/)[1]) > 0,
    );
    await $('button=Clear all cached previews').click();
    await $('.maintenance-review [role="status"]').waitForDisplayed();
    const cleared = await native({
      kind: 'prepare_maintenance',
      target: { kind: 'preview_cache' },
    });
    assert.equal(cleared.data.cachedPreviews, 0);
    const notesAfter = (await native({ kind: 'detail', asset_id: assetId })).data.asset;
    for (const field of ['favorite', 'rating', 'tags'])
      assert.deepEqual(notesAfter[field], notesBefore[field]);
    assert.equal((await native({ kind: 'status' })).data.roots.length, before.roots.length);
    await $('button=Done').click();
    await $('button[aria-label="Remove library ' + root.label + '"]').click();
    await $('button=Remove library and its previews').waitForEnabled();
    await browser.setWindowSize(1180, 780);
    if (await $('button[aria-label="Expand navigation"]').isExisting())
      await $('button[aria-label="Expand navigation"]').click();
    await browser.execute(() =>
      document.querySelector('.maintenance-review').scrollIntoView({ block: 'center' }),
    );
    await browser.saveScreenshot('.context/screenshots/remove-library-review-desktop.png');
    await browser.setWindowSize(660, 560);
    await browser.execute(() =>
      document.querySelector('.maintenance-review').scrollIntoView({ block: 'start' }),
    );
    assert.equal(
      await browser.execute(() => document.documentElement.scrollWidth <= innerWidth),
      true,
    );
    await browser.saveScreenshot('.context/screenshots/remove-library-review-minimum.png');
    assert.ok((await $('.maintenance-review').getText()).includes(root.displayPath));
    await browser.execute(() =>
      document.querySelector('.maintenance-review form').scrollIntoView({ block: 'end' }),
    );
    await browser.saveScreenshot('.context/screenshots/remove-library-review-minimum-actions.png');
    await $('button=Remove library and its previews').click();
    await $('.maintenance-review [role="status"]').waitForDisplayed();
    assert.equal((await native({ kind: 'status' })).data.roots.length, 0);
    const preferences = await browser.tauri.execute(async ({ core }) =>
      core.invoke('foundation_request', { request: { kind: 'get_preferences' } }),
    );
    assert.equal(preferences.data.defaultRootId, null);
    await $('button=Done').click();
    const verification = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.equal(verification.verifiedFiles, 32);
    await $('button=Library').click();
    await $('button=Choose a folder').waitForDisplayed();
    await browser.saveScreenshot('.context/screenshots/remove-library-empty.png');
  });
});
