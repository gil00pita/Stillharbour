import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { lstatSync, readFileSync, realpathSync } from 'node:fs';
import { deflateSync } from 'node:zlib';

assert.equal(
  process.argv.length,
  3,
  'Supply only the built application executable, never a library',
);
const executable = realpathSync(process.argv[2]);
assert.ok(lstatSync(executable).isFile(), 'Application executable must be a regular file');
const digest = () => createHash('sha256').update(readFileSync(executable)).digest('hex');
const before = digest();

function child(args, input, expectedStatus = 0) {
  const result = spawnSync(executable, args, {
    input,
    timeout: 15000,
    maxBuffer: 8 * 1024 * 1024,
    windowsHide: true,
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null, `Child interrupted: ${args[0]}`);
  assert.equal(result.status, expectedStatus, `Unexpected child exit: ${args[0]}`);
  return result.stdout.toString('utf8');
}

function chunk(type, payload) {
  const body = Buffer.concat([Buffer.from(type, 'ascii'), payload]);
  let crc = 0xffffffff;
  for (const byte of body) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  const length = Buffer.alloc(4);
  length.writeUInt32BE(payload.length);
  const checksum = Buffer.alloc(4);
  checksum.writeUInt32BE((crc ^ 0xffffffff) >>> 0);
  return Buffer.concat([length, body, checksum]);
}

// Generate a two-pixel RGB PNG in memory. No source/catalog paths are accepted
// or created, and every invocation uses an early worker/diagnostic entry point.
const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const header = Buffer.alloc(13);
header.writeUInt32BE(2, 0);
header.writeUInt32BE(1, 4);
header[8] = 8;
header[9] = 2;
const png = Buffer.concat([
  signature,
  chunk('IHDR', header),
  chunk('IDAT', deflateSync(Buffer.from([0, 20, 80, 50, 120, 140, 220]))),
  chunk('IEND', Buffer.alloc(0)),
]);

let receipt;
try {
  const sqlite = JSON.parse(child(['--diagnostic-sqlite']));
  assert.equal(sqlite.userCatalogOpened, false);
  assert.equal(typeof sqlite.version, 'string');
  assert.equal(typeof sqlite.sourceId, 'string');
  assert.ok(Array.isArray(sqlite.compileOptions) && sqlite.compileOptions.length > 0);
  const valid = JSON.parse(child(['--media-worker-stdin', '256'], png));
  assert.ok(valid.Ok && !valid.Err, 'Generated PNG did not produce a successful worker result');
  assert.deepEqual(
    [
      valid.Ok.source_width,
      valid.Ok.source_height,
      valid.Ok.thumbnail_width,
      valid.Ok.thumbnail_height,
    ],
    [2, 1, 2, 1],
  );
  assert.ok(
    Array.isArray(valid.Ok.png) &&
      valid.Ok.png.length >= 24 &&
      valid.Ok.png.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255),
  );
  const output = Buffer.from(valid.Ok.png);
  assert.deepEqual(output.subarray(0, 8), signature);
  assert.equal(output.readUInt32BE(16), 2);
  assert.equal(output.readUInt32BE(20), 1);
  const malformed = JSON.parse(
    child(['--media-worker-stdin', '256'], Buffer.from('generated malformed image')),
  );
  assert.ok(malformed.Err && !malformed.Ok, 'Malformed input was not returned as a typed error');
  assert.equal(typeof malformed.Err.code, 'string');
  assert.equal(typeof malformed.Err.message, 'string');
  assert.equal(malformed.Err.cause, undefined, 'Private error cause crossed worker output');
  child(['--media-worker-stdin', '999999'], png, 1);
  child(['--media-worker-unrecognized'], Buffer.alloc(0), 2);
  child(['--diagnostic-sqlite', 'forbidden-catalog-argument'], Buffer.alloc(0), 2);
  receipt = {
    kind: 'release-worker-memory-smoke',
    platform: process.platform,
    architecture: process.arch,
    executableSha256: before,
    sqlite,
    generatedPng: { source: [2, 1], output: [2, 1], encodedBytes: output.length },
    malformedInputRejected: true,
    invalidArgumentsRejected: true,
    userCatalogOpened: false,
    scope:
      'Unsigned executable worker/SQLite only; no GUI, installer, helper or restore acceptance',
  };
} finally {
  assert.equal(digest(), before, 'The inspected application executable changed during the probe');
}
console.log(JSON.stringify(receipt, null, 2));
