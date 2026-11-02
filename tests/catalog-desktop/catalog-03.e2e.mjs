/* global describe, it */
import assert from 'node:assert/strict';
import { browser } from '@wdio/globals';
import { auditIdentity, auditSource, evidence, fixture, quitPhase, session } from './native.mjs';
import { readPhaseReceipt } from './session-owner.mjs';

describe('Catalog switch — actual native launch three', () => {
  it('opens the first newer catalog and proves the second edit survived in another retained catalog', async function () {
    this.timeout(180000);
    const first = readPhaseReceipt(session, 1);
    const second = readPhaseReceipt(session, 2);
    const active = await fixture('catalog_active_audit');
    assert.equal(active.phase, 3);
    assert.notEqual(active.nativePid, first.nativePid);
    assert.notEqual(active.nativePid, second.nativePid);
    assert.equal(active.catalogId, first.catalogId);
    assert.equal(active.rootId, first.rootId);
    assert.equal(active.activeTransactionId, second.preparation.transactionId);
    assert.equal(active.latestRetainedTransactionId, second.preparation.transactionId);
    assert.equal(active.firstEditActive, true);
    assert.equal(active.secondEditActive, false);
    assert.equal(active.secondEditRetained, true);
    assert.equal(active.repositoryCredentialUnavailable, true);
    for (const field of ['preRestoreVerified', 'candidateNativeVerified', 'priorCatalogsRetained'])
      assert.equal(active[field], true, field);
    assert.equal(active.retainedPriorCatalogCount, 4);
    auditIdentity(active.activeCatalogIdentity);
    auditSource(active.sourceAudit);
    assert.notDeepEqual(active.activeCatalogIdentity, first.preparation.activeCatalogIdentity);
    assert.notDeepEqual(active.activeCatalogIdentity, second.active.activeCatalogIdentity);
    await browser.saveScreenshot(`${evidence}/phase-03-returned.png`);
    const cleanup = await fixture('catalog_session_cleanup');
    assert.equal(cleanup.nativePid, active.nativePid);
    for (const field of [
      'repositoryCredentialRemoved',
      'activationAuthorityRemoved',
      'sessionScopeVerified',
    ])
      assert.equal(cleanup[field], true, field);
    auditSource(cleanup.audit.sourceAudit);
    await quitPhase(3, active.nativePid, {
      catalogId: first.catalogId,
      rootId: first.rootId,
      active,
      cleanup,
      nativeCleanupConfirmed: true,
      actualGuiRestartActivationVerified: true,
      bothGenerationsOfCurrentEditsRetained: true,
    });
  });
});
