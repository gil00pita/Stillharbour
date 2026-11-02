/* global describe, it */
import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';
import {
  auditIdentity,
  auditSource,
  evidence,
  fixture,
  openBackup,
  prepareCatalog,
  quitPhase,
  session,
  snapshot,
} from './native.mjs';
import { readPhaseReceipt } from './session-owner.mjs';

describe('Catalog switch — actual native launch two', () => {
  it('opens the older catalog and prepares a NEW reviewed return while preserving a second edit', async function () {
    this.timeout(360000);
    const first = readPhaseReceipt(session, 1);
    assert.equal(first.passed, true);
    const active = await fixture('catalog_active_audit');
    assert.equal(active.phase, 2);
    assert.notEqual(active.nativePid, first.nativePid);
    assert.equal(active.catalogId, first.catalogId);
    assert.equal(active.rootId, first.rootId);
    assert.equal(active.activeTransactionId, first.preparation.transactionId);
    assert.equal(active.latestRetainedTransactionId, first.preparation.transactionId);
    assert.equal(active.firstEditActive, false);
    assert.equal(active.secondEditActive, false);
    assert.equal(active.firstEditRetained, true);
    assert.equal(active.secondEditRetained, false);
    for (const field of ['preRestoreVerified', 'candidateNativeVerified', 'priorCatalogsRetained'])
      assert.equal(active[field], true, field);
    assert.equal(active.retainedPriorCatalogCount, 2);
    auditIdentity(active.activeCatalogIdentity);
    auditSource(active.sourceAudit);
    assert.notDeepEqual(
      active.activeCatalogIdentity,
      first.preparation.activeCatalogIdentity,
      'Startup opened a NEW candidate object, not the old catalog pointer',
    );
    const edited = await fixture('catalog_second_edit');
    assert.equal(edited.secondEditApplied, true);
    assert.equal(edited.repositoryCredentialUnavailable, true);
    assert.equal(edited.nativePid, active.nativePid);
    const caps = await snapshot({ kind: 'capabilities' }, 'capabilities');
    assert.equal(caps.latestRetainedCatalogTransactionId, first.preparation.transactionId);
    await openBackup();
    assert.equal(
      await $('.backup-workspace select').getValue(),
      '',
      'Retained return has no selected snapshot profile',
    );
    const prepared = await prepareCatalog({
      sourceKind: 'retained_pre_restore',
      catalogId: first.catalogId,
      rootId: first.rootId,
      retainedTransactionId: active.latestRetainedTransactionId,
    });
    assert.notEqual(
      prepared.preparation.transactionId,
      first.preparation.transactionId,
      'A NEW transaction preserves current edits instead of replaying the old pointer',
    );
    assert.equal(prepared.preparation.nativePid, active.nativePid);
    await browser.saveScreenshot(`${evidence}/phase-02-prepared-return.png`);
    await quitPhase(2, active.nativePid, {
      catalogId: first.catalogId,
      rootId: first.rootId,
      active,
      edited,
      ...prepared,
      firstActivationAfterRealRestartVerified: true,
      returnActivationAfterGuiRestartTested: false,
    });
  });
});
