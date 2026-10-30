import { useState } from 'react';
import type { LibraryRoot, LibraryStatus, ScanJob } from '../bindings';
import { errorMessage, library } from '../ipc';
import { ScanJobProgress } from './ScanJobProgress';
import { thumbnailProgress } from './jobProgress';

const activeStates = new Set(['queued', 'running', 'cancel_requested']);

export function PreparationBanner({
  status,
  rootId,
  onRefresh,
}: {
  status: LibraryStatus;
  rootId: string | null;
  onRefresh: () => Promise<void>;
}) {
  const roots = status.roots.filter(
    (root) =>
      (!rootId || root.id === rootId) &&
      (root.preparingPreviews ||
        status.jobs.some((job) => job.rootId === root.id && activeStates.has(job.state))),
  );
  const jobs = status.jobs.filter(
    (job) => roots.some((root) => root.id === job.rootId) && activeStates.has(job.state),
  );
  const job =
    jobs.find((job) => job.kind !== 'thumbnails') ??
    jobs.find((job) => job.state === 'running') ??
    jobs[0];
  const root = roots.find((root) => root.id === job?.rootId) ?? roots[0];
  if (!root) return null;
  return (
    <PreparationStatus
      key={`${root.id}:${job?.id ?? 'paused'}`}
      root={root}
      job={job}
      otherRoots={roots.length - 1}
      pausedMessage={
        status.jobs.find(
          (item) =>
            item.rootId === root.id && item.kind === 'thumbnails' && !activeStates.has(item.state),
        )?.message
      }
      onRefresh={onRefresh}
    />
  );
}

function PreparationStatus({
  root,
  job,
  otherRoots,
  pausedMessage,
  onRefresh,
}: {
  root: LibraryRoot;
  job: ScanJob | undefined;
  otherRoots: number;
  pausedMessage: string | undefined;
  onRefresh: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const thumbnails = job?.kind === 'thumbnails' ? thumbnailProgress(job) : null;
  const stopping = job?.state === 'cancel_requested';
  async function act() {
    setBusy(true);
    setError('');
    try {
      await library(
        job
          ? { kind: 'cancel', job_id: job.id }
          : { kind: 'generate_thumbnails', root_id: root.id },
      );
      try {
        await onRefresh();
      } catch (failure) {
        setError(`Request accepted. Could not refresh progress: ${errorMessage(failure)}`);
      }
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section
      className="preparation-banner"
      aria-label="Library preparation"
      title={`${root.label}${otherRoots > 0 ? ` · ${otherRoots} more folders in Jobs` : ''}`}
    >
      <div className="preparation-row">
        <span className="preparation-summary">
          {!job
            ? 'Preview preparation paused'
            : thumbnails?.percentage !== undefined
              ? `${thumbnails.remaining.toLocaleString()} remaining`
              : 'Remaining unknown'}
        </span>
        {job ? <ScanJobProgress job={job} label={root.label} compact /> : <span />}
        <button disabled={busy || stopping} onClick={() => void act()}>
          {busy
            ? 'Updating…'
            : stopping
              ? 'Stopping…'
              : !job
                ? 'Resume previews'
                : job.kind === 'thumbnails'
                  ? 'Pause previews'
                  : 'Stop scan'}
        </button>
      </div>
      {job && job.errors > 0 && (
        <p className="preparation-detail">
          {job.errors.toLocaleString()} {thumbnails ? 'previews unavailable' : 'scan issues'} ·
          Details in Jobs
        </p>
      )}
      {!job && pausedMessage && <p className="preparation-detail">{pausedMessage}</p>}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
