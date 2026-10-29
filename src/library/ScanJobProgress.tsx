import { useId } from 'react';
import type { ScanJob } from '../bindings';

const descriptions: Record<string, string> = {
  running: 'Scanning · total files not yet known',
  cancel_requested: 'Stopping scan…',
  completed: 'Scan complete',
  partial: 'Scan finished with issues',
  canceled: 'Scan canceled',
  interrupted: 'Scan interrupted',
  failed: 'Scan failed',
};

export function ScanJobProgress({ job, label }: { job: ScanJob; label: string }) {
  const descriptionId = useId();
  if (job.kind === 'thumbnails')
    return <ThumbnailProgress job={job} label={label} descriptionId={descriptionId} />;
  const active = job.state === 'running' || job.state === 'cancel_requested';
  const completed = job.state === 'completed';
  const description = descriptions[job.state] ?? 'Scan not running';
  return (
    <div className="job-progress">
      {active || completed ? (
        <div
          className={`job-progress-track ${active ? 'is-active' : 'is-complete'}`}
          role="progressbar"
          aria-label={`Scan progress for ${label}`}
          aria-describedby={descriptionId}
          aria-valuetext={description}
          aria-valuenow={completed ? 100 : undefined}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <span className="job-progress-fill" />
        </div>
      ) : (
        <div className="job-progress-track" aria-hidden="true" />
      )}
      <p id={descriptionId} className="help">
        {description}
      </p>
    </div>
  );
}

function ThumbnailProgress({
  job,
  label,
  descriptionId,
}: {
  job: ScanJob;
  label: string;
  descriptionId: string;
}) {
  const active =
    job.state === 'running' || job.state === 'cancel_requested' || job.state === 'queued';
  const processed = Math.min(job.discovered, job.indexed + job.skipped + job.errors);
  const percentage = job.discovered > 0 ? Math.floor((processed / job.discovered) * 100) : 0;
  const descriptions: Record<string, string> = {
    queued: 'Thumbnail generation queued',
    running: `Preparing thumbnails · ${processed.toLocaleString()} of ${job.discovered.toLocaleString()}`,
    cancel_requested: 'Stopping thumbnail generation…',
    completed: 'Thumbnails complete',
    partial: 'Thumbnails finished with unavailable items',
    canceled: 'Thumbnail generation canceled',
    interrupted: 'Thumbnail generation interrupted',
    failed: 'Thumbnail generation failed',
  };
  const description = descriptions[job.state] ?? 'Thumbnail generation not running';
  return (
    <div className="job-progress">
      {active || job.state === 'completed' ? (
        <div
          className="job-progress-track is-complete"
          role="progressbar"
          aria-label={`Thumbnail progress for ${label}`}
          aria-describedby={descriptionId}
          aria-valuetext={description}
          aria-valuenow={
            job.state === 'completed' ? 100 : job.state === 'queued' ? undefined : percentage
          }
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <span
            className="job-progress-fill"
            style={{ width: `${job.state === 'completed' ? 100 : percentage}%` }}
          />
        </div>
      ) : (
        <div className="job-progress-track" aria-hidden="true" />
      )}
      <p id={descriptionId} className="help">
        {description}
      </p>
    </div>
  );
}
