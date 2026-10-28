import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { JobDetail, LibraryStatus, ScanJob } from '../bindings';
import { errorMessage, library, libraryData } from '../ipc';
import { ScanJobProgress } from './ScanJobProgress';

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
    status?.jobs.filter(
      (job) =>
        job.state === 'queued' || job.state === 'running' || job.state === 'cancel_requested',
    ) ?? [];
  const recent =
    status?.jobs.filter(
      (job) =>
        job.state !== 'queued' && job.state !== 'running' && job.state !== 'cancel_requested',
    ) ?? [];
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
      if (event.key !== 'Escape') return;
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
      <button
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
        {active.length > 0 && (
          <span className="jobs-count" aria-hidden="true">
            {active.length}
          </span>
        )}
      </button>
      <span id={`${id}-status`} className="screen-reader-status" role="status">
        {status
          ? `${active.length} running ${active.length === 1 ? 'job' : 'jobs'}`
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
              <button
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
              </button>
            </div>
            {!status ? (
              <p role="status">Connecting to jobs…</p>
            ) : (
              <>
                <p className="help">
                  {active.length
                    ? `${active.length} running ${active.length === 1 ? 'job' : 'jobs'}`
                    : 'No running jobs.'}
                </p>
                <ThumbnailJobAction status={status} onRefresh={onRefresh} />
                {entries(active)}
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
  const [canceling, setCanceling] = useState(false);
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
  }, [expanded, job.id]);
  async function cancel() {
    setCanceling(true);
    setCancelError(null);
    try {
      await library({ kind: 'cancel', job_id: job.id });
      await onRefresh();
    } catch (error) {
      setCancelError(errorMessage(error));
    } finally {
      setCanceling(false);
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
        <button
          aria-expanded={expanded}
          aria-controls={detailId}
          onClick={() => {
            setExpanded(!expanded);
            setDetail(null);
            setDetailError(null);
          }}
        >
          {expanded ? 'Hide details' : 'Details'}
        </button>
        {['queued', 'running', 'cancel_requested'].includes(job.state) && (
          <button
            disabled={canceling || job.state === 'cancel_requested'}
            onClick={() => void cancel()}
          >
            {canceling || job.state === 'cancel_requested'
              ? 'Stopping…'
              : job.kind === 'thumbnails'
                ? 'Cancel thumbnails'
                : 'Cancel scan'}
          </button>
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
        <select
          value={selected.id}
          onChange={(event) => setRootId(event.target.value)}
          disabled={pending}
        >
          {status.roots.map((root) => (
            <option value={root.id} key={root.id}>
              {root.label}
            </option>
          ))}
        </select>
      </label>
      <button onClick={() => void generate()} disabled={pending}>
        {pending ? 'Starting…' : 'Generate thumbnails'}
      </button>
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
        <ul>
          {Array.from(new Set(detail.issues)).map((issue) => (
            <li key={issue}>
              <bdi>{issue}</bdi>
            </li>
          ))}
        </ul>
      ) : (
        <p>No file issues recorded.</p>
      )}
    </div>
  );
}
