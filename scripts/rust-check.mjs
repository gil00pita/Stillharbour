import { run } from './process.mjs';
run('cargo', ['fmt', '--all', '--check']);
run('cargo', [
  'clippy',
  '--workspace',
  '--all-targets',
  '--features',
  'stillharbour-core/fixture-probes,stillharbour-core/copy-workflow',
  '--',
  '-D',
  'warnings',
]);
run('cargo', [
  'test',
  '--workspace',
  '--features',
  'stillharbour-core/fixture-probes,stillharbour-core/copy-workflow',
]);
