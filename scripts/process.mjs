import { spawnSync } from 'node:child_process';
import { homedir } from 'node:os';
import { delimiter, join } from 'node:path';

export function run(command, args, options = {}) {
  const path = [join(homedir(), '.cargo', 'bin'), process.env.PATH ?? ''].join(delimiter);
  const result = spawnSync(command, args, {
    stdio: 'inherit',
    ...options,
    env: { ...process.env, PATH: path, ...options.env },
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
  return result;
}
