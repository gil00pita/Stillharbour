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
    <chakra.section
      className="jobs-page"
      css={{
        '& h2': { fontSize: '17px', letterSpacing: '0', margin: '0' },
        width: '100%',
        minWidth: '0',
        padding: '0',
        '@media (max-width: 760px)': { padding: '0' },
        '& h3': { margin: '0', fontSize: '13px', fontWeight: '550', overflowWrap: 'anywhere' },
        '& p': { overflowWrap: 'anywhere' },
        '& li': { overflowWrap: 'anywhere' },
        '& > .help': { margin: '8px 0 0' },
      }}
      aria-labelledby={id}
    >
      <PageHeader>
        <chakra.header
          className="toolbar jobs-heading"
          css={{
            flexShrink: '0',
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'space-between',
            gap: '16px',
            minHeight: '64px',
            padding: '12px 24px',
            borderBottom: '1px solid {colors.border}',
            background: '{colors.bg.toolbar}',
            '@media (max-width: 600px)': { minHeight: '64px', padding: '18px 24px' },
            marginBottom: '24px',
            '& h1': { margin: '0', fontSize: '24px' },
            margin: '0',
          }}
        >
          <Box>
            <Heading as="h1" unstyled id={id} ref={heading} tabIndex={-1}>
              Jobs
            </Heading>
          </Box>
          <Box
            className="header-actions"
            css={{ display: 'flex', alignItems: 'center', gap: '8px' }}
          >
            <Button type="button" aria-label="Back from Jobs" onClick={onBack}>
              Back
            </Button>
          </Box>
        </chakra.header>
      </PageHeader>
      <Box
        className="jobs-content"
        css={{ padding: '24px', '@media (max-width: 760px)': { paddingInline: '16px' } }}
      >
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Follow background work and review recent outcomes.
        </Text>
        {!status ? (
          <Text role="status">Connecting to jobs…</Text>
        ) : (
          <>
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
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
              <chakra.details
                className="recent-jobs"
                css={{
                  marginTop: '20px',
                  '& > summary': {
                    cursor: 'pointer',
                    color: '{colors.fg.muted}',
                    fontSize: '12px',
                  },
                }}
              >
                <summary>Recent jobs ({recent.length})</summary>
                {entries(recent)}
              </chakra.details>
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
      css={{
        padding: '20px 0',
        borderBottom: '1px solid {colors.border}',
        '&:last-child': { borderBottom: '0', paddingBottom: '0' },
        '& p': { fontSize: '13px', lineHeight: '20px' },
      }}
      aria-label={`${job.kind === 'thumbnails' ? 'Thumbnail' : 'Scan'} job for ${label}`}
    >
      <Heading as="h3" unstyled>
        <bdi>{label}</bdi>
      </Heading>
      <ScanJobProgress job={job} label={label} />
      <JobCounts job={job} />
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        {job.message}
      </Text>
      <Box
        className="job-actions"
        css={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '12px' }}
      >
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
        <Text
          className="error"
          css={{
            border: '1px solid {colors.border.control}',
            borderRadius: 'var(--control-radius)',
            background: '{colors.danger.bg}',
            color: '{colors.danger.fg}',
            padding: '16px 20px',
            margin: '24px 0 0',
            fontSize: '13px',
            lineHeight: '1.6',
            '& p': { margin: '0' },
            '& button': {
              marginTop: '12px',
              border: '1px solid currentColor',
              background: 'transparent',
              color: 'inherit',
              borderRadius: '4px',
              padding: '7px 12px',
            },
          }}
          role="alert"
        >
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
    <Box
      className="thumbnail-job-action"
      css={{
        display: 'grid',
        gap: '8px',
        marginBlock: '16px',
        '& label': { display: 'grid', gap: '6px' },
        '& select': { width: '100%', minWidth: '0' },
        '& .help': { margin: '0' },
      }}
    >
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
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        Prepare tiny previews in the background. Originals stay read-only.
      </Text>
      {error && (
        <Text
          role="alert"
          className="error"
          css={{
            border: '1px solid {colors.border.control}',
            borderRadius: 'var(--control-radius)',
            background: '{colors.danger.bg}',
            color: '{colors.danger.fg}',
            padding: '16px 20px',
            margin: '24px 0 0',
            fontSize: '13px',
            lineHeight: '1.6',
            '& p': { margin: '0' },
            '& button': {
              marginTop: '12px',
              border: '1px solid currentColor',
              background: 'transparent',
              color: 'inherit',
              borderRadius: '4px',
              padding: '7px 12px',
            },
          }}
        >
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
    <Box
      id={id}
      className="job-details"
      css={{ fontSize: '13px', lineHeight: '20px', '& li': { overflowWrap: 'anywhere' } }}
    >
      {error ? (
        <Text
          className="error"
          css={{
            border: '1px solid {colors.border.control}',
            borderRadius: 'var(--control-radius)',
            background: '{colors.danger.bg}',
            color: '{colors.danger.fg}',
            padding: '16px 20px',
            margin: '24px 0 0',
            fontSize: '13px',
            lineHeight: '1.6',
            '& p': { margin: '0' },
            '& button': {
              marginTop: '12px',
              border: '1px solid currentColor',
              background: 'transparent',
              color: 'inherit',
              borderRadius: '4px',
              padding: '7px 12px',
            },
          }}
          role="alert"
        >
          {error}
        </Text>
      ) : !detail ? (
        <Text role="status">Loading job details…</Text>
      ) : detail.issues.length ? (
        <>
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
          >
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
