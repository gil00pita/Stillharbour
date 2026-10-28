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
