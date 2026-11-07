import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';

// A separate native process and generated catalog; never scan personal libraries.
describe('Utility scan lifecycle', () => {
  it('shows first-use settings, retains real reports and reconciles on return', async function () {
    this.timeout(120000);
    const invoke = (command, args) =>
      browser.tauri.execute(
        async ({ core }, command, args) => core.invoke(command, args),
        command,
        args,
      );
    const native = async (request) => (await invoke('library_request', { request })).data;
    // Let real startup status reads finish before installing a picker-only fixture.
    await $('.welcome button.primary').waitForEnabled({ timeout: 30000 });
    const selection = await invoke('test_library_fixture', { verify: false });
    await browser.execute((selection) => {
      window.__wdio_mocks__ ??= {};
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, selection);
    await $('.welcome button.primary').click();
    await $('.selected-folder').waitForDisplayed();
    await $('.root-setup input[required]').setValue('Generated utility lifecycle');
    await $('button=Save & Start scanning').click();
    await $('.asset-open', { strict: false }).waitForDisplayed({ timeout: 45000 });

    for (const [section, kind, start] of [
      ['Duplicates', 'duplicates', 'Start duplicate inspection'],
      ['Integrity', 'health', 'Start quick header check'],
      ['Storage', 'storage', 'Start storage inspection'],
    ]) {
      await $(`button[aria-label="${section}"]`).click();
      await $('section[aria-label="Scan introduction"]').waitForDisplayed();
      assert.equal(await $('section[aria-label="Duplicate groups"]').isExisting(), false);
      assert.equal(
        (await native({ kind: 'inspection_jobs' })).filter((job) => job.input.task.kind === kind)
          .length,
        0,
      );
      if (section === 'Integrity') {
        await browser.execute(() => {
          const mode = document.querySelector('select[aria-label="Integrity mode"]');
          Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set.call(
            mode,
            'quick',
          );
          mode.dispatchEvent(new Event('change', { bubbles: true }));
        });
      }
      for (const [width, height] of [
        [1180, 780],
        [660, 560],
      ]) {
        await browser.setWindowSize(width, height);
        assert.equal(
          await browser.execute(() => document.documentElement.scrollWidth > innerWidth),
          false,
        );
        await browser.saveScreenshot(`.context/utility-${kind}-welcome-${width}.png`);
      }
      await $(`button=${start}`).scrollIntoView({ block: 'center' });
      await $(`button=${start}`).waitForEnabled();
      await $(`button=${start}`).click();
      let first;
      await browser.waitUntil(
        async () => {
          first = (await native({ kind: 'inspection_jobs' })).find(
            (job) => job.input.task.kind === kind,
          );
          return first && !['queued', 'running', 'cancel_requested'].includes(first.state);
        },
        { timeout: 45000, interval: 150 },
      );
      assert(['completed', 'partial'].includes(first.state), first.message);
      assert(await native({ kind: 'inspection_report', job_id: first.id }));
      await $('button[aria-label="Photos"]').click();
      await $(`button[aria-label="${section}"]`).click();
      await browser.waitUntil(
        async () =>
          (await native({ kind: 'inspection_jobs' })).filter((job) => job.input.task.kind === kind)
            .length === 2,
      );
      await $('button[aria-label="Scan history"]').click();
      await $('[aria-label="Saved scan versions"]').waitForDisplayed();
      await browser.waitUntil(
        async () =>
          (await browser.execute(
            () => document.querySelectorAll('[aria-label="Saved scan versions"] button').length,
          )) === 2,
      );
      await browser.saveScreenshot(`.context/utility-${kind}-history.png`);
      await browser.keys('Escape');
      await $('[aria-label="Saved scan versions"]').waitForExist({ reverse: true });
      await browser.waitUntil(
        async () =>
          (await browser.execute(() => document.activeElement?.getAttribute('aria-label'))) ===
          'Scan history',
      );
      await browser.waitUntil(
        async () =>
          !(await native({ kind: 'inspection_jobs' })).some((job) =>
            ['queued', 'running', 'cancel_requested'].includes(job.state),
          ),
        { timeout: 45000 },
      );
    }

    await $('button[aria-label="Sync"]').click();
    await $('section[aria-label="Scan introduction"]').waitForDisplayed();
    const destination = await invoke('test_performance', {
      action: 'backup_destination_fixture',
      rows: null,
    });
    await browser.execute((selection) => {
      window.__wdio_mocks__.foundation_request = () => ({ kind: 'directory', data: selection });
    }, destination);
    await $('button=Choose destination').click();
    await $('input[maxlength="80"]').setValue('Generated utility comparison');
    await $('button=Save comparison profile').waitForEnabled();
    await $('button=Save comparison profile').click();
    // Saving remounts the workspace. Wait for its selected native profile
    // before acquiring Compare, rather than retaining the disabled old button.
    let profile;
    await browser.waitUntil(async () => {
      profile = (await native({ kind: 'backup_profiles' })).find(
        (item) => item.label === 'Generated utility comparison',
      );
      return (
        profile &&
        (await $('section[aria-label="Saved comparisons"] select').getValue()) === profile.id
      );
    });
    await $('button=Compare').waitForEnabled();
    await $('button=Compare').click();
    await browser.waitUntil(
      async () => (await native({ kind: 'backup_comparison_history' })).length === 1,
      { timeout: 45000 },
    );
    await $('.backup-table').waitForDisplayed();
    const saved = (await native({ kind: 'backup_comparison_history' }))[0];
    assert.equal(saved.profileId, profile.id);
    assert.equal(saved.planDigest, null);
    assert.equal(saved.reviewPlan, null);
    assert.equal(saved.capabilities.executionAvailable, false);
    await $('button[aria-label="Photos"]').click();
    await $('button[aria-label="Sync"]').click();
    await browser.waitUntil(
      async () => (await native({ kind: 'backup_comparison_history' })).length === 2,
      { timeout: 45000 },
    );
    await $('button[aria-label="Scan history"]').click();
    await $('[aria-label="Saved scan versions"]').waitForDisplayed();
    await browser.waitUntil(
      async () =>
        (await browser.execute(
          () => document.querySelectorAll('[aria-label="Saved scan versions"] button').length,
        )) === 2,
    );
    await browser.saveScreenshot('.context/utility-backup-history.png');
    assert((await invoke('test_library_fixture', { verify: true })).verifiedFiles > 20);
    await invoke('test_performance', { action: 'backup_destination_verify', rows: null });
  });
});
