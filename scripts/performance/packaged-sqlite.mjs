// A fixed workspace-built binary and a read-only, in-memory SQLite diagnostic flag.
import { execFileSync } from 'node:child_process';
import { createReadStream } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
const executable =
  process.platform === 'darwin'
    ? 'target/release/bundle/macos/Stillharbour.app/Contents/MacOS/stillharbour'
    : 'target/release/stillharbour.exe';
if (!['darwin', 'win32'].includes(process.platform))
  throw new Error('No native packaged fixture for this host.');
const digest = createHash('sha256');
for await (const chunk of createReadStream(executable)) digest.update(chunk);
const info = JSON.parse(
  execFileSync(executable, ['--diagnostic-sqlite'], {
    encoding: 'utf8',
    timeout: 10000,
    maxBuffer: 65536,
  }),
);
if (
  info.userCatalogOpened !== false ||
  !/^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d [a-f0-9]{64}$/.test(info.sourceId)
)
  throw new Error('Unexpected provenance response.');
const result = {
  ...info,
  binarySha256: digest.digest('hex'),
  profile: 'release',
  run: randomUUID(),
};
await mkdir('.context/evidence', { recursive: true });
await writeFile(
  `.context/evidence/performance-packaged-sqlite-${result.run}.json`,
  `${JSON.stringify(result, null, 2)}\n`,
  { flag: 'wx' },
);
console.log(JSON.stringify(result));
