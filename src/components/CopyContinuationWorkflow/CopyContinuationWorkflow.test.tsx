import { fireEvent, render, screen, waitFor } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type { CopyContinuationReview, CopyWorkflowJob, LibraryRequest } from '../../bindings';
import { library, libraryData } from '../../ipc';
import { CopyContinuationWorkflow } from '@/components/CopyContinuationWorkflow';
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
  library: vi.fn(),
}));
const job: CopyWorkflowJob = {
  id: 'prior',
  planId: 'plan',
  reviewId: 'stream',
  sourceRootId: 'root',
  action: 'copy_files',
  state: 'needs_recovery',
  totalFiles: 1,
  verifiedFiles: 0,
  readyReviewId: null,
  message: 'Interrupted',
  error: null,
  startedAt: 'now',
  updatedAt: 'now',
};
const selection = {
  id: 'native-selection',
  displayPath: 'Chosen destination',
  identityAvailable: true,
};
const review: CopyContinuationReview = {
  id: 'review',
  priorJobId: 'prior',
  planId: 'plan',
  manifestId: 'manifest',
  digest: 'review-digest',
  sourceRootId: 'root',
  destinationDisplayPath: 'Chosen destination',
  outputFolder: 'Stillharbour organized retained',
  files: [
    {
      file: {
        id: 'file',
        sourcePath: 'before/photo.JPG',
        targetPath: 'Stillharbour organized retained/after/photo.JPG',
        bytes: '42',
      },
      recordedPhase: 'stage_recorded',
      retainedStageBytes: '17',
      outputPresent: false,
    },
  ],
  assessment: {
    planId: 'plan',
    manifestId: 'manifest',
    manifestDigest: 'manifest-digest',
    observedAt: 'now',
    capacity: {
      policy: 'copy-continuation-estimate-v1',
      status: 'sufficient_for_estimate',
      allocationUnitBytes: '4096',
      logicalBytes: '42',
      roundedDataBytes: '4096',
      metadataAllowanceBytes: '131072',
      journalAllowanceBytes: '33554432',
      headroomBytes: '67108864',
      estimatedPeakAdditionalBytes: '100798464',
      observedAvailableBytes: '1000000000',
      observedTotalBytes: '2000000000',
      outputDirectories: 2,
      spaceReserved: false,
    },
    fidelity: [],
    sourcesRetained: true,
    independentBackupVerified: false,
    executionAvailable: false,
  },
  recordedComplete: false,
  sourcesRetained: true,
  independentBackup: false,
};
const continued: CopyWorkflowJob = {
  ...job,
  id: 'continued',
  state: 'running',
  message: 'Fresh comparisons pending.',
};
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(library).mockResolvedValue({ kind: 'done' });
});
it('reviews recorded state, requires acknowledgment and separate approval/start, then polls fresh progress', async () => {
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    switch (request.kind) {
      case 'organization_copy_continuation_review':
        return review;
      case 'organization_copy_continuation_approve':
        return { id: 'ticket', reviewId: 'review', action: 'copy_files', expiresInSeconds: 600 };
      case 'organization_copy_continuation_start':
        return continued;
      case 'organization_copy_job':
        return { ...continued, state: 'completed_verified', verifiedFiles: 1 };
      case 'organization_copy_io':
        return { aggregateBytesPerSecond: '10485760', byteIoPaused: false, burstBytes: 131072 };
      default:
        throw new Error(`Unexpected request ${request.kind}`);
    }
  });
  render(<CopyContinuationWorkflow job={job} selection={selection} />);
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Review verification and continuation' }));
  await screen.findByRole('heading', { name: 'Continuation review' });
  expect(screen.getByText('before/photo.JPG')).toBeVisible();
  expect(screen.getByText('Stage recorded')).toBeVisible();
  expect(
    screen.getByRole('button', { name: 'Approve verification and continuation' }),
  ).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Approve verification and continuation' }));
  await screen.findByRole('button', { name: 'Start reviewed continuation' });
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([request]) => request.kind === 'organization_copy_continuation_start'),
  ).toBe(false);
  fireEvent.click(screen.getByRole('button', { name: 'Start reviewed continuation' }));
  await screen.findByRole('heading', { name: 'File bytes copied and verified' });
  expect(screen.getByText('1 of 1 files byte-verified.')).toBeVisible();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'organization_copy_continuation_start',
      selection_id: 'native-selection',
      input: { reviewId: 'review', digest: 'review-digest', approvalId: 'ticket' },
    },
    'organization_copy_job',
  );
  expect(
    screen.queryByRole('button', { name: 'Start reviewed continuation' }),
  ).not.toBeInTheDocument();
});
it('rejects foreign reviews before offering approval', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce({ ...review, priorJobId: 'other' });
  render(<CopyContinuationWorkflow job={job} selection={selection} />);
  fireEvent.click(screen.getByRole('button', { name: 'Review verification and continuation' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('does not match');
  expect(
    screen.queryByRole('button', { name: 'Approve verification and continuation' }),
  ).not.toBeInTheDocument();
});
it('discards a valid review that arrives after unmount without exposing approval', async () => {
  let finish!: (value: CopyContinuationReview) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const view = render(<CopyContinuationWorkflow job={job} selection={selection} />);
  fireEvent.click(screen.getByRole('button', { name: 'Review verification and continuation' }));
  view.unmount();
  finish(review);
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({
      kind: 'organization_copy_continuation_discard',
      review_id: 'review',
    }),
  );
  expect(
    screen.queryByRole('button', { name: 'Approve verification and continuation' }),
  ).not.toBeInTheDocument();
});
