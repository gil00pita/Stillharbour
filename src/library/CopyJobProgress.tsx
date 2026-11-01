import { useEffect, useRef, useState } from 'react';
import type { CopyWorkflowJob, CopyIoStatus, CopyJobState } from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { copyJobActive } from './copyJobs';

const labels: Record<CopyJobState, string> = {
  queued: 'Queued',
  running: 'Running',
  pause_requested: 'I/O pause requested',
  cancel_requested: 'Cancellation requested',
  prepared: 'Output names prepared',
  completed_verified: 'File bytes copied and verified',
  canceled_with_partial_copies: 'Canceled; output objects retained',
  needs_attention: 'Needs attention',
  needs_recovery: 'Recovery inspection required',
};
export function CopyJobProgress({
  job,
  onRefresh,
}: {
  job: CopyWorkflowJob;
  onRefresh?: () => Promise<void>;
}) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function cancel() {
    setPending(true);
    setError(null);
    try {
      await libraryData(
        { kind: 'organization_copy_cancel', job_id: job.id },
        'organization_copy_job',
      );
      await onRefresh?.();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
    }
  }
  return (
    <section
      className="job-row"
      aria-label={job.action === 'prepare_names' ? 'Output preparation job' : 'Verified copy job'}
    >
      <h3>{labels[job.state]}</h3>
      <p>
        {job.action === 'prepare_names'
          ? 'Preparing native names; no source bytes copied.'
          : `${job.verifiedFiles} of ${job.totalFiles} files byte-verified.`}
      </p>
      <p className="help">{job.message}</p>
      {job.error && (
        <p role="alert" className="error">
          {job.error.message}
        </p>
      )}
      {copyJobActive(job) && (
        <button
          type="button"
          disabled={pending || job.state === 'cancel_requested'}
          onClick={() => void cancel()}
        >
          {pending || job.state === 'cancel_requested' ? 'Stopping…' : 'Cancel organization job'}
        </button>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
export function CopyIoControls() {
  const [status, setStatus] = useState<CopyIoStatus | null>(null);
  const [rate, setRate] = useState('10240');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const epoch = useRef(0);
  useEffect(() => {
    const token = ++epoch.current;
    libraryData({ kind: 'organization_copy_io' }, 'organization_copy_io')
      .then((value) => {
        if (epoch.current === token) {
          setStatus(value);
          setRate((BigInt(value.aggregateBytesPerSecond) / 1024n).toString());
        }
      })
      .catch((error) => {
        if (epoch.current === token) setError(errorMessage(error));
      });
    return () => {
      epoch.current += 1;
    };
  }, []);
  async function update(paused: boolean | null) {
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    try {
      const bytes = paused === null ? (BigInt(rate) * 1024n).toString() : null;
      const value = await libraryData(
        {
          kind: 'organization_copy_set_io',
          aggregate_bytes_per_second: bytes,
          byte_io_paused: paused,
        },
        'organization_copy_io',
      );
      if (epoch.current === token) setStatus(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  return (
    <section aria-label="Operation I/O controls" aria-busy={pending}>
      <h4>Operation I/O</h4>
      <label>
        Aggregate KiB/s{' '}
        <input
          type="number"
          min="1"
          max="1048576"
          step="1"
          value={rate}
          onChange={(event) => setRate(event.target.value)}
          disabled={pending}
        />
      </label>
      <button type="button" disabled={pending || !status} onClick={() => void update(null)}>
        Apply I/O limit
      </button>
      <button
        type="button"
        disabled={pending || !status}
        onClick={() => void update(!status?.byteIoPaused)}
      >
        {status?.byteIoPaused ? 'Resume byte I/O' : 'Pause byte I/O'}
      </button>
      <p className="help">
        Reads, writes and verification share this limit. Pause takes effect at the next admitted
        byte boundary. Metadata, flushes and external helpers have separate limits.
      </p>
      {status && (
        <p>
          {BigInt(status.aggregateBytesPerSecond).toLocaleString()} bytes/s ·{' '}
          {status.byteIoPaused ? 'byte I/O pause requested' : 'byte I/O enabled'}
        </p>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
