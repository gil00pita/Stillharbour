import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type {
  ComparisonEntry,
  ComparisonRun,
  SyncCapabilities,
  SyncJob,
  SyncReply,
  SyncRequest,
  SyncReview,
} from '../bindings';
import { libraryData } from '../ipc';
import { SyncWorkflow } from './SyncWorkflow';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const capabilities: SyncCapabilities = {
  executionAvailable: true,
  retainedReplacementAvailable: false,
  reason: '',
  ioScope: 'Aggregate application reads and writes',
  bytesPerSecond: '10485760',
};
const run = {
  id: 'run',
  sourceRootId: 'source',
  mode: 'verified',
  state: 'needs_review',
  planDigest: 'a'.repeat(64),
} as ComparisonRun;
const entry: ComparisonEntry = {
  id: 'difference',
  displayPath: '<script>.jpg',
  category: 'different_content',
  proposal: 'review_difference',
  sourceKind: 'file',
  destinationKind: 'file',
  sourceBytes: '4',
  destinationBytes: '4',
  sourceModified: null,
  destinationModified: null,
  evidence: 'Different bytes.',
  companions: [],
};
function review(action: SyncReview['action'] = 'prepare_names'): SyncReview {
  return {
    id: action === 'prepare_names' ? 'prepare-review' : 'copy-review',
    jobId: action === 'prepare_names' ? null : 'preparation-job',
    runId: 'run',
    action,
    digest: action === 'prepare_names' ? 'a'.repeat(64) : 'b'.repeat(64),
    sourceRootId: 'source',
    sourceDisplayPath: '/generated/source',
    datasetDisplayPath: '/generated/destination/dataset',
    copyFiles: 1,
    copyBytes: '4',
    retainedDestinationFiles: 1,
    uncopiedSourceRevisions: 1,
    availableBytes: '10000',
    requiredBytes: '8',
    metadataWarning: 'Essential streams and permissions are not proven; source retained.',
    executionAvailable: true,
    reason: '',
    approvalLifetimeSeconds: 600,
    items: [
      { entryId: 'new', displayPath: 'new.mp4', action: 'copy_new', sourceBytes: '4' },
      {
        entryId: 'difference',
        displayPath: '<script>.jpg',
        action: 'keep_destination',
        sourceBytes: '4',
      },
    ],
  };
}
function job(action: SyncJob['action'] = 'prepare_names'): SyncJob {
  return {
    id: action === 'prepare_names' ? 'preparation-job' : 'copy-job',
    runId: 'run',
    sourceRootId: 'source',
    action,
    state: action === 'prepare_names' ? 'needs_review' : 'completed_partial',
    phase: action === 'prepare_names' ? 'prepared' : 'completed',
    createdAt: '2026-11-02T00:00:00Z',
    updatedAt: '2026-11-02T00:00:01Z',
    sourceDisplayPath: '/generated/source',
    datasetDisplayPath: '/generated/destination/dataset',
    totalFiles: 1,
    verifiedFiles: action === 'prepare_names' ? 0 : 1,
    copyBytes: '4',
    uncopiedSourceRevisions: 1,
    retainedDestinationFiles: 1,
    readyReviewId: action === 'prepare_names' ? 'copy-review' : null,
    bytesPerSecond: '10485760',
    message:
      action === 'prepare_names'
        ? 'Prepared; separate approval required.'
        : 'Copied the new file; one source revision remains uncopied.',
  };
}
let handler: (request: SyncRequest) => Promise<SyncReply>;
beforeEach(() => {
  handler = async (request) => {
    if (request.kind === 'capabilities') return { kind: 'capabilities', data: capabilities };
    if (request.kind === 'list') return { kind: 'jobs', data: [] };
    if (request.kind === 'review_append') return { kind: 'review', data: review() };
    if (request.kind === 'review') return { kind: 'review', data: review('copy_files') };
    if (request.kind === 'approve')
      return {
        kind: 'approval',
        data: {
          id: `approval-${request.review_id}`,
          reviewId: request.review_id,
          action: request.review_id === 'prepare-review' ? 'prepare_names' : 'copy_files',
        },
      };
    if (request.kind === 'start')
      return {
        kind: 'job',
        data: job(
          request.approval_id === 'approval-prepare-review' ? 'prepare_names' : 'copy_files',
        ),
      };
    throw new Error('Unexpected request');
  };
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'backup_snapshots' && request.request.kind === 'profiles')
        return { kind: 'profiles', data: [] };
      if (request.kind !== 'backup_sync') throw new Error('Unexpected IPC family');
      return handler(request.request);
    });
});
function open(selected: ComparisonEntry | null = entry) {
  return render(
    <SyncWorkflow key="run" run={run} selected={selected} onActivityChange={vi.fn()} />,
  );
}
async function plan() {
  fireEvent.click(screen.getByRole('button', { name: 'Review append plan' }));
  return screen.findByRole('region', { name: 'Safe-copy action review' });
}
function acknowledge(panel: HTMLElement) {
  fireEvent.click(within(panel).getByRole('checkbox', { name: /I reviewed the source/ }));
  fireEvent.click(
    within(panel).getByRole('checkbox', { name: /I understand that 1 source revisions/ }),
  );
  fireEvent.click(within(panel).getByRole('checkbox', { name: /I reviewed the metadata limits/ }));
}

