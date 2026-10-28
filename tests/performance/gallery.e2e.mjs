import assert from 'node:assert/strict';
import { browser, $, $$ } from '@wdio/globals';
import { mkdir, writeFile } from 'node:fs/promises';
import { execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import { randomUUID } from 'node:crypto';

assert.equal(
  process.platform,
  'darwin',
  'The P0 native fixture adapter is macOS-only; Windows is not measured.',
);
const rows = Number(process.env.STILLHARBOUR_PERF_ROWS ?? '10000');
const phase = process.env.STILLHARBOUR_PERF_PHASE ?? 'P0';
assert.ok(['P0', 'P1'].includes(phase));
const steps = Number(process.env.STILLHARBOUR_PERF_STEPS ?? '120');
assert.ok([6, 120].includes(steps));
assert.ok([10000, 100000, 1000000].includes(rows));
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function native(request) {
  return browser.tauri.execute(
    async ({ core }, request) => core.invoke('library_request', { request }),
    request,
  );
}
async function diagnostic(action, rows = null) {
  return browser.tauri.execute(
    async ({ core }, action, rows) => core.invoke('test_performance', { action, rows }),
    action,
    rows,
  );
}
async function view(label) {
  await $('button[aria-controls="library-filter-panel"]').click();
  await $('#library-filter-panel[open]').waitForDisplayed();
  await (await $('#library-filter-panel')).$(`button=${label}`).click();
  await $('button=Apply filters').click();
}
function manifest(root) {
  return JSON.parse(
    execFileSync('python3', ['scripts/performance/manifest.py', root], {
      encoding: 'utf8',
      maxBuffer: 2 * 1024 * 1024,
    }),
  );
}

describe(`${phase} native diagnostic route — generated sources and synthetic observations only`, () => {
  it(`observes ${rows} synthetic records without altering source fixtures`, async () => {
    await browser.setTimeout({ script: 180000 });
    const run = randomUUID();
    const output = `.context/performance/native-${run}-${rows}`;
    await mkdir('.context/performance', { recursive: true });
    await mkdir(output, { recursive: false });
    const routeStarted = Date.now();
    const progress = (stage) =>
      writeFile(
        `${output}/progress.json`,
        JSON.stringify({ phase, stage, elapsedMs: Date.now() - routeStarted }),
      );
    await progress('generated_fixture');
    await $('button.primary').waitForEnabled();
    const selection = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_library_fixture', { verify: false }),
    );
    const { fixtureRoot } = await diagnostic('fixture_location');
    const before = manifest(fixtureRoot);
    await writeFile(`${output}/manifest-before.json`, JSON.stringify(before, null, 2));
    const { pid } = await diagnostic('start');
    const monitor = spawn('python3', ['scripts/performance/process_stats.py', String(pid), '900'], {
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    let memory = '';
    monitor.stdout.setEncoding('utf8');
    monitor.stdout.on('data', (chunk) => {
      if (memory.length + chunk.length <= 4 * 1024 * 1024) memory += chunk;
    });
    await browser.execute(() => window.__stillharbourPerf.start());
    let failure = null;
    const measurements = { phase };
    try {
      const picker = await browser.tauri.mock('foundation_request');
      try {
        await picker.mockReturnValue({ kind: 'directory', data: selection });
        await $('button.primary').click();
        await $('.root-setup input[required]').setValue('P0 generated sources');
        const started = Date.now();
        await $('.folder-actions .primary').click();
        await $('.asset-viewport').waitForDisplayed();
        await browser.waitUntil(
          async () =>
            browser.execute(() =>
              [...document.querySelectorAll('.asset-viewport img')].some(
                (image) => image.complete && image.naturalWidth > 0,
              ),
            ),
          { timeout: 45000 },
        );
        measurements.freshFixtureSaveScanToFirstDecodedImageMs = Date.now() - started;
      } finally {
        await picker.mockRestore();
      }
      await browser.waitUntil(
        async () =>
          !(await native({ kind: 'status' })).data.jobs.some((job) =>
            ['running', 'cancel_requested'].includes(job.state),
          ),
        { timeout: 45000 },
      );
      measurements.freshCacheTrace = {
        backend: await diagnostic('read'),
        frontend: await browser.execute(() => window.__stillharbourPerf.read()),
      };
      // The pinned embedded evaluator has its own 30s deadline, independent of
      // WebDriver setTimeout. Keep its invocation short and poll one owned promise.
      await progress('seed_started');
      await browser.tauri.execute(({ core }, rows) => {
        window.__p0Seed = { state: 'pending' };
        core.invoke('test_performance', { action: 'seed', rows }).then(
          (value) => {
            window.__p0Seed = { state: 'done', value };
          },
          () => {
            window.__p0Seed = { state: 'failed' };
          },
        );
        return true;
      }, rows);
      await browser.waitUntil(
        async () => browser.execute(() => window.__p0Seed.state !== 'pending'),
        { timeout: rows === 1000000 ? 600000 : 180000, interval: 250 },
      );
      const seeded = await browser.execute(() => window.__p0Seed.value);
      assert.ok(seeded, 'Owned seed operation failed');
      await progress('seed_finished');
      measurements.fixture = {
        rows: seeded.rows,
        synthetic: true,
        seed: seeded.seed,
        realGeneratedSourceEntries: before.entries.length,
      };
      await browser.waitUntil(
        async () =>
          browser.execute(
            () => Number(document.querySelector('.collection-main')?.dataset.feedCount) >= 240,
          ),
        { timeout: 45000 },
      );
      await diagnostic('start');
      await browser.execute(() => window.__stillharbourPerf.start());
      const started = Date.now();
      // 120 alternating steps, at least 60 seconds. This is a scripted scrollTop route,
      // not evidence about physical wheel input or actual presented frames.
      for (let step = 0; step < steps; step++) {
        await browser.execute((step) => {
          const viewport = document.querySelector('.asset-viewport');
          viewport.scrollTop += step % 40 < 25 ? 330 : -250;
        }, step);
        await pause(500);
      }
      measurements.scrollRouteSteps = steps;
      measurements.scrollRouteMs = Date.now() - started;
      await progress('scroll_finished');
      measurements.randomJumpCompletionsMs = [];
      for (const fraction of [0.8, 0.2, 0.65, 0.4, 0.9, 0.1, 0.5, 0.3, 0.7, 0]) {
        const started = Date.now();
        const target = await browser.execute((fraction) => {
          const buttons = [...document.querySelectorAll('.date-rail button')].filter(
            (button) => button.getAttribute('aria-current') !== 'date',
          );
          const button =
            buttons[Math.min(buttons.length - 1, Math.floor(buttons.length * fraction))];
          const year = button?.textContent;
          button?.click();
          return year;
        }, fraction);
        assert.ok(target);
        await browser.waitUntil(
          async () =>
            browser.execute((target) => {
              const date = document.querySelector('.asset[data-date]')?.getAttribute('data-date');
              return (
                date !== undefined &&
                (target === 'Undated' ? date === '' : date?.startsWith(target))
              );
            }, target),
          { timeout: 45000 },
        );
        measurements.randomJumpCompletionsMs.push(Date.now() - started);
      }
      measurements.resizeCommandMs = [];
      for (const [width, height] of [
        [660, 560],
        [1180, 780],
        [900, 700],
        [1180, 780],
      ]) {
        const started = Date.now();
        await browser.setWindowSize(width, height);
        await pause(100);
        measurements.resizeCommandMs.push(Date.now() - started);
      }
      await view('Text list');
      await $('.text-list-table').waitForDisplayed();
      assert.equal((await $$('.asset-viewport img')).length, 0);
      const root = (await native({ kind: 'status' })).data.roots.find(
        (root) => root.label === 'P0 generated sources',
      );
      const scan = (await native({ kind: 'scan', root_id: root.id, filter: root.filter })).data;
      measurements.scanOverlapSamples = [];
      for (let index = 0; index < 5; index++) {
        const running = (await native({ kind: 'status' })).data.jobs.some(
          (job) => job.id === scan.id && job.state === 'running',
        );
        const started = Date.now();
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
          limit: 240,
        });
        measurements.scanOverlapSamples.push({
          runningAtQueryStart: running,
          queryMs: Date.now() - started,
        });
      }
      await browser.waitUntil(
        async () =>
          !(await native({ kind: 'status' })).data.jobs.some(
            (job) => job.id === scan.id && ['running', 'cancel_requested'].includes(job.state),
          ),
      );
      measurements.backend = await diagnostic('read');
      measurements.frontend = await browser.execute(() => window.__stillharbourPerf.read());
      assert.ok(
        measurements.frontend.events
          .filter((event) => event.metric === 'resident_summaries')
          .every((event) => event.value <= 720),
      );
      await diagnostic('start');
      await browser.execute(() => window.__stillharbourPerf.start());
      await $('input[aria-label="Search filenames"]').setValue('Generated');
      await browser.waitUntil(async () =>
        browser.execute(() =>
          document.querySelector('.asset-open')?.textContent.includes('Generated'),
        ),
      );
      const checkbox = (await $$('.asset-check'))[0];
      await checkbox.click();
      assert.equal(await checkbox.isSelected(), true);
      await $('button=Clear selection').click();
      await (await $$('.asset-open'))[0].click();
      await $('.photo-viewer[open]').waitForDisplayed();
      await $('button[aria-label="Next photo"]').waitForEnabled();
      await $('button[aria-label="Next photo"]').click();
      await $('button=Back to gallery').click();
      await view('Uniform grid');
      await view('Text list');
      measurements.viewerSelectionTrace = {
        backend: await diagnostic('read'),
        frontend: await browser.execute(() => window.__stillharbourPerf.read()),
      };
      measurements.environment = await browser.execute(() => ({
        userAgent: navigator.userAgent,
        devicePixelRatio,
        screenWidth: screen.width,
        screenHeight: screen.height,
        viewportWidth: innerWidth,
        viewportHeight: innerHeight,
      }));
      measurements.mode =
        'native macOS WKWebView debug, Vite desktop-test frontend; React profiling variant recorded by build log; not a release UI benchmark';
      measurements.reactProfilingSamples = measurements.frontend.events.filter(
        (event) => event.metric === 'react_render_ms',
      ).length;
      measurements.backup = 'blocked: production backup adapter unavailable';
      measurements.presentedFrames =
        'not run: rAF observations are not native presented-frame evidence';
    } catch (error) {
      failure = error;
      measurements.failure =
        'Diagnostic route failed; see runner assertion/build evidence. No raw private error exported.';
    } finally {
      await browser.execute(() => window.__stillharbourPerf.stop());
      await diagnostic('stop');
      const after = manifest(fixtureRoot);
      measurements.sourceManifestEqual = JSON.stringify(before) === JSON.stringify(after);
      await writeFile(`${output}/manifest-after.json`, JSON.stringify(after, null, 2));
      if (monitor.exitCode === null && monitor.signalCode === null) {
        const exited = once(monitor, 'exit');
        monitor.kill('SIGTERM');
        await exited;
      }
      try {
        measurements.memory = JSON.parse(memory);
      } catch {
        measurements.memory = {
          status: 'failed',
          reason: 'sampler did not return a complete bounded report',
        };
      }
      await writeFile(`${output}/baseline.json`, JSON.stringify(measurements, null, 2));
      await progress(failure ? 'failed' : 'completed');
      assert.equal(measurements.sourceManifestEqual, true);
    }
    if (failure) throw failure;
  });
});
