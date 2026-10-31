import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { CopyActionReview, CopyManifest, CopyWorkflowJob, LibraryRequest } from '../bindings';
import { libraryData, pickDirectory } from '../ipc';
import { VerifiedCopyWorkflow } from './VerifiedCopyWorkflow';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
  pickDirectory: vi.fn(),
}));
const manifest: CopyManifest = {
  id: 'manifest',
  planId: 'plan',
  digest: 'manifest-digest',
  draftDigest: 'draft',
  createdAt: 'now',
  bundleConfirmationIds: [],
  files: [],
  issues: [],
  selectedBytes: '42',
  executionAvailable: false,
};
const preparation: CopyActionReview = {
  id: 'preparation',
  planId: 'plan',
  manifestId: 'manifest',
  digest: 'preparation-digest',
  action: 'prepare_names',
  sourceRootId: 'root',
  destinationDisplayPath: 'Chosen destination',
  outputFolder: 'Stillharbour organized fixture',
  files: [
    {
      id: 'photo',
      sourcePath: 'source/photo.JPG',
      targetPath: 'Stillharbour organized fixture/after/photo.JPG',
      bytes: '42',
    },
  ],
  assessment: {
    planId: 'plan',
    manifestId: 'manifest',
    manifestDigest: 'manifest-digest',
    observedAt: 'now',
    capacity: {
      policy: 'estimate',
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
      outputDirectories: 1,
      spaceReserved: false,
    },
    fidelity: [],
    sourcesRetained: true,
    independentBackupVerified: false,
    executionAvailable: false,
  },
  sourcesRetained: true,
  independentBackup: false,
};
const copy: CopyActionReview = {
  ...preparation,
  id: 'copy',
  digest: 'copy-digest',
  action: 'copy_files',
};
function job(action: 'prepare_names' | 'copy_files', finished: boolean): CopyWorkflowJob {
  return {
    id: action === 'prepare_names' ? 'prepare-job' : 'copy-job',
    planId: 'plan',
    reviewId: action === 'prepare_names' ? 'preparation' : 'copy',
    sourceRootId: 'root',
    action,
    state: finished ? (action === 'prepare_names' ? 'prepared' : 'completed_verified') : 'queued',
    totalFiles: 1,
    verifiedFiles: finished && action === 'copy_files' ? 1 : 0,
    readyReviewId: finished && action === 'prepare_names' ? 'copy' : null,
    message: finished ? 'Native operation finished' : 'Native operation queued',
    error: null,
    startedAt: 'now',
    updatedAt: 'now',
  };
}
beforeEach(() => {
  vi.mocked(libraryData).mockReset();
  vi.mocked(pickDirectory).mockReset().mockResolvedValue({
    id: 'native-selection',
    displayPath: 'Chosen destination',
    identityAvailable: true,
  });
});
it('keeps unsupported builds disabled without opening a picker or starting filesystem work', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce({
    available: false,
    reason: 'Native workflow gates are pending.',
  });
  render(<VerifiedCopyWorkflow manifest={manifest} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and review preparation' }),
  );
  await screen.findByText('Native workflow gates are pending.');
  expect(pickDirectory).not.toHaveBeenCalled();
  expect(libraryData).toHaveBeenCalledTimes(1);
});
it('requires separate exact approvals and starts, then loads the prepared review and actual inspection', async () => {
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    switch (request.kind) {
      case 'organization_copy_capabilities':
        return { available: true, reason: 'Sources retained' };
      case 'organization_copy_review_create':
        return preparation;
      case 'organization_copy_review':
        return copy;
      case 'organization_copy_approve':
        return {
          id: `approval-${request.input.action}`,
          reviewId: request.input.reviewId,
          action: request.input.action,
          expiresInSeconds: 600,
        };
      case 'organization_copy_start':
        return job(
          request.input.reviewId === 'preparation' ? 'prepare_names' : 'copy_files',
          false,
        );
      case 'organization_copy_job':
        return job(request.job_id === 'prepare-job' ? 'prepare_names' : 'copy_files', true);
      case 'organization_copy_io':
        return { aggregateBytesPerSecond: '10485760', byteIoPaused: false, burstBytes: 131072 };
      case 'organization_copy_inspect':
        return {
          reviewId: 'copy',
          states: ['published_verified'],
          allPublishedBytesVerified: true,
        };
      default:
        throw new Error(`Unexpected request: ${request.kind}`);
    }
  });
  render(<VerifiedCopyWorkflow manifest={manifest} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and review preparation' }),
  );
  await screen.findByRole('heading', { name: 'Output preparation review' });
  expect(screen.getByText('Stillharbour organized fixture/after/photo.JPG')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Approve output preparation' }));
  await screen.findByRole('button', { name: 'Start output preparation' });
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([request]) => request.kind === 'organization_copy_start'),
  ).toBe(false);
  fireEvent.click(screen.getByRole('button', { name: 'Start output preparation' }));
  await screen.findByRole('heading', { name: 'Prepared copy review' });
  expect(screen.getByRole('button', { name: 'Approve byte-verified copy' })).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Approve byte-verified copy' }));
  await screen.findByRole('button', { name: 'Start verified copy' });
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.filter(([request]) => request.kind === 'organization_copy_start'),
  ).toHaveLength(1);
  fireEvent.click(screen.getByRole('button', { name: 'Start verified copy' }));
  await screen.findByRole('heading', { name: 'File bytes copied and verified' });
  fireEvent.click(screen.getByRole('button', { name: 'Inspect actual output bytes' }));
  await screen.findByText('Every selected output was freshly compared with its source.');
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'organization_copy_start',
      selection_id: 'native-selection',
      input: { reviewId: 'copy', digest: 'copy-digest', approvalId: 'approval-copy_files' },
    },
    'organization_copy_job',
  );
});
it('rejects a foreign review before exposing any approval or start control', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ available: true, reason: 'test' })
    .mockResolvedValueOnce({ ...preparation, planId: 'another-plan' });
  render(<VerifiedCopyWorkflow manifest={manifest} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and review preparation' }),
  );
  await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('does not match'));
  expect(
    screen.queryByRole('button', { name: 'Approve output preparation' }),
  ).not.toBeInTheDocument();
});