it('loads recorded retained versions and requires a fresh restore review before any start', async () => {
  const fallback = handler;
  handler = async (input) => {
    if (input.kind === 'list')
      return {
        kind: 'jobs',
        data: [{ ...job('copy_files'), id: 'replacement', action: 'replace_retaining' }],
      };
    if (input.kind === 'versions')
      return {
        kind: 'versions',
        data: [
          {
            id: 'old-version',
            jobId: 'replacement',
            displayPath: 'old/photo.jpg',
            bytes: '4',
            fullDigest: 'f'.repeat(64),
            retained: true,
            restoreAvailable: true,
            reason: 'Exact recorded version available.',
          },
        ],
      };
    if (input.kind === 'review_restore_retained')
      return {
        kind: 'review',
        data: {
          ...review('restore_retained'),
          jobId: 'replacement',
          items: [
            {
              entryId: 'old-version',
              displayPath: 'old/photo.jpg',
              action: 'restore_to_new_output',
              sourceBytes: '4',
            },
          ],
        },
      };
    return fallback(input);
  };
  open();
  fireEvent.click(await screen.findByRole('button', { name: 'Inspect retained versions' }));
  expect(
    await screen.findByText('Exact recorded version available.', { exact: false }),
  ).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Review retained restore to new output' }));
  const panel = await screen.findByRole('region', { name: 'Safe-copy action review' });
  expect(
    within(panel).getByRole('button', { name: 'Approve this safe-copy action' }),
  ).toBeDisabled();
  expect(screen.queryByRole('button', { name: /^Start restore/ })).not.toBeInTheDocument();
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'backup_sync', request: { kind: 'review_restore_retained', job_id: 'replacement' } },
    'backup_sync',
  );
});

it('requires exact keep decisions and separately approves preparation and copying', async () => {
  open();
  fireEvent.click(screen.getByRole('checkbox', { name: /Keep this destination version/ }));
  let panel = await plan();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_sync',
      request: { kind: 'review_append', run_id: 'run', keep_destination: ['difference'] },
    },
    'backup_sync',
  );
  expect(
    within(panel).getByRole('button', { name: 'Approve this safe-copy action' }),
  ).toBeDisabled();
  expect(document.querySelector('script')).toBeNull();
  acknowledge(panel);
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this safe-copy action' }));
  const prepare = await within(panel).findByRole('button', {
    name: 'Start prepare destination names',
  });
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([input]) => input.kind === 'backup_sync' && input.request.kind === 'start'),
  ).toBe(false);
  fireEvent.click(prepare);
  fireEvent.click(await screen.findByRole('button', { name: 'Review prepared copy' }));
  panel = await screen.findByRole('region', { name: 'Safe-copy action review' });
  expect(
    within(panel).getByRole('button', { name: 'Approve this safe-copy action' }),
  ).toBeDisabled();
  expect(screen.queryByRole('button', { name: 'Start copy new files' })).not.toBeInTheDocument();
  acknowledge(panel);
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this safe-copy action' }));
  fireEvent.click(await within(panel).findByRole('button', { name: 'Start copy new files' }));
  expect(await screen.findByText(/one source revision remains uncopied/)).toBeVisible();
  for (const warning of screen.getAllByText(/does not establish complete protection/))
    expect(warning).toBeVisible();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.filter(
        ([input]) => input.kind === 'backup_sync' && input.request.kind === 'start',
      ),
  ).toHaveLength(2);
});

