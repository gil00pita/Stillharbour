import { Button, Select } from '../components/ui/controls';
import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { JobDetail, LibraryStatus, ScanJob } from '../bindings';
import { errorMessage, library, libraryData } from '../ipc';
import { ScanJobProgress } from './ScanJobProgress';
import { LibraryChanges } from './LibraryChanges';
import { CopyJobProgress, CopyIoControls } from './CopyJobProgress';
import { copyJobActive } from './copyJobs';
import { RetainedCopyInspection } from './RetainedCopyInspection';
import { ReopenPreparedCopy } from './VerifiedCopyWorkflow';
import { OriginalJobProgress } from './OriginalJobProgress';
import { OriginalHistoryReview } from './OriginalHistoryReview';
import { InspectionJobProgress } from '../inspection/InspectionJobProgress';

export function JobsDropdown({
  status,
  onRefresh,
}: {
  status: LibraryStatus | null;
  onRefresh: () => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const opener = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDialogElement>(null);
  const id = useId();
  const [position, setPosition] = useState({ top: 64, right: 16, maxHeight: 400 });
  const active =
    status?.jobs.filter((job) =>
      ['queued', 'running', 'cancel_requested', 'pause_requested', 'paused'].includes(job.state),
    ) ?? [];
  const recent =
    status?.jobs.filter(
      (job) =>
        !['queued', 'running', 'cancel_requested', 'pause_requested', 'paused'].includes(job.state),
    ) ?? [];
  const copyJobs = status?.copyJobs ?? [];
  const activeCopies = copyJobs.filter(copyJobActive);
  const originalJobs = status?.originalJobs ?? [];
  const inspections = status?.inspectionJobs ?? [];
  const activeCount =
    inspections.filter((job) => ['queued', 'running', 'cancel_requested'].includes(job.state))
      .length +
    active.length +
    activeCopies.length +
    originalJobs.filter((job) => job.state === 'queued' || job.state === 'running').length;
  useLayoutEffect(() => {
    if (!open) return;
    function place() {
      const rect = opener.current?.getBoundingClientRect();
      if (!rect) return;
      const top = Math.max(16, Math.min(rect.bottom + 8, window.innerHeight - 160));
      setPosition({
        top,
        right: Math.min(
          Math.max(16, window.innerWidth - rect.right),
          Math.max(16, window.innerWidth - 406),
        ),
        maxHeight: Math.max(120, window.innerHeight - top - 16),
      });
    }
    function outside(event: Event) {
      if (
        event.target instanceof Node &&
        !panel.current?.contains(event.target) &&
        !opener.current?.contains(event.target)
      )
        setOpen(false);
    }
    function escape(event: KeyboardEvent) {
      if (event.key !== 'Escape' || panel.current?.querySelector('dialog[open]')) return;
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
      opener.current?.focus();
    }
    function scroll(event: Event) {
      if (!(event.target instanceof Node) || !panel.current?.contains(event.target)) place();
    }
    place();
    panel.current?.querySelector('h2')?.focus();
    document.addEventListener('pointerdown', outside, true);
    document.addEventListener('focusin', outside);
    document.addEventListener('keydown', escape, true);
    document.addEventListener('scroll', scroll, true);
    window.addEventListener('resize', place);
    return () => {
      document.removeEventListener('pointerdown', outside, true);
      document.removeEventListener('focusin', outside);
      document.removeEventListener('keydown', escape, true);
      document.removeEventListener('scroll', scroll, true);
      window.removeEventListener('resize', place);
    };
  }, [open]);
  const entries = (jobs: ScanJob[]) =>
    jobs.map((job) => (
      <JobEntry
        key={job.id}
        job={job}
        label={status?.roots.find((root) => root.id === job.rootId)?.label ?? 'Library'}
        onRefresh={onRefresh}
      />
    ));
  return (
    <>
      <Button
        ref={opener}
        className="jobs-trigger"
        aria-label="Jobs"
        title="Jobs"
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-controls={id}
        aria-describedby={`${id}-status`}
        onClick={() => setOpen(!open)}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M8 6V3h8v3M3 6h18v14H3ZM3 11h18M10 10v4h4v-4"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinejoin="round"
          />
        </svg>
        {activeCount > 0 && (
          <span className="jobs-count" aria-hidden="true">
            {activeCount}
          </span>
        )}
      </Button>
      <span id={`${id}-status`} className="screen-reader-status" role="status">
        {status
          ? `${activeCount} running ${activeCount === 1 ? 'job' : 'jobs'}`
          : 'Connecting to jobs'}
      </span>
      {open &&
        createPortal(
          <dialog
            open
            ref={panel}
            id={id}
            className="jobs-dropdown"
            aria-labelledby={`${id}-title`}
            style={position}
          >
            <div className="section-heading">
              <h2 id={`${id}-title`} tabIndex={-1}>
                Jobs
              </h2>
              <Button
                className="icon-button"
                aria-label="Close jobs"
                onClick={() => {
                  setOpen(false);
                  opener.current?.focus();
                }}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <path d="m6 6 12 12M18 6 6 18" stroke="currentColor" strokeWidth="1.5" />
                </svg>
              </Button>
            </div>
            {!status ? (
              <p role="status">Connecting to jobs…</p>
            ) : (
              <>
                <p className="help">
                  {activeCount
                    ? `${activeCount} running ${activeCount === 1 ? 'job' : 'jobs'}`
                    : 'No running jobs.'}
                </p>
                <ThumbnailJobAction status={status} onRefresh={onRefresh} />
                {entries(active)}
                {activeCopies.map((job) => (
                  <CopyJobProgress key={job.id} job={job} onRefresh={onRefresh} />
                ))}
                {activeCopies.some((job) => job.action === 'copy_files') && <CopyIoControls />}
                {copyJobs
                  .filter((job) => !copyJobActive(job))
                  .map((job) => (
                    <div key={job.id}>
                      <CopyJobProgress job={job} onRefresh={onRefresh} />
                      {job.action === 'prepare_names' && !job.readyReviewId && (
                        <ReopenPreparedCopy job={job} />
                      )}
                      {(job.action === 'copy_files' || job.readyReviewId) && (
                        <RetainedCopyInspection job={job} onRefresh={onRefresh} />
                      )}
                    </div>
                  ))}
                {inspections.map((job) => (
                  <InspectionJobProgress key={job.id} job={job} onRefresh={onRefresh} />
                ))}
                <LibraryChanges status={status} />
                {originalJobs.map((job) => (
                  <div key={job.id}>
                    <OriginalJobProgress job={job} />
                    <OriginalHistoryReview job={job} />
                  </div>
                ))}
                {recent.length > 0 && (
                  <details className="recent-jobs">
                    <summary>Recent jobs ({recent.length})</summary>
                    {entries(recent)}
                  </details>
                )}
              </>
            )}
          </dialog>,
          document.body,
        )}
    </>
  );
}

function JobEntry({
  job,
  label,
  onRefresh,
}: {
  job: ScanJob;
  label: string;
  onRefresh: () => Promise<void>;
}) {
  const [expanded, setExpanded] = useState(false);
  const [detail, setDetail] = useState<JobDetail | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [cancelError, setCancelError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<
    'cancel' | 'pause' | 'resume' | 'retry' | null
  >(null);
  const canceling = pendingAction !== null;
  const detailId = useId();
  useEffect(() => {
    if (!expanded) return;
    let active = true;
    libraryData({ kind: 'job_details', job_id: job.id }, 'job_details')
      .then((value) => {
        if (active) setDetail(value);
      })
      .catch((error) => {
        if (active) setDetailError(errorMessage(error));
      });
    return () => {
      active = false;
    };
  }, [expanded, job.id, job.skipped, job.errors, job.state]);
  async function action(kind: 'cancel' | 'pause' | 'resume' | 'retry') {
    setPendingAction(kind);
    setCancelError(null);
    try {
      await library({ kind, job_id: job.id });
      await onRefresh();
    } catch (error) {
      setCancelError(errorMessage(error));
    } finally {
      setPendingAction(null);
    }
  }
  return (
    <section
      data-job-kind={job.kind ?? 'scan'}
      className="job-row"
      aria-label={`${job.kind === 'thumbnails' ? 'Thumbnail' : 'Scan'} job for ${label}`}
    >
      <h3>
        <bdi>{label}</bdi>
      </h3>
      <ScanJobProgress job={job} label={label} />
      <JobCounts job={job} />
      <p className="help">{job.message}</p>
      <div className="job-actions">
        <Button
          aria-expanded={expanded}
          aria-controls={detailId}
          onClick={() => {
            setExpanded(!expanded);
            setDetail(null);
            setDetailError(null);
          }}
        >
          {expanded ? 'Hide details' : 'Details'}
        </Button>
        {['queued', 'running', 'pause_requested', 'paused'].includes(job.state) && (
          <Button
            disabled={canceling || job.state === 'pause_requested'}
            onClick={() => void action(job.state === 'paused' ? 'resume' : 'pause')}
          >
            {pendingAction === 'resume'
              ? 'Resuming…'
              : pendingAction === 'pause'
                ? 'Pausing…'
                : job.state === 'paused'
                  ? 'Resume'
                  : job.state === 'pause_requested'
                    ? 'Pausing…'
                    : 'Pause'}
          </Button>
        )}
        {!['queued', 'running', 'cancel_requested', 'pause_requested', 'paused'].includes(
          job.state,
        ) && (
          <Button disabled={canceling} onClick={() => void action('retry')}>
            {pendingAction === 'retry'
              ? 'Retrying…'
              : `Retry ${job.kind === 'thumbnails' ? 'thumbnails' : 'scan'}`}
          </Button>
        )}
        {['queued', 'running', 'cancel_requested', 'pause_requested', 'paused'].includes(
          job.state,
        ) && (
          <Button
            disabled={pendingAction === 'cancel' || job.state === 'cancel_requested'}
            onClick={() => void action('cancel')}
          >
            {pendingAction === 'cancel' || job.state === 'cancel_requested'
              ? 'Stopping…'
              : job.kind === 'thumbnails'
                ? 'Cancel thumbnails'
                : 'Cancel scan'}
          </Button>
        )}
      </div>
      {cancelError && (
        <p className="error" role="alert">
          {cancelError}
        </p>
      )}
      {expanded && <JobDetails id={detailId} detail={detail} error={detailError} />}
    </section>
  );
}

function ThumbnailJobAction({
  status,
  onRefresh,
}: {
  status: LibraryStatus;
  onRefresh: () => Promise<void>;
}) {
  const [rootId, setRootId] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const selected = status.roots.find((root) => root.id === rootId) ?? status.roots[0];
  if (!selected) return null;
  async function generate() {
    if (!selected) return;
    setPending(true);
    setError(null);
    try {
      await library({ kind: 'generate_thumbnails', root_id: selected.id });
      await onRefresh();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
    }
  }
  return (
    <div className="thumbnail-job-action">
      <label>
        Preview library
        <Select
          value={selected.id}
          onChange={(event) => setRootId(event.target.value)}
          disabled={pending}
        >
          {status.roots.map((root) => (
            <option value={root.id} key={root.id}>
              {root.label}
            </option>
          ))}
        </Select>
      </label>
      <Button onClick={() => void generate()} disabled={pending}>
        {pending ? 'Starting…' : 'Generate thumbnails'}
      </Button>
      <p className="help">Prepare tiny previews in the background. Originals stay read-only.</p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </div>
  );
}

function JobCounts({ job }: { job: ScanJob }) {
  return (
    <p className="job-counts">
      {job.kind === 'thumbnails' ? (
        <>
          {job.indexed.toLocaleString()} generated · {job.skipped.toLocaleString()} cached ·{' '}
          {job.errors.toLocaleString()} unavailable
        </>
      ) : (
        <>
          {job.discovered.toLocaleString()} found · {job.indexed.toLocaleString()} indexed ·{' '}
          {job.skipped.toLocaleString()} skipped · {job.errors.toLocaleString()} issues
        </>
      )}
    </p>
  );
}

function JobDetails({
  id,
  detail,
  error,
}: {
  id: string;
  detail: JobDetail | null;
  error: string | null;
}) {
  return (
    <div id={id} className="job-details">
      {error ? (
        <p className="error" role="alert">
          {error}
        </p>
      ) : !detail ? (
        <p role="status">Loading job details…</p>
      ) : detail.issues.length ? (
        <>
          <p className="help">
            Skip reasons and failures (up to 200 entries). Retry scans the saved scope again,
            reusing unchanged metadata. Change scan settings to include intentionally excluded
            files. Cached thumbnails are reused.
          </p>
          <ul>
            {Array.from(new Set(detail.issues)).map((issue) => (
              <li key={issue}>
                <bdi>{issue}</bdi>
              </li>
            ))}
          </ul>
        </>
      ) : (
        <p>No file issues recorded.</p>
      )}
    </div>
  );
}
