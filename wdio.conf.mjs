import { resolve } from 'node:path';
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
};
