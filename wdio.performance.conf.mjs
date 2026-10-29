import { config as foundation } from './wdio.conf.mjs';
export const config = {
  ...foundation,
  specs: ['./tests/performance/gallery.e2e.mjs'],
  mochaOpts: { timeout: 900000 },
  outputDir: './.context/wdio-performance',
};
