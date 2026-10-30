import { useId } from 'react';
import type { ScanJob } from '../bindings';
import { thumbnailProgress, thumbnailDescription } from './jobProgress';

const descriptions: Record<string, string> = {
  running: 'Scanning · total files not yet known',
  cancel_requested: 'Stopping scan…',
  completed: 'Scan complete',
  partial: 'Scan finished with issues',
  canceled: 'Scan canceled',
  interrupted: 'Scan interrupted',
  failed: 'Scan failed',
};

export function ScanJobProgress({
  job,
  label,
  compact = false,
}: {
  job: ScanJob;
  label: string;
  compact?: boolean;
}) {
  const descriptionId = useId();
  if (job.kind === 'thumbnails')
    return (
      <ThumbnailProgress job={job} label={label} descriptionId={descriptionId} compact={compact} />
    );
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
      <p id={descriptionId} className="help" hidden={compact}>
        {description}
      </p>
    </div>
  );
}

function ThumbnailProgress({
  job,
  label,
  descriptionId,
  compact,
}: {
  job: ScanJob;
  label: string;
  descriptionId: string;
  compact: boolean;
}) {
  const active =
    job.state === 'running' || job.state === 'cancel_requested' || job.state === 'queued';
  const { processed, percentage } = thumbnailProgress(job);
  const completed = job.state === 'completed';
  const value = completed ? 100 : percentage;
  const indeterminate = value === undefined;
  const description = thumbnailDescription(job, processed, percentage);
  const showProgress = active || completed;
  return (
    <div className="job-progress">
      <div
        className={`job-progress-track ${showProgress ? (indeterminate ? 'is-active' : 'is-complete') : ''}`}
        role={showProgress ? 'progressbar' : undefined}
        aria-hidden={showProgress ? undefined : true}
        aria-label={`Thumbnail progress for ${label}`}
        aria-describedby={descriptionId}
        aria-valuetext={description}
        aria-valuenow={value}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <span
          className="job-progress-fill"
          style={indeterminate ? undefined : { width: `${value}%` }}
        />
      </div>
      <p id={descriptionId} className="help" hidden={compact}>
        {description}
      </p>
    </div>
  );
}
