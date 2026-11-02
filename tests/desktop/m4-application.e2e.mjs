import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { browser, $ } from '@wdio/globals';
import { createEvidenceDirectory } from './evidence.mjs';

async function fixture(target) {
  return browser.tauri.execute(
    async ({ core }, target) =>
      core.invoke('test_performance', { action: `m4_application_${target}`, rows: null }),
    target,
  );
}
async function snapshot(request, kind) {
  const envelope = await browser.tauri.execute(
    async ({ core }, request) =>
      core.invoke('library_request', { request: { kind: 'backup_snapshots', request } }),
    request,
  );
  assert.equal(envelope.kind, 'backup_snapshots', JSON.stringify(envelope));
  assert.equal(envelope.data.kind, kind, JSON.stringify(envelope));
  return envelope.data.data;
}
async function finished(initial) {
  let job = initial;
  await browser.waitUntil(
    async () => {
      job = await snapshot({ kind: 'job', job_id: initial.id }, 'job');
      assert.equal(job.id, initial.id);
      assert.equal(job.profileId, initial.profileId);
      assert.equal(job.action, initial.action);
      return !['queued', 'running', 'cancel_requested'].includes(job.state);
    },
    { timeout: 120000, interval: 250, timeoutMsg: `Native ${initial.action} did not finish` },
  );
  if (job.state !== 'completed_verified') {
    const evidence = await createEvidenceDirectory('.context/m4-application');
    await writeFile(`${evidence}/failed-job.json`, JSON.stringify(job, null, 2));
    await browser.saveScreenshot(`${evidence}/failed-job.png`);
  }
  assert.equal(job.state, 'completed_verified', job.message);
  return job;
}
async function approveAndStart(review) {
  assert.ok(['backup', 'restore'].includes(review.action));
  assert.equal(review.executionAvailable, true, review.unavailableReason);
  assert.equal(review.sourceRetained, true);
  const approval = await snapshot(
    {
      kind: review.action === 'backup' ? 'backup_approve' : 'restore_approve',
      review_id: review.id,
      digest: review.digest,
    },
    'approval',
  );
  assert.equal(approval.reviewId, review.id);
  assert.equal(approval.digest, review.digest);
  const initial = await snapshot(
    {
      kind: review.action === 'backup' ? 'backup_start' : 'restore_start',
      review_id: review.id,
      approval_id: approval.id,
    },
    'job',
  );
  assert.equal(initial.action, review.action);
  assert.equal(initial.profileId, review.profileId);
  return finished(initial);
}

