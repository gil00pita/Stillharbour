import { config as foundation } from './wdio.conf.mjs';
export const config = {
  ...foundation,
  specs: ['./tests/formats/native.e2e.mjs'],
  mochaOpts: { timeout: 90000 },
  outputDir: './.context/wdio-formats',
};
