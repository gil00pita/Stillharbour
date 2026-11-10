import { expect, it } from 'vitest';
import { managedDestinationReady } from './managed-destination.mjs';

const fixture = { rootId: 'destination', jobId: 'discovery' };
function status(preparationState) {
  return {
    roots: [{ id: fixture.rootId, availability: 'online', preparingPreviews: false }],
    jobs: [
      { id: fixture.jobId, rootId: fixture.rootId, state: 'completed' },
      ...(preparationState
        ? [
            {
              id: `thumbnail:initial:${fixture.rootId}`,
              rootId: fixture.rootId,
              state: preparationState,
            },
          ]
        : []),
    ],
  };
}

it.each([undefined, 'queued', 'running', 'paused', 'failed', 'partial'])(
  'does not build a destination preview while initial preparation is %s',
  (state) => expect(managedDestinationReady(status(state), fixture)).toBe(false),
);
it('admits the generated destination after both workers finish', () => {
  expect(managedDestinationReady(status('completed'), fixture)).toBe(true);
});
it('keeps discovery, root availability and other active work as prerequisites', () => {
  const snapshot = status('completed');
  snapshot.jobs[0].state = 'running';
  expect(managedDestinationReady(snapshot, fixture)).toBe(false);
  snapshot.jobs[0].state = 'completed';
  snapshot.roots[0].availability = 'offline';
  expect(managedDestinationReady(snapshot, fixture)).toBe(false);
  snapshot.roots[0].availability = 'online';
  snapshot.roots[0].preparingPreviews = true;
  expect(managedDestinationReady(snapshot, fixture)).toBe(false);
  snapshot.roots[0].preparingPreviews = false;
  snapshot.jobs.push({ id: 'rescan', rootId: fixture.rootId, state: 'running' });
  expect(managedDestinationReady(snapshot, fixture)).toBe(false);
  snapshot.jobs.at(-1).rootId = 'another-library';
  expect(managedDestinationReady(snapshot, fixture)).toBe(false);
});
it('does not accept a different root’s completed preview job', () => {
  const snapshot = status('completed');
  snapshot.jobs[1].id = 'thumbnail:initial:another-library';
  expect(managedDestinationReady(snapshot, fixture)).toBe(false);
});