describe('M4 application snapshot lifecycle — generated native fixtures', () => {
  it('backs up, reads stored data and restores bytes through real application commands', async function () {
    this.timeout(360000);
    const source = await fixture('source');
    assert.ok(source.rootId);
    assert.ok(source.catalogId);
    assert.ok(source.credentialRef);
    await browser.waitUntil(
      async () => {
        const reply = await browser.tauri.execute(async ({ core }) =>
          core.invoke('library_request', { request: { kind: 'status' } }),
        );
        assert.equal(reply.kind, 'status');
        const scan = reply.data.jobs.find((job) => job.rootId === source.rootId);
        if (!scan || ['queued', 'running', 'scanning', 'cancel_requested'].includes(scan.state))
          return false;
        assert.equal(scan.state, 'completed', scan.message);
        return true;
      },
      { timeout: 30000, interval: 100, timeoutMsg: 'Generated M4 source scan did not finish' },
    );
    const destination = await fixture('repository');
    assert.ok(destination.selectionId);
    const capabilities = await snapshot({ kind: 'capabilities' }, 'capabilities');
    assert.equal(capabilities.backupAvailable, true, capabilities.unavailableReason);
    assert.equal(capabilities.restoreAvailable, true, capabilities.unavailableReason);
    const profile = await snapshot(
      {
        kind: 'profile_create',
        label: 'Generated native M4 lifecycle',
        root_ids: [source.rootId],
        repository_credential_ref: source.credentialRef,
        transport_credential_ref: null,
        destination: { kind: 'local', selection_id: destination.selectionId },
      },
      'profile',
    );
    assert.equal(profile.catalogId, source.catalogId);
    assert.deepEqual(profile.rootIds, [source.rootId]);
    assert.deepEqual(await snapshot({ kind: 'jobs', profile_id: profile.id }, 'jobs'), []);

    await browser.refresh();
    await $('button[aria-label="Backup & Sync"]').waitForDisplayed();
    await $('button[aria-label="Backup & Sync"]').click();
    await $('button=Versioned backup').click();
    await $('.backup-workspace select').waitForEnabled();
    // The embedded driver's option click does not perform a native select change.
    // Exercise the same DOM change event as the existing foundation native suite.
    await browser.execute((id) => {
      const select = document.querySelector('.backup-workspace select');
      if (!(select instanceof HTMLSelectElement))
        throw new Error('Snapshot profile selector is unavailable');
      select.value = id;
      select.dispatchEvent(new Event('change', { bubbles: true }));
    }, profile.id);
    await $('section[aria-label="Snapshot profile workspace"]').waitForDisplayed();
    assert.equal(await $('button=Start encrypted backup').isExisting(), false);
    assert.deepEqual(await snapshot({ kind: 'jobs', profile_id: profile.id }, 'jobs'), []);

    const review = await snapshot({ kind: 'backup_review', profile_id: profile.id }, 'review');
    assert.equal(review.catalogIncluded, true);
    assert.deepEqual(review.rootIds, [source.rootId]);
    assert.ok(review.sourceDisplayPaths.length);
    assert.ok(review.files > 0);
    await $('button=Review backup scope').click();
    await $('section[aria-label="Snapshot action review"]').waitForDisplayed();
    await browser.waitUntil(
      async () =>
        (await $('section[aria-label="Snapshot action review"]').getText()).includes(review.digest),
      { timeout: 15000, timeoutMsg: 'Actual UI did not show the expected immutable manifest' },
    );
    await $('section[aria-label="Snapshot action review"] input[type="checkbox"]').click();
    await $('button=Approve this snapshot action').waitForEnabled();
    await $('button=Approve this snapshot action').click();
    await $('button=Start encrypted backup').waitForEnabled();
    assert.deepEqual(await snapshot({ kind: 'jobs', profile_id: profile.id }, 'jobs'), []);
    await $('button=Start encrypted backup').click();
    let started;
    await browser.waitUntil(
      async () => {
        const jobs = await snapshot({ kind: 'jobs', profile_id: profile.id }, 'jobs');
        started = jobs.find((job) => job.action === 'backup');
        return Boolean(started);
      },
      {
        timeout: 15000,
        interval: 100,
        timeoutMsg: 'Explicit UI start did not create a native backup job',
      },
    );
    assert.equal(started.profileId, profile.id);
    const backup = await finished(started);
    assert.equal(backup.included, true);
    assert.equal(backup.structuralChecked, true);
    assert.equal(backup.dataReadVerified, true);
    assert.equal(backup.restoredCompared, true);
    assert.equal(backup.independentRecoveryVerified, false);
    assert.ok(backup.snapshotId);
    assert.match(backup.snapshotId, /^[a-f0-9]{64}$/);
    const liveProfile = (await snapshot({ kind: 'profiles' }, 'profiles')).find(
      (value) => value.id === profile.id,
    );
    assert.ok(liveProfile.repositoryId);
    await browser.waitUntil(
      async () => {
        const text = await $('section[aria-label="Snapshot profile workspace"]').getText();
        return (
          text.includes(`Repository identity: ${liveProfile.repositoryId}`) &&
          !text.includes('not initialized') &&
          !text.includes('Not established yet')
        );
      },
      {
        timeout: 15000,
        interval: 100,
        timeoutMsg: 'Terminal backup did not refresh authoritative repository identity/status',
      },
    );
    assert.ok(
      (await $('section[aria-label="Snapshot source roots"]').getText()).includes(
        'Last observed access:',
      ),
    );

    const records = await snapshot({ kind: 'snapshots', profile_id: profile.id }, 'snapshots');
    const record = records.find((value) => value.id === backup.snapshotId);
    assert.ok(record);
    assert.equal(record.catalogIncluded, true);
    const entries = [];
    let offset = 0;
    do {
      const page = await snapshot(
        { kind: 'browse', profile_id: profile.id, snapshot_id: record.id, offset },
        'browse',
      );
      assert.equal(page.profileId, profile.id);
      assert.equal(page.snapshotId, record.id);
      entries.push(...page.entries);
      offset = page.nextOffset;
    } while (offset !== null);
    assert.ok(entries.length > 0);
    const verification = await finished(
      await snapshot({ kind: 'verify', profile_id: profile.id, snapshot_id: record.id }, 'job'),
    );
    assert.equal(verification.dataReadVerified, true);
    assert.equal(verification.restoredCompared, true);

    assert.equal(capabilities.scheduleAvailable, true, capabilities.unavailableReason);
    assert.equal(
      await snapshot({ kind: 'schedule_get', profile_id: profile.id }, 'schedule'),
      null,
    );
    const repeatReview = await snapshot(
      { kind: 'backup_review', profile_id: profile.id },
      'review',
    );
    const schedule = await snapshot(
      {
        kind: 'schedule_save',
        profile_id: profile.id,
        review_id: repeatReview.id,
        digest: repeatReview.digest,
        interval_minutes: 10080,
        timezone: 'UTC',
        enabled: true,
        catch_up: true,
      },
      'schedule',
    );
    assert.equal(schedule.enabled, true);
    assert.equal(schedule.approvedDigest, repeatReview.digest);
    assert.equal(schedule.fullyQuitSupported, false);
    assert.equal(schedule.timezone, 'UTC');
    const disabledSchedule = await snapshot(
      { kind: 'schedule_disable', profile_id: profile.id },
      'schedule',
    );
    assert.equal(disabledSchedule.enabled, false);
    // This verifies explicit recurring consent and disablement, not a due execution.

    // This native action removes only the exact constructor-owned generated source after
    // a verified snapshot. Restoring its earlier bytes tests retention after disappearance.
    const removed = await fixture('remove_source');
    assert.equal(removed.sourceRemoved, true);
    const retainedHistory = await snapshot(
      { kind: 'snapshots', profile_id: profile.id },
      'snapshots',
    );
    assert.ok(retainedHistory.some((value) => value.id === record.id));

    const restoreTarget = await fixture('restore');
    const restoreReview = await snapshot(
      {
        kind: 'restore_review',
        profile_id: profile.id,
        snapshot_id: record.id,
        selection_id: restoreTarget.selectionId,
      },
      'review',
    );
    assert.equal(restoreReview.action, 'restore');
    const restored = await approveAndStart(restoreReview);
    assert.equal(restored.restoredCompared, true);
    assert.equal(restored.sourceRetained, true);
    await $('button=Refresh snapshot history').click();
    await browser.waitUntil(
      async () => {
        const text = await $('section[aria-label="Snapshot profile workspace"]').getText();
        return text.includes(`Snapshot ${record.id}`) && text.includes(restored.message);
      },
      {
        timeout: 15000,
        interval: 100,
        timeoutMsg: 'Native snapshot and restore history was not displayed',
      },
    );
    const audit = await fixture('audit');
    for (const key of [
      'sourceUnchangedBeforeRemoval',
      'sourceRemoved',
      'destinationOnlyPreserved',
      'restoredCompared',
      'catalogHeaderFound',
      'onlyFixtureWrites',
    ])
      assert.equal(audit[key], true, `Native fixture audit: ${key}`);
    assert.equal(audit.restoredFilesCompared, 4);
    const evidence = await createEvidenceDirectory('.context/m4-application');
    await writeFile(
      `${evidence}/snapshot-lifecycle.json`,
      JSON.stringify(
        {
          capabilities,
          review,
          backup,
          record,
          verification,
          schedule,
          disabledSchedule,
          removed,
          restoreReview,
          restored,
          audit,
        },
        null,
        2,
      ),
    );
    await browser.saveScreenshot(`${evidence}/snapshot-lifecycle.png`);
  });
});
