import type { ScanJob } from '../bindings';

export function thumbnailProgress(job: ScanJob) {
  const total = job.discovered;
  const processed = Math.min(total, job.indexed + job.skipped + job.errors);
  return {
    total,
    processed,
    remaining: total - processed,
    percentage:
      job.state !== 'queued' && total > 0 ? Math.floor((processed / total) * 100) : undefined,
  };
}

export function thumbnailDescription(
  job: ScanJob,
  processed: number,
  percentage: number | undefined,
) {
  const descriptions: Record<string, string> = {
    queued: 'Thumbnail generation queued',
    running:
      percentage === undefined
        ? 'Counting photos for preview preparation…'
        : `Preparing thumbnails · ${processed.toLocaleString()} of ${job.discovered.toLocaleString()}`,
    cancel_requested: 'Stopping thumbnail generation…',
    completed: 'Thumbnails complete',
    partial: 'Thumbnails finished with unavailable items',
    canceled: 'Thumbnail generation canceled',
    interrupted: 'Thumbnail generation interrupted',
    failed: 'Thumbnail generation failed',
  };
  return descriptions[job.state] ?? 'Thumbnail generation not running';
}
