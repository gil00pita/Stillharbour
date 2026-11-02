import { config as map } from './wdio.map.conf.mjs';
export const config = {
  ...map,
  specs: ['./tests/desktop/metadata-export.e2e.mjs'],
  outputDir: './.context/wdio-metadata',
};
