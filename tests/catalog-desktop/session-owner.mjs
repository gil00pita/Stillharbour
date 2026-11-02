import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import {
  closeSync,
  existsSync,
  fsyncSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';

export const ROOT_ENV = 'STILLHARBOUR_CATALOG_NATIVE_SESSION_ROOT';
export const OWNER_ENV = 'STILLHARBOUR_CATALOG_NATIVE_SESSION_OWNER';
const marker = 'session-owner.json';
function identity(path) {
  const stat = lstatSync(path, { bigint: true });
  return { dev: String(stat.dev), ino: String(stat.ino) };
}
function readOwner(root) {
  const path = join(root, marker);
  const stat = lstatSync(path);
  assert.ok(
    stat.isFile() && !stat.isSymbolicLink() && stat.nlink === 1 && stat.size <= 512,
    'Owner marker must be a bounded single-link regular file',
  );
  const value = JSON.parse(readFileSync(path, 'utf8'));
  assert.deepEqual(Object.keys(value).sort(), ['ownerNonce', 'version']);
  assert.equal(value.version, 1);
  assert.match(value.ownerNonce, /^[a-f0-9]{32}$/);
  return value;
}
export function verifySessionOwner(session) {
  assert.match(session.ownerNonce, /^[a-f0-9]{32}$/);
  const stat = lstatSync(session.root);
  assert.ok(
    stat.isDirectory() && !stat.isSymbolicLink(),
    'Session root must be an actual directory',
  );
  const canonical = realpathSync.native(session.root);
  assert.equal(canonical, session.root, 'Session path must remain canonical');
  assert.equal(
    dirname(canonical),
    realpathSync.native(tmpdir()),
    'Session must be a direct native temporary child',
  );
  assert.ok(basename(canonical).startsWith(`stillharbour-m4-catalog-${session.ownerNonce}-`));
  assert.equal(readOwner(canonical).ownerNonce, session.ownerNonce);
  assert.deepEqual(
    identity(canonical),
    session.directoryIdentity,
    'Session native directory identity changed',
  );
  assert.deepEqual(
    identity(join(canonical, marker)),
    session.markerIdentity,
    'Session owner marker identity changed',
  );
  return session;
}
export function createSessionOwner() {
  const ownerNonce = randomBytes(16).toString('hex');
  const root = realpathSync.native(
    mkdtempSync(join(realpathSync.native(tmpdir()), `stillharbour-m4-catalog-${ownerNonce}-`)),
  );
  const file = openSync(join(root, marker), 'wx', 0o600);
  try {
    writeFileSync(file, JSON.stringify({ version: 1, ownerNonce }));
    fsyncSync(file);
  } finally {
    closeSync(file);
  }
  return verifySessionOwner({
    root,
    ownerNonce,
    directoryIdentity: identity(root),
    markerIdentity: identity(join(root, marker)),
  });
}
export function sessionFromEnvironment() {
  const requested = process.env[ROOT_ENV];
  const ownerNonce = process.env[OWNER_ENV];
  assert.ok(requested && ownerNonce, 'The catalog session must be created by its Node parent');
  const root = realpathSync.native(requested);
  return verifySessionOwner({
    root,
    ownerNonce,
    directoryIdentity: identity(root),
    markerIdentity: identity(join(root, marker)),
  });
}
export function evidenceDirectory(session) {
  verifySessionOwner(session);
  const directory = resolve('.context/m4-catalog-restart', session.ownerNonce);
  mkdirSync(directory, { recursive: true });
  return directory;
}
export function writePhaseReceipt(session, phase, value) {
  assert.ok([1, 2, 3].includes(phase));
  const body = JSON.stringify({ ownerNonce: session.ownerNonce, phase, ...value }, null, 2);
  assert.ok(Buffer.byteLength(body) <= 131072, 'Bounded native phase receipt');
  writeFileSync(join(evidenceDirectory(session), `phase-0${phase}.json`), body, {
    flag: 'wx',
    mode: 0o600,
  });
}
export function readPhaseReceipt(session, phase) {
  verifySessionOwner(session);
  const file = join(evidenceDirectory(session), `phase-0${phase}.json`);
  const stat = lstatSync(file);
  assert.ok(stat.isFile() && !stat.isSymbolicLink() && stat.nlink === 1 && stat.size <= 131072);
  const value = JSON.parse(readFileSync(file, 'utf8'));
  assert.equal(value.ownerNonce, session.ownerNonce);
  assert.equal(value.phase, phase);
  return value;
}
export function processExists(pid) {
  assert.ok(Number.isSafeInteger(pid) && pid > 0 && pid !== process.pid);
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    if (error.code === 'ESRCH') return false;
    throw error;
  }
}
export async function waitForProcessExit(pid, timeoutMs = 20000) {
  assert.ok(Number.isSafeInteger(timeoutMs) && timeoutMs > 0 && timeoutMs <= 60000);
  const until = Date.now() + timeoutMs;
  while (processExists(pid)) {
    assert.ok(Date.now() < until, 'Generated native application did not exit after graceful quit');
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
}
export function cleanupSessionOwner(session) {
  // All checks are repeated immediately before the only recursive deletion.
  verifySessionOwner(session);
  assert.ok(existsSync(join(session.root, marker)));
  rmSync(session.root, { recursive: true });
}
