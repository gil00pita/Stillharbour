import type { LibraryStatus, ScanJob } from '../bindings';
import { copyJobActive } from '../library/copyJobs';

export function activeScanJob(job: ScanJob): boolean {
  return ['queued', 'running', 'cancel_requested', 'pause_requested', 'paused'].includes(job.state);
}

export function activeJobCount(status: LibraryStatus | null): number {
  if (!status) return 0;
  return (
    status.jobs.filter(activeScanJob).length +
    (status.copyJobs ?? []).filter(copyJobActive).length +
    (status.originalJobs ?? []).filter((job) => job.state === 'queued' || job.state === 'running')
      .length +
    (status.inspectionJobs ?? []).filter((job) =>
      ['queued', 'running', 'cancel_requested'].includes(job.state),
    ).length
  );
}
