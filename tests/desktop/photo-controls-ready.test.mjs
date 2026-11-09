import assert from 'node:assert/strict';
import { it as test } from 'vitest';
import { photoControlsReady } from './photo-controls-ready.mjs';

function fixture(scanState = 'partial', previewState = 'partial') {
  // Sanitized from the generated Windows library's windows-status.json in CI
  // run 42649858582: both jobs finish partial with two invalid-media errors.
  return {
    roots: [{ id: 'generated-root', availability: 'online', preparingPreviews: false }],
    jobs: [
      {
        id: 'scan-generated-root',
        rootId: 'generated-root',
        kind: 'scan',
        state: scanState,
        errors: scanState === 'partial' ? 2 : 0,
      },
      {
        id: 'thumbnail:initial:generated-root',
        rootId: 'generated-root',
        kind: 'thumbnails',
        state: previewState,
        errors: previewState === 'partial' ? 2 : 0,
      },
    ],
  };
}

for (const scanState of ['completed', 'partial']) {
  for (const previewState of ['completed', 'partial']) {
    test(`settled generated media is ready with ${scanState} discovery and ${previewState} previews`, () => {
      assert.equal(photoControlsReady(fixture(scanState, previewState)), true);
    });
  }
}

for (const state of [
  'queued',
  'running',
  'pause_requested',
  'paused',
  'cancel_requested',
  'canceled',
  'failed',
  'interrupted',
]) {
  test(`does not measure photo controls while either preparation job is ${state}`, () => {
    assert.equal(photoControlsReady(fixture(state, 'completed')), false);
    assert.equal(photoControlsReady(fixture('completed', state)), false);
  });
}

test('requires the generated online root to finish preparation', () => {
  const status = fixture('completed', 'completed');
  status.roots[0].preparingPreviews = true;
  assert.equal(photoControlsReady(status), false);
  status.roots[0].preparingPreviews = false;
  status.roots[0].availability = 'offline';
  assert.equal(photoControlsReady(status), false);
  status.roots = [];
  assert.equal(photoControlsReady(status), false);
});

test('requires discovery and the exact root initial-preview job', () => {
  for (const index of [0, 1]) {
    const status = fixture('completed', 'completed');
    status.jobs.splice(index, 1);
    assert.equal(photoControlsReady(status), false);
  }
  const status = fixture('completed', 'completed');
  status.jobs[1].id = 'thumbnail:initial:other-root';
  status.jobs[1].rootId = 'other-root';
  assert.equal(photoControlsReady(status), false);
});

test('does not accept completed discovery belonging to a different root', () => {
  const status = fixture('completed', 'completed');
  status.jobs[0].rootId = 'other-root';
  assert.equal(photoControlsReady(status), false);
});
