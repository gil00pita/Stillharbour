import { useState } from 'react';
import type { InspectionJob } from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { displayBytes } from './format';
export function InspectionJobProgress({
  job,
  onRefresh,
}: {
  job: InspectionJob;
  onRefresh: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const active = ['queued', 'running', 'cancel_requested'].includes(job.state);
  async function cancel() {
    setBusy(true);
    setError(null);
    try {
      await libraryData({ kind: 'inspection_cancel', job_id: job.id }, 'inspection_job');
      await onRefresh();
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="job-row" aria-label={`${job.input.task.kind} inspection job`}>
      <h3>
        {job.input.task.kind === 'duplicates'
          ? 'Duplicate'
          : job.input.task.kind === 'health'
            ? 'Integrity'
            : 'Storage'}{' '}
        inspection · {job.state.replaceAll('_', ' ')}
      </h3>
      <p>{job.message}</p>
      <p>
        {job.filesObserved} observed · {job.filesChecked} examined · {displayBytes(job.bytesRead)}{' '}
        read · {job.issues} issues
      </p>
      {active && (
        <>
          <progress aria-label="Inspection in progress" />
          <button disabled={busy || job.state === 'cancel_requested'} onClick={() => void cancel()}>
            Cancel inspection
          </button>
        </>
      )}
      <p>Open Duplicates, Integrity or Storage to review the retained scope and results.</p>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
