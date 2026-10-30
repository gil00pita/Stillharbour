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
    <section className="preparation-banner" aria-label="Library preparation">
      <div className="preparation-heading">
        <div>
          <strong>
            {job
              ? job.kind === 'thumbnails'
                ? 'Preparing photo previews'
                : 'Finding your photos'
              : 'Preview preparation paused'}
          </strong>
          <p>
            <bdi>{root.label}</bdi>
            {otherRoots > 0 && ` · ${otherRoots} more folders in Jobs`}
          </p>
        </div>
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
      {job ? (
        <>
          <ScanJobProgress job={job} label={root.label} />
          <p className="preparation-detail">
            {thumbnails && thumbnails.percentage !== undefined
              ? `${thumbnails.remaining.toLocaleString()} remaining${job.errors ? ` · ${job.errors.toLocaleString()} previews unavailable` : ''}`
              : job.kind === 'thumbnails'
                ? 'Waiting for the preview inventory. The remaining count is not known yet.'
                : `${job.discovered.toLocaleString()} files found · ${job.indexed.toLocaleString()} indexed. The remaining count is not known yet.`}
          </p>
        </>
      ) : (
        <p className="preparation-detail">
          {pausedMessage ? `${pausedMessage} ` : ''}Your indexed photos are kept. Resume to prepare
          the remaining previews.
        </p>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
