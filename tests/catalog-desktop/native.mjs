/* global document, HTMLSelectElement */
import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';
import {
  evidenceDirectory,
  sessionFromEnvironment,
  waitForProcessExit,
  writePhaseReceipt,
} from './session-owner.mjs';

export const session = sessionFromEnvironment();
export const evidence = evidenceDirectory(session);
export async function fixture(target) {
  return browser.tauri.execute(
    async ({ core }, action) => core.invoke('test_performance', { action, rows: null }),
    `m4_application_${target}`,
  );
}
export async function snapshot(request, kind) {
  const reply = await browser.tauri.execute(
    async ({ core }, request) =>
      core.invoke('library_request', { request: { kind: 'backup_snapshots', request } }),
    request,
  );
  assert.equal(reply.kind, 'backup_snapshots');
  assert.equal(reply.data.kind, kind);
  return reply.data.data;
}
export async function completed(initial) {
  let value = initial;
  await browser.waitUntil(
    async () => {
      value = await snapshot({ kind: 'job', job_id: initial.id }, 'job');
      assert.equal(value.id, initial.id);
      assert.equal(value.profileId, initial.profileId);
      assert.equal(value.action, initial.action);
      return !['queued', 'running', 'cancel_requested'].includes(value.state);
    },
    { timeout: 120000, interval: 100, timeoutMsg: 'Generated native job did not stop' },
  );
  assert.equal(value.state, 'completed_verified', value.message);
  for (const field of [
    'included',
    'structuralChecked',
    'dataReadVerified',
    'restoredCompared',
    'sourceRetained',
  ])
    assert.equal(value[field], true, field);
  assert.equal(value.independentRecoveryVerified, false);
  return value;
}
export async function approvedRun(review) {
  assert.equal(review.executionAvailable, true, review.unavailableReason);
  const backup = review.action === 'backup';
  assert.ok(backup || review.action === 'restore');
  const approval = await snapshot(
    {
      kind: backup ? 'backup_approve' : 'restore_approve',
      review_id: review.id,
      digest: review.digest,
    },
    'approval',
  );
  assert.equal(approval.reviewId, review.id);
  assert.equal(approval.digest, review.digest);
  return completed(
    await snapshot(
      {
        kind: backup ? 'backup_start' : 'restore_start',
        review_id: review.id,
        approval_id: approval.id,
      },
      'job',
    ),
  );
}
export function auditSource(audit) {
  for (const field of [
    'sourceUnchangedBeforeRemoval',
    'sourceRemoved',
    'destinationOnlyPreserved',
    'restoredCompared',
    'catalogHeaderFound',
    'onlyFixtureWrites',
  ])
    assert.equal(audit[field], true, field);
  assert.equal(audit.restoredFilesCompared, 4);
}
export function auditIdentity(value) {
  for (const field of ['volume', 'file', 'links']) assert.match(value[field], /^\d+$/);
}
export async function openBackup(profileId = null) {
  await $('button[aria-label="Backup & Sync"]').waitForDisplayed();
  await $('button[aria-label="Backup & Sync"]').click();
  await $('button=Versioned backup').click();
  if (profileId) {
    await $('.backup-workspace select').waitForEnabled();
    await browser.execute((id) => {
      const select = document.querySelector('.backup-workspace select');
      if (!(select instanceof HTMLSelectElement))
        throw new Error('Snapshot profile selector missing');
      select.value = id;
      select.dispatchEvent(new Event('change', { bubbles: true }));
    }, profileId);
    await $('section[aria-label="Snapshot profile workspace"]').waitForDisplayed();
  }
}
export async function prepareCatalog({
  sourceKind,
  catalogId,
  rootId,
  restoreJobId = null,
  snapshotId = null,
  retainedTransactionId = null,
}) {
  const retained = sourceKind === 'retained_pre_restore';
  const button = $(
    retained ? 'button=Review latest retained catalog' : 'button=Review catalog switch',
  );
  await button.waitForEnabled();
  await button.click();
  const panel = $(
    retained
      ? 'section[aria-label="Retained catalog switch"]'
      : 'section[aria-label="Restored catalog switch"]',
  );
  await panel.$('button=Approve catalog switch').waitForDisplayed();
  assert.equal(await panel.$('button=Approve catalog switch').isEnabled(), false);
  const roots = await panel.$$('ul[aria-label="Restored catalog root mappings"] li');
  assert.equal(roots.length, 1, 'This dedicated generated session has exactly one catalog root');
  const choice = await fixture('catalog_root_picker');
  assert.equal(choice.rootId, rootId);
  await roots[0].$('button').click();
  await browser.waitUntil(
    async () => (await panel.getText()).includes('Native identity confirmed:'),
    { timeout: 15000, interval: 100 },
  );
  const review = await browser.execute((retained) => {
    const panel = document.querySelector(
      retained
        ? 'section[aria-label="Retained catalog switch"]'
        : 'section[aria-label="Restored catalog switch"]',
    );
    if (!panel) throw new Error('Catalog review missing');
    const facts = [...panel.querySelectorAll('dl dd')].map((node) => node.textContent.trim());
    return {
      sourceKind: panel.getAttribute('data-catalog-source'),
      restoreJobId: panel.getAttribute('data-restore-job-id'),
      snapshotId: panel.getAttribute('data-snapshot-id'),
      retainedTransactionId: panel.getAttribute('data-retained-transaction-id'),
      catalogId: facts[0],
      schema: facts[1],
      currentRevision: facts[2],
      restoredRevision: facts[3],
    };
  }, retained);
  assert.equal(review.sourceKind, sourceKind);
  assert.equal(review.catalogId, catalogId);
  assert.equal(review.restoreJobId, restoreJobId);
  assert.equal(review.snapshotId, snapshotId);
  assert.equal(review.retainedTransactionId, retainedTransactionId);
  await panel.$('input[type="checkbox"]').click();
  await panel.$('button=Approve catalog switch').click();
  await browser.waitUntil(
    async () => (await panel.getText()).includes('Catalog preparation has not started.'),
    { timeout: 15000, interval: 100 },
  );
  await panel.$('button=Prepare catalog switch').click();
  const stopped = $('main[aria-label="Catalog switch session"]');
  await browser.waitUntil(
    async () => (await stopped.getText()).includes('Catalog switch awaits restart'),
    { timeout: 60000, interval: 100 },
  );
  assert.ok((await stopped.getText()).includes('not confirmed active'));
  assert.ok((await stopped.getText()).includes('Close and reopen Stillharbour'));
  assert.equal(await $('button=Versioned backup').isExisting(), false);
  const admission = await browser.tauri.execute(async ({ core }) => {
    try {
      await core.invoke('library_request', { request: { kind: 'status' } });
      return { rejected: false, code: null };
    } catch (error) {
      return {
        rejected: true,
        code: typeof error === 'object' && error !== null && 'code' in error ? error.code : null,
      };
    }
  });
  assert.deepEqual(admission, { rejected: true, code: 'BUSY' });
  const preparation = await fixture('catalog_audit');
  assert.equal(preparation.catalogId, catalogId);
  assert.equal(preparation.activationClaimed, false);
  for (const field of [
    'pendingActivation',
    'currentCatalogRetained',
    'preRestoreVerified',
    'candidateVerified',
    'fixturePickerOnly',
  ])
    assert.equal(preparation[field], true, field);
  auditIdentity(preparation.activeCatalogIdentity);
  auditSource(preparation.sourceAudit);
  return { review, preparation, admission };
}
export async function quitPhase(phase, nativePid, values) {
  const quit = await fixture('catalog_quit');
  assert.equal(quit.quitRequested, true);
  assert.equal(quit.nativePid, nativePid);
  await waitForProcessExit(nativePid, 45000);
  writePhaseReceipt(session, phase, {
    ...values,
    nativePid,
    nativeQuitConfirmed: true,
    passed: true,
  });
}
