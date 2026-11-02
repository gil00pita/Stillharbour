import assert from 'node:assert/strict';
import { existsSync, linkSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { cleanupSessionOwner, createSessionOwner, verifySessionOwner } from './session-owner.mjs';

test('native-session cleanup refuses another owner or directory identity', () => {
  const session = createSessionOwner();
  try {
    assert.throws(() => cleanupSessionOwner({ ...session, ownerNonce: '0'.repeat(32) }));
    assert.throws(() =>
      cleanupSessionOwner({
        ...session,
        directoryIdentity: { ...session.directoryIdentity, ino: '0' },
      }),
    );
    assert.ok(existsSync(session.root));
    verifySessionOwner(session);
  } finally {
    cleanupSessionOwner(session);
  }
  assert.equal(existsSync(session.root), false);
});
test('native-session cleanup refuses a multiply-linked owner marker', () => {
  const session = createSessionOwner();
  const extra = join(session.root, 'extra-owner-link');
  try {
    linkSync(join(session.root, 'session-owner.json'), extra);
    assert.throws(() => cleanupSessionOwner(session));
    assert.ok(existsSync(session.root));
    unlinkSync(extra);
    verifySessionOwner(session);
  } finally {
    if (existsSync(extra)) unlinkSync(extra);
    cleanupSessionOwner(session);
  }
});