it('shows backend capability and fidelity blocks without enabling approval', async () => {
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'review_append'
      ? {
          kind: 'review',
          data: {
            ...review(),
            executionAvailable: false,
            reason: 'Destination identity or recovery gate is unavailable.',
          },
        }
      : fallback(input);
  open();
  const panel = await plan();
  expect(within(panel).getByRole('status')).toHaveTextContent(
    'Destination identity or recovery gate is unavailable.',
  );
  expect(
    within(panel).getByRole('button', { name: 'Approve this safe-copy action' }),
  ).toBeDisabled();
  expect(
    within(panel).getByRole('checkbox', { name: /I reviewed the metadata limits/ }),
  ).toBeDisabled();
});

it('invalidates the displayed review and approval when a keep decision changes', async () => {
  open();
  const panel = await plan();
  acknowledge(panel);
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this safe-copy action' }));
  await within(panel).findByRole('button', { name: 'Start prepare destination names' });
  fireEvent.click(screen.getByRole('checkbox', { name: /Keep this destination version/ }));
  expect(screen.queryByRole('region', { name: 'Safe-copy action review' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /^Start / })).not.toBeInTheDocument();
});

it('does not reuse an approval after an uncertain start response', async () => {
  const fallback = handler;
  handler = async (input) => {
    if (input.kind === 'start') throw new Error('Connection lost.');
    return fallback(input);
  };
  open();
  const panel = await plan();
  acknowledge(panel);
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this safe-copy action' }));
  fireEvent.click(
    await within(panel).findByRole('button', { name: 'Start prepare destination names' }),
  );
  expect(await screen.findByRole('alert')).toHaveTextContent('Refresh safe-copy history');
  expect(screen.queryByRole('button', { name: /^Start / })).not.toBeInTheDocument();
  expect(screen.queryByRole('region', { name: 'Safe-copy action review' })).not.toBeInTheDocument();
});

it('drops a late approval after its comparison component has been retired', async () => {
  let finish!: (reply: SyncReply) => void;
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'approve'
      ? new Promise<SyncReply>((resolve) => {
          finish = resolve;
        })
      : fallback(input);
  const view = open();
  const panel = await plan();
  acknowledge(panel);
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this safe-copy action' }));
  await waitFor(() => expect(finish).toBeDefined());
  view.rerender(
    <SyncWorkflow
      key="new-comparison"
      run={{ ...run, id: 'other-run' }}
      selected={null}
      onActivityChange={vi.fn()}
    />,
  );
  finish({
    kind: 'approval',
    data: { id: 'late', reviewId: 'prepare-review', action: 'prepare_names' },
  });
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: /^Start / })).not.toBeInTheDocument(),
  );
});

it('paginates the immutable plan without narrowing its approval scope', async () => {
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'review_append'
      ? {
          kind: 'review',
          data: {
            ...review(),
            items: Array.from({ length: 205 }, (_, index) => ({
              entryId: `entry-${index}`,
              displayPath: `file-${index}.bin`,
              action: 'copy_new' as const,
              sourceBytes: '1',
            })),
            copyFiles: 205,
            copyBytes: '205',
          },
        }
      : fallback(input);
  open();
  const panel = await plan();
  expect(within(panel).getByText(/includes all 205 plan entries/)).toBeVisible();
  expect(within(panel).queryByText('file-204.bin')).not.toBeInTheDocument();
  fireEvent.click(within(panel).getByRole('button', { name: 'Next plan entries' }));
  expect(within(panel).getByText('file-204.bin')).toBeVisible();
  expect(within(panel).getByText(/Plan entries 201–205 of 205/)).toBeVisible();
});
