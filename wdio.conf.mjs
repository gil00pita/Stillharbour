import { resolve } from 'node:path';
import assert from 'node:assert/strict';
import { waitForProcessExit } from './tests/catalog-desktop/session-owner.mjs';
const app = resolve(`target/debug/stillharbour${process.platform === 'win32' ? '.exe' : ''}`);

export const config = {
  runner: 'local',
  specs: ['./tests/desktop/**/*.e2e.mjs'],
  maxInstances: 1,
  services: [
    [
      '@wdio/tauri-service',
      {
        appBinaryPath: app,
        driverProvider: 'embedded',
        autoInstallTauriDriver: false,
        autoDownloadEdgeDriver: false,
        captureFrontendLogs: true,
        captureBackendLogs: true,
      },
    ],
  ],
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: app } }],
  framework: 'mocha',
  reporters: ['spec'],
  logLevel: 'warn',
  waitforTimeout: 15000,
  connectionRetryCount: 0,
  mochaOpts: { timeout: 45000 },
  outputDir: './.context/wdio',
  async after() {
    // Every default spec gets a genuinely new native process/data tree. The
    // embedded service restarts a dead app before the next worker (1.5.1).
    const { browser } = await import('@wdio/globals');
    const result = await browser.tauri.execute(async ({ core }) =>
      core.invoke('test_performance', { action: 'm4_application_catalog_quit', rows: null }),
    );
    assert.equal(result.quitRequested, true);
    await waitForProcessExit(result.nativePid, 45000);
  },
};
