export function photoControlsReady(status) {
  if (status.roots.length !== 1) return false;
  const root = status.roots[0];
  if (root.availability !== 'online' || root.preparingPreviews) return false;
  // Invalid generated media can make both discovery and previews partial on
  // Windows. This is a settled UI fixture, not a successful integrity claim.
  const settled = (job) => job.rootId === root.id && ['completed', 'partial'].includes(job.state);
  return (
    status.jobs.some((job) => job.kind === 'scan' && settled(job)) &&
    status.jobs.some(
      (job) =>
        job.id === `thumbnail:initial:${root.id}` && job.kind === 'thumbnails' && settled(job),
    ) &&
    status.jobs.every(settled)
  );
}
