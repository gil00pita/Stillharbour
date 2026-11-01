import { resolve, join } from 'node:path';
import { run } from './process.mjs';

if (!['win32', 'darwin'].includes(process.platform))
  throw new Error('Native cross-volume fixtures require Windows or macOS.');
const preflight = process.argv.slice(2).includes('--preflight');
if (process.argv.slice(2).some((arg) => arg !== '--preflight'))
  throw new Error('Cross-volume fixtures accept no supplied disk or library paths.');
if (!preflight)
  run(process.execPath, [
    'scripts/cargo.mjs',
    'build',
    '--locked',
    '-p',
    'stillharbour-core',
    '--features',
    'fixture-probes,copy-workflow',
    '--example',
    'cross_volume_probe',
  ]);
const probe = resolve(
  process.env.CARGO_TARGET_DIR ?? 'target',
  'debug',
  'examples',
  `cross_volume_probe${process.platform === 'win32' ? '.exe' : ''}`,
);
if (process.platform === 'win32') {
  run(
    join(
      process.env.SystemRoot ?? 'C:\\Windows',
      'System32',
      'WindowsPowerShell',
      'v1.0',
      'powershell.exe',
    ),
    [
      '-NoProfile',
      '-NonInteractive',
      '-ExecutionPolicy',
      'RemoteSigned',
      '-File',
      resolve('scripts/cross-volume/windows.ps1'),
      '-Probe',
      probe,
      ...(preflight ? ['-PreflightOnly'] : []),
    ],
    { env: { PSModulePath: undefined } },
  );
} else {
  run('python3', [
    'scripts/cross-volume/macos.py',
    '--probe',
    probe,
    ...(preflight ? ['--preflight'] : []),
  ]);
}
