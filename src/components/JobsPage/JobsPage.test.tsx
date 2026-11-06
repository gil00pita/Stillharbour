import { fireEvent, render, screen, waitFor, within } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type { LibraryStatus, ScanJob } from '../../bindings';
import { library, libraryData } from '../../ipc';
import { JobsPage } from '@/components/JobsPage';
import { defaultFilter } from '../../library/model';

vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  library: vi.fn(),
  libraryData: vi.fn(),
}));
const job: ScanJob = {
  id: 'fixture-job',
  rootId: 'fixture-root',
  state: 'running',
  discovered: 40,
  indexed: 12,
  skipped: 1,
  errors: 2,
  message: 'Reading originals',
  startedAt: '2026-10-28',
};
const status: LibraryStatus = {
  roots: [],
  jobs: [job],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
const refresh = vi.fn<() => Promise<void>>();
beforeEach(() => {
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  vi.mocked(libraryData).mockReset();
  refresh.mockReset().mockResolvedValue();
});
function open() {
  return screen.getByRole('region', { name: 'Jobs' });
}
it('includes durable generated organization jobs without exposing a replay control', () => {
  render(
    <JobsPage
      status={{
        ...status,
        jobs: [],
        originalJobs: [
          {
            id: 'organization',
            planId: 'plan',
            sourceRootId: 'root',
            kind: 'undo',
            state: 'needs_recovery',
            totalMoves: 10,
            verifiedMoves: 0,
            startedAt: 'now',
            updatedAt: 'now',
            message: 'Inspect current files; approval was not restored.',
            generatedFixtureOnly: true,
          },
        ],
      }}
      onRefresh={refresh}
      onBack={vi.fn()}
    />,
  );
  const panel = open();
  expect(within(panel).getByText('Undo organization · Generated fixture')).toBeInTheDocument();
  expect(within(panel).getByText('Needs recovery review')).toBeInTheDocument();
  expect(
    within(panel).queryByRole('button', { name: /resume|retry|start/i }),
  ).not.toBeInTheDocument();
  expect(library).not.toHaveBeenCalled();
});

it('lists every active job with truthful counts and stopping state; keeps history secondary', () => {
  render(
    <JobsPage
      status={{
        ...status,
        jobs: [
          job,
          { ...job, id: 'stopping', state: 'cancel_requested' },
          { ...job, id: 'done', state: 'completed' },
        ],
      }}
      onRefresh={refresh}
      onBack={vi.fn()}
    />,
  );
  expect(screen.getByText('2 running jobs')).toBeVisible();
  const panel = open();
  expect(
    within(panel)
      .getAllByRole('progressbar')
      .filter((bar) => !bar.hasAttribute('aria-valuenow')),
  ).toHaveLength(2);
  expect(within(panel).getByText('Stopping scan…')).toBeVisible();
  expect(within(panel).getByRole('button', { name: 'Stopping…' })).toBeDisabled();
  expect(within(panel).getAllByText('40 found · 12 indexed · 1 skipped · 2 issues')).toHaveLength(
    3,
  );
  expect(within(panel).getByText('Scan complete')).not.toBeVisible();
  fireEvent.click(within(panel).getByText('Recent jobs (1)'));
  expect(within(panel).getByText('Scan complete')).toBeVisible();
});
it('distinguishes connecting from no jobs, and moves completed work to history without navigation', () => {
  const { rerender } = render(<JobsPage status={null} onRefresh={refresh} onBack={vi.fn()} />);
  const panel = open();
  expect(within(panel).getByText('Connecting to jobs…')).toBeVisible();
  rerender(<JobsPage status={status} onRefresh={refresh} onBack={vi.fn()} />);
  expect(within(panel).getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
  rerender(
    <JobsPage
      status={{ ...status, jobs: [{ ...job, state: 'completed' }] }}
      onRefresh={refresh}
      onBack={vi.fn()}
    />,
  );
  expect(within(panel).getByText('No running jobs.')).toBeVisible();
  expect(screen.getByRole('heading', { name: 'Jobs' })).toBeVisible();
});
it('focuses its heading and stays open on Escape or outside interaction until Back is chosen', () => {
  const onBack = vi.fn();
  render(
    <>
      <JobsPage status={status} onRefresh={refresh} onBack={onBack} />
      <button>Other navigation</button>
    </>,
  );
  expect(screen.getByRole('heading', { name: 'Jobs' })).toHaveFocus();
  fireEvent.keyDown(document, { key: 'Escape' });
  fireEvent.pointerDown(screen.getByRole('button', { name: 'Other navigation' }));
  expect(screen.getByRole('region', { name: 'Jobs' })).toBeVisible();
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(onBack).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Back from Jobs' }));
  expect(onBack).toHaveBeenCalledOnce();
});
it('cancels by validated job identity, refreshes status and exposes native failures without fake success', async () => {
  render(<JobsPage status={status} onRefresh={refresh} onBack={vi.fn()} />);
  open();
  vi.mocked(library).mockRejectedValueOnce(new Error('Scan could not be stopped. Try again.'));
  fireEvent.click(screen.getByRole('button', { name: 'Cancel scan' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Scan could not be stopped');
  expect(refresh).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel scan' }));
  await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
  expect(library).toHaveBeenLastCalledWith({ kind: 'cancel', job_id: job.id });
});
it('loads issue details lazily, renders them as text and handles failure', async () => {
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Details unavailable.'));
  render(<JobsPage status={status} onRefresh={refresh} onBack={vi.fn()} />);
  open();
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Details' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Details unavailable');
  fireEvent.click(screen.getByRole('button', { name: 'Hide details' }));
  vi.mocked(libraryData).mockResolvedValueOnce({
    job,
    issues: ['<script>untrusted</script>', '<script>untrusted</script>'],
  });
  fireEvent.click(screen.getByRole('button', { name: 'Details' }));
  expect(await screen.findByText('<script>untrusted</script>')).toBeVisible();
  expect(screen.getAllByRole('listitem')).toHaveLength(1);
  expect(libraryData).toHaveBeenCalledWith({ kind: 'job_details', job_id: job.id }, 'job_details');
});

it('starts a separate thumbnail job for a selected saved library and exposes thumbnail cancellation', async () => {
  render(
    <JobsPage
      status={{
        ...status,
        roots: [
          {
            id: 'fixture-root',
            label: 'Generated library',
            displayPath: '/generated',
            mode: 'referenced',
            isBase: true,
            availability: 'online',
            filter: defaultFilter,
            lastScan: null,
            fileCount: 40,
          },
        ],
        jobs: [{ ...job, kind: 'thumbnails', state: 'queued' }],
      }}
      onRefresh={refresh}
      onBack={vi.fn()}
    />,
  );
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Generate thumbnails' }));
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({ kind: 'generate_thumbnails', root_id: 'fixture-root' }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Cancel thumbnails' }));
  await waitFor(() => expect(library).toHaveBeenCalledWith({ kind: 'cancel', job_id: job.id }));
});

it('shows and cancels a durable inspection outside its results screen', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_cancel')
      return {
        id: 'inspection',
        input: { rootIds: ['root'], scope: 'all_eligible_files', task: { kind: 'storage' } },
        state: 'cancel_requested',
        startedAt: '2026-11-01',
        updatedAt: '2026-11-01',
        finishedAt: null,
        filesObserved: 5,
        filesChecked: 2,
        bytesRead: '100',
        issues: 0,
        limited: false,
        message: 'Stopping inspection',
      };
    if (request.kind === 'changes') return { changes: [], nextBefore: null };
    throw new Error('Unexpected request');
  });
  render(
    <JobsPage
      status={{
        ...status,
        jobs: [],
        inspectionJobs: [
          {
            id: 'inspection',
            input: { rootIds: ['root'], scope: 'all_eligible_files', task: { kind: 'storage' } },
            state: 'running',
            startedAt: '2026-11-01',
            updatedAt: '2026-11-01',
            finishedAt: null,
            filesObserved: 5,
            filesChecked: 2,
            bytesRead: '100',
            issues: 0,
            limited: false,
            message: 'Inspecting generated files',
          },
        ],
      }}
      onRefresh={refresh}
      onBack={vi.fn()}
    />,
  );
  const panel = open();
  expect(within(panel).getByText('Inspecting generated files')).toBeVisible();
  fireEvent.click(within(panel).getByRole('button', { name: 'Cancel inspection' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'inspection_cancel', job_id: 'inspection' },
      'inspection_job',
    ),
  );
  await waitFor(() => expect(refresh).toHaveBeenCalled());
});

it('pauses and resumes the same job and retries terminal work by native identity', async () => {
  const { rerender } = render(<JobsPage status={status} onRefresh={refresh} onBack={vi.fn()} />);
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
  await waitFor(() => expect(library).toHaveBeenCalledWith({ kind: 'pause', job_id: job.id }));
  await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
  rerender(
    <JobsPage
      status={{ ...status, jobs: [{ ...job, state: 'paused' }] }}
      onRefresh={refresh}
      onBack={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Resume' }));
  await waitFor(() => expect(library).toHaveBeenCalledWith({ kind: 'resume', job_id: job.id }));
  await waitFor(() => expect(refresh).toHaveBeenCalledTimes(2));
  rerender(
    <JobsPage
      status={{ ...status, jobs: [{ ...job, state: 'partial' }] }}
      onRefresh={refresh}
      onBack={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByText('Recent jobs (1)'));
  fireEvent.click(screen.getByRole('button', { name: 'Retry scan' }));
  await waitFor(() => expect(library).toHaveBeenCalledWith({ kind: 'retry', job_id: job.id }));
});
