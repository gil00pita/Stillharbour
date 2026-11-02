/* global describe, it */
import assert from 'node:assert/strict';
import { browser } from '@wdio/globals';
import {
  approvedRun,
  auditSource,
  completed,
  evidence,
  fixture,
  openBackup,
  prepareCatalog,
  quitPhase,
  snapshot,
} from './native.mjs';

describe('Catalog switch — actual native launch one', () => {
  it('preserves a newer catalog before preparing its earlier verified snapshot and actually exits', async function () {
    this.timeout(360000);
    const source = await fixture('source');
    await browser.waitUntil(
      async () => {
        const envelope = await browser.tauri.execute(async ({ core }) =>
          core.invoke('library_request', { request: { kind: 'status' } }),
        );
        const job = envelope.data.jobs.find((job) => job.rootId === source.rootId);
        if (!job || ['queued', 'running', 'scanning', 'cancel_requested'].includes(job.state))
          return false;
        assert.equal(job.state, 'completed');
        assert.equal(envelope.data.roots.length, 1);
        return true;
      },
      { timeout: 30000, interval: 100 },
    );
    const destination = await fixture('repository');
    const profile = await snapshot(
      {
        kind: 'profile_create',
        label: 'Generated three-launch catalog recovery',
        root_ids: [source.rootId],
        repository_credential_ref: source.credentialRef,
        transport_credential_ref: null,
        destination: { kind: 'local', selection_id: destination.selectionId },
      },
      'profile',
    );
    const review = await snapshot({ kind: 'backup_review', profile_id: profile.id }, 'review');
    assert.equal(review.catalogIncluded, true);
    assert.equal(review.files, 4);
    const backup = await approvedRun(review);
    const records = await snapshot({ kind: 'snapshots', profile_id: profile.id }, 'snapshots');
    const record = records.find((record) => record.id === backup.snapshotId);
    assert.ok(record);
    const browsed = await snapshot(
      { kind: 'browse', profile_id: profile.id, snapshot_id: record.id, offset: 0 },
      'browse',
    );
    assert.ok(browsed.entries.length > 0);
    const verification = await completed(
      await snapshot({ kind: 'verify', profile_id: profile.id, snapshot_id: record.id }, 'job'),
    );
    const removed = await fixture('remove_source');
    assert.equal(removed.sourceUnchangedBeforeRemoval, true);
    assert.equal(removed.sourceRemoved, true);
    const destinationRestore = await fixture('restore');
    const restoreReview = await snapshot(
      {
        kind: 'restore_review',
        profile_id: profile.id,
        snapshot_id: record.id,
        selection_id: destinationRestore.selectionId,
      },
      'review',
    );
    const restored = await approvedRun(restoreReview);
    const sourceAudit = await fixture('audit');
    auditSource(sourceAudit);
    const edited = await fixture('catalog_newer_edit');
    assert.equal(edited.firstEditApplied, true);
    assert.equal(edited.catalogId, source.catalogId);
    await openBackup(profile.id);
    const prepared = await prepareCatalog({
      sourceKind: 'verified_snapshot_restore',
      catalogId: source.catalogId,
      rootId: source.rootId,
      restoreJobId: restored.id,
      snapshotId: record.id,
    });
    assert.equal(prepared.preparation.nativePid, edited.nativePid);
    await browser.saveScreenshot(`${evidence}/phase-01-prepared.png`);
    await quitPhase(1, edited.nativePid, {
      catalogId: source.catalogId,
      rootId: source.rootId,
      profileId: profile.id,
      snapshotId: record.id,
      verification,
      sourceAudit,
      edited,
      ...prepared,
      guiRestartActivationTested: false,
    });
  });
});
