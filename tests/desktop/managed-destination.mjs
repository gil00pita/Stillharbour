// Discovery finishes before the separate initial-preview worker is enqueued.
// A preview built in that gap is invalidated by the worker's catalog writes.
export function managedDestinationReady(status, { rootId, jobId }) {
  return (
    status.roots.some(
      (root) => root.id === rootId && root.availability === 'online' && !root.preparingPreviews,
    ) &&
    status.jobs.some((job) => job.id === jobId && job.state === 'completed') &&
    status.jobs.some(
      (job) => job.id === `thumbnail:initial:${rootId}` && job.state === 'completed',
    ) &&
    status.jobs.every(
      (job) =>
        !['queued', 'running', 'cancel_requested', 'pause_requested', 'paused'].includes(job.state),
    )
  );
}
