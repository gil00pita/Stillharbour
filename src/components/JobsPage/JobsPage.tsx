import { PageHeader } from '@/components/PageHeader';
import { Field, Box, Heading, List, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { NativeSelect } from '@/components/NativeSelect';
import { useEffect, useId, useRef, useState } from 'react';
import type { JobDetail, LibraryStatus, ScanJob } from '../../bindings';
import { errorMessage, library, libraryData } from '../../ipc';
import { ScanJobProgress } from '@/components/ScanJobProgress';
import { LibraryChanges } from '@/components/LibraryChanges';
import { CopyJobProgress, CopyIoControls } from '@/components/CopyJobProgress';
import { copyJobActive } from '../../library/copyJobs';
import { RetainedCopyInspection } from '@/components/RetainedCopyInspection';
import { ReopenPreparedCopy } from '@/components/VerifiedCopyWorkflow';
import { OriginalJobProgress } from '@/components/OriginalJobProgress';
import { OriginalHistoryReview } from '@/components/OriginalHistoryReview';
import { InspectionJobProgress } from '@/components/InspectionJobProgress';

import { activeJobCount, activeScanJob } from '../../utils/job-activity';
export function JobsPage({
  status,
  onRefresh,
  onBack,
}: {
  status: LibraryStatus | null;
  onRefresh: () => Promise<void>;
  onBack: () => void;
}) {
  const heading = useRef<HTMLHeadingElement>(null);
  const id = useId();
  useEffect(() => {
    heading.current?.focus();
  }, []);
  const active = status?.jobs.filter(activeScanJob) ?? [];
  const recent = status?.jobs.filter((job) => !activeScanJob(job)) ?? [];
  const copyJobs = status?.copyJobs ?? [];
  const activeCopies = copyJobs.filter(copyJobActive);
  const originalJobs = status?.originalJobs ?? [];
  const inspections = status?.inspectionJobs ?? [];
  const activeCount = activeJobCount(status);
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
    <chakra.section className="jobs-page" aria-labelledby={id}>
      <PageHeader>
        <chakra.header className="toolbar jobs-heading">
          <Box>
            <Heading as="h1" unstyled id={id} ref={heading} tabIndex={-1}>
              Jobs
            </Heading>
          </Box>
          <Box className="header-actions">
            <Button type="button" aria-label="Back from Jobs" onClick={onBack}>
              Back
            </Button>
          </Box>
        </chakra.header>
      </PageHeader>
      <Box className="jobs-content">
        <Text className="help">Follow background work and review recent outcomes.</Text>
        {!status ? (
          <Text role="status">Connecting to jobs…</Text>
        ) : (
          <>
            <Text className="help">
              {activeCount
                ? `${activeCount} running ${activeCount === 1 ? 'job' : 'jobs'}`
                : 'No running jobs.'}
            </Text>
            <ThumbnailJobAction status={status} onRefresh={onRefresh} />
            {entries(active)}
            {activeCopies.map((job) => (
              <CopyJobProgress key={job.id} job={job} onRefresh={onRefresh} />
            ))}
            {activeCopies.some((job) => job.action === 'copy_files') && <CopyIoControls />}
            {copyJobs
              .filter((job) => !copyJobActive(job))
              .map((job) => (
                <Box key={job.id}>
                  <CopyJobProgress job={job} onRefresh={onRefresh} />
                  {job.action === 'prepare_names' && !job.readyReviewId && (
                    <ReopenPreparedCopy job={job} />
                  )}
                  {(job.action === 'copy_files' || job.readyReviewId) && (
                    <RetainedCopyInspection job={job} onRefresh={onRefresh} />
                  )}
                </Box>
              ))}
            {inspections.map((job) => (
              <InspectionJobProgress key={job.id} job={job} onRefresh={onRefresh} />
            ))}
            <LibraryChanges status={status} />
            {originalJobs.map((job) => (
              <Box key={job.id}>
                <OriginalJobProgress job={job} />
                <OriginalHistoryReview job={job} />
              </Box>
            ))}
            {recent.length > 0 && (
              <details className="recent-jobs">
                <summary>Recent jobs ({recent.length})</summary>
                {entries(recent)}
              </details>
            )}
          </>
        )}
      </Box>
    </chakra.section>
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
    <chakra.section
      data-job-kind={job.kind ?? 'scan'}
      className="job-row"
      aria-label={`${job.kind === 'thumbnails' ? 'Thumbnail' : 'Scan'} job for ${label}`}
    >
      <Heading as="h3" unstyled>
        <bdi>{label}</bdi>
      </Heading>
      <ScanJobProgress job={job} label={label} />
      <JobCounts job={job} />
      <Text className="help">{job.message}</Text>
      <Box className="job-actions">
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
      </Box>
      {cancelError && (
        <Text className="error" role="alert">
          {cancelError}
        </Text>
      )}
      {expanded && <JobDetails id={detailId} detail={detail} error={detailError} />}
    </chakra.section>
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
    <Box className="thumbnail-job-action">
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Preview library</span>
          </Field.Label>
          <NativeSelect
            value={selected.id}
            onChange={(event) => setRootId(event.target.value)}
            disabled={pending}
          >
            {status.roots.map((root) => (
              <option value={root.id} key={root.id}>
                {root.label}
              </option>
            ))}
          </NativeSelect>
        </label>
      </Field.Root>
      <Button onClick={() => void generate()} disabled={pending}>
        {pending ? 'Starting…' : 'Generate thumbnails'}
      </Button>
      <Text className="help">
        Prepare tiny previews in the background. Originals stay read-only.
      </Text>
      {error && (
        <Text role="alert" className="error">
          {error}
        </Text>
      )}
    </Box>
  );
}

function JobCounts({ job }: { job: ScanJob }) {
  return (
    <Text className="job-counts">
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
    </Text>
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
    <Box id={id} className="job-details">
      {error ? (
        <Text className="error" role="alert">
          {error}
        </Text>
      ) : !detail ? (
        <Text role="status">Loading job details…</Text>
      ) : detail.issues.length ? (
        <>
          <Text className="help">
            Skip reasons and failures (up to 200 entries). Retry scans the saved scope again,
            reusing unchanged metadata. Change scan settings to include intentionally excluded
            files. Cached thumbnails are reused.
          </Text>
          <List.Root unstyled>
            {Array.from(new Set(detail.issues)).map((issue) => (
              <List.Item unstyled key={issue}>
                <bdi>{issue}</bdi>
              </List.Item>
            ))}
          </List.Root>
        </>
      ) : (
        <Text>No file issues recorded.</Text>
      )}
    </Box>
  );
}
