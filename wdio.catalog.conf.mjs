import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { config as normal } from './wdio.conf.mjs';
import {
  ROOT_ENV,
  OWNER_ENV,
  cleanupSessionOwner,
  createSessionOwner,
  evidenceDirectory,
  readPhaseReceipt,
  sessionFromEnvironment,
  verifySessionOwner,
  waitForProcessExit,
} from './tests/catalog-desktop/session-owner.mjs';

// Module evaluation precedes service onPrepare/spawn. Workers inherit the two
// bounded native session variables instead of creating another application tree.
const createdHere = !process.env[ROOT_ENV] && !process.env[OWNER_ENV];
const session = createdHere ? createSessionOwner() : sessionFromEnvironment();
process.env[ROOT_ENV] = session.root;
process.env[OWNER_ENV] = session.ownerNonce;

export const config = {
  ...normal,
  specs: [
    './tests/catalog-desktop/catalog-01.e2e.mjs',
    './tests/catalog-desktop/catalog-02.e2e.mjs',
    './tests/catalog-desktop/catalog-03.e2e.mjs',
  ],
  maxInstances: 1,
  specFileRetries: 0,
  mochaOpts: { timeout: 360000 },
  outputDir: './.context/wdio-catalog-restart',
  after() {
    /* Each phase requests and confirms its own real native quit. */
  },
  onPrepare() {
    verifySessionOwner(session);
  },
  async onComplete(exitCode) {
    if (!createdHere) return;
    const evidence = evidenceDirectory(session);
    if (exitCode !== 0) {
      writeFileSync(
        join(evidence, 'retained.json'),
        JSON.stringify(
          { retained: true, reason: 'Native suite failed', ownerNonce: session.ownerNonce },
          null,
          2,
        ),
        { flag: 'wx', mode: 0o600 },
      );
      console.info(`Retained generated catalog session: ${session.root}`);
      return;
    }
    try {
      const phases = [1, 2, 3].map((phase) => readPhaseReceipt(session, phase));
      for (const phase of phases) {
        assert.equal(phase.passed, true);
        assert.equal(phase.nativeQuitConfirmed, true);
        await waitForProcessExit(phase.nativePid);
      }
      assert.equal(
        new Set(phases.map((phase) => phase.nativePid)).size,
        3,
        'Three actual native application processes',
      );
      assert.equal(phases[2].nativeCleanupConfirmed, true);
      verifySessionOwner(session);
      cleanupSessionOwner(session);
      writeFileSync(
        join(evidence, 'complete.json'),
        JSON.stringify(
          {
            passed: true,
            actualNativeLaunches: 3,
            generatedDirectoryRemoved: true,
            nativeCredentialsAndAuthorityCleanupConfirmed: true,
          },
          null,
          2,
        ),
        { flag: 'wx', mode: 0o600 },
      );
    } catch (error) {
      console.info(
        `Retained generated catalog session after incomplete validation: ${session.root}`,
      );
      throw error;
    }
  },
};
