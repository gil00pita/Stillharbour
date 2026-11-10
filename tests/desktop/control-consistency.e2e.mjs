import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';

describe('Screenshot feedback control consistency — generated native fixture', () => {
  it('keeps form controls, icons, headers and the narrow timeline consistent', async function () {
    this.timeout(120000);
    const output = '.context/control-polish';
    await mkdir(output, { recursive: true });
    await browser.setWindowSize(1180, 780);
    const selection = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: false }),
    );
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').waitForEnabled();
    const icon = await browser.execute(() => {
      const svg = document.querySelector('.sidebar [aria-current="page"] svg');
      return { stroke: getComputedStyle(svg).stroke, width: svg.getBoundingClientRect().width };
    });
    assert.equal(icon.stroke === 'none', false);
    assert.equal(icon.width, 18);
    await $('.welcome button.primary').click();
    await $('.root-setup input[required]').setValue('Generated control consistency');
    await $('.root-setup .scan-options summary').click();
    const measurements = [];
    for (const mode of ['light', 'dark']) {
      if (await $(`button[aria-label="Switch to ${mode} appearance"]`).isExisting())
        await $(`button[aria-label="Switch to ${mode} appearance"]`).click();
      await browser.execute(() =>
        document.querySelector('.file-size-controls').scrollIntoView({ block: 'center' }),
      );
      const form = await browser.execute(() => ({
        fields: [...document.querySelectorAll('.file-size-controls')].map((field) => {
          const input = field.querySelector('input');
          const select = field.querySelector('select');
          const i = input.getBoundingClientRect();
          const s = select.getBoundingClientRect();
          return {
            height: i.height,
            selectHeight: s.height,
            offset: i.top - s.top,
            embedded:
              s.left >= i.left &&
              s.right <= i.right &&
              i.right - parseFloat(getComputedStyle(input).paddingRight) <= s.left,
            border: getComputedStyle(input).borderColor,
          };
        }),
        switches: [...document.querySelectorAll('.switch-control')].map((e) => ({
          width: e.getBoundingClientRect().width,
          height: e.getBoundingClientRect().height,
          thumb: getComputedStyle(e, '::before').backgroundColor,
        })),
        buttons: [...document.querySelectorAll('.root-setup .folder-actions button')].map((e) => ({
          top: e.getBoundingClientRect().top,
          height: e.getBoundingClientRect().height,
        })),
      }));
      measurements.push({ mode, form });
      assert.equal(form.fields.length, 2);
      for (const f of form.fields) {
        assert.equal(f.embedded, true, JSON.stringify(f));
        assert.equal(f.height, 36);
        assert.equal(f.selectHeight, 36);
        assert(Math.abs(f.offset) <= 1);
      }
      assert.equal(form.buttons.length, 2);
      assert.equal(form.buttons[0].height, 36);
      assert.equal(form.buttons[1].height, 36);
      assert.equal(form.buttons[0].top, form.buttons[1].top);
      for (const s of form.switches) {
        assert.equal(s.width, 36);
        assert.equal(s.height, 22);
        assert.equal(s.thumb, 'rgb(248, 248, 250)');
      }
      await browser.saveScreenshot(`${output}/setup-${mode}.png`);
    }
    await browser.tauri.execute(({ core }) =>
      core.invoke('test_performance', { action: 'initial_previews_hold', rows: null }),
    );
    await $('button=Save & Start scanning').click();
    try {
      await $('.photo-scanning-art').waitForDisplayed();
      const art = await browser.execute(() => {
        const svg = document.querySelector('.photo-scanning-art');
        const b = svg.getBoundingClientRect();
        return {
          width: b.width,
          height: b.height,
          animation: getComputedStyle(svg.querySelector('.photo-scanning-lens')).animationName,
        };
      });
      assert.equal(art.width, 160);
      assert(art.height > 0 && art.height <= 120);
      measurements.push({ art });
      await browser.saveScreenshot(`${output}/scanning.png`);
    } finally {
      await browser.tauri.execute(({ core }) =>
        core.invoke('test_performance', { action: 'initial_previews_release', rows: null }),
      );
    }
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });
    for (const [width, height] of [
      [1180, 780],
      [660, 560],
    ]) {
      await browser.setWindowSize(width, height);
      await $('.sidebar button[aria-label="Photos"]').click();
      await $('.date-rail').waitForDisplayed();
      const rail = await browser.execute(
        () => document.querySelector('.date-rail').getBoundingClientRect().width,
      );
      assert.equal(rail, 32);
      for (const section of [
        'Photos',
        'Folders',
        'Albums / Tags',
        'Structure Formater',
        'Integrity',
        'Storage',
        'Settings',
      ]) {
        await $(`.sidebar button[aria-label="${section}"]`).click();
        await browser.waitUntil(() =>
          browser.execute(
            (section) => document.querySelector('.app-page-header h1')?.textContent === section,
            section,
          ),
        );
        const header = await browser.execute(() => {
          const h = document.querySelector('.app-page-header');
          const controls = [...h.querySelectorAll('button')].filter(
            (e) => e.getBoundingClientRect().width > 0,
          );
          return {
            controls: controls.map((e) => ({
              label: e.getAttribute('aria-label') || e.textContent,
              x: e.getBoundingClientRect().left,
              right: e.getBoundingClientRect().right,
              width: e.getBoundingClientRect().width,
              height: e.getBoundingClientRect().height,
            })),
            grouped:
              controls.length < 2 ||
              (() => {
                const last = controls.at(-1).getBoundingClientRect();
                const previous = controls.at(-2).getBoundingClientRect();
                return last.left - previous.right <= 12;
              })(),
            height: h.getBoundingClientRect().height,
          };
        });
        measurements.push({ width, section, header });
        assert.equal(header.height, 64);
        for (const c of header.controls) {
          assert.equal(c.height, 36, JSON.stringify(c));
          assert(c.width >= 36, JSON.stringify(c));
          assert(c.right <= width, JSON.stringify(c));
        }
        assert.equal(header.grouped, true, JSON.stringify({ section, header }));
        if (width === 1180 && ['Structure Formater', 'Storage'].includes(section))
          await browser.saveScreenshot(
            `${output}/${section.replaceAll(' ', '-').toLowerCase()}.png`,
          );
      }
    }
    await writeFile(`${output}/geometry.json`, JSON.stringify(measurements, null, 2));
    const audit = await browser.tauri.execute(({ core }) =>
      core.invoke('test_library_fixture', { verify: true }),
    );
    assert.ok(audit.verifiedFiles > 20);
    await writeFile(`${output}/audit.json`, JSON.stringify(audit, null, 2));
  });
});
