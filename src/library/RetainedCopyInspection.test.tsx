import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { CopyRecoveryInspection, CopyWorkflowJob } from '../bindings';
import { libraryData, pickDirectory } from '../ipc';
import { RetainedCopyInspection } from './RetainedCopyInspection';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
  pickDirectory: vi.fn(),
}));
const job: CopyWorkflowJob = {
  id: 'job',
  planId: 'plan',
  reviewId: 'copy',
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
const result: CopyRecoveryInspection = {
  jobId: 'job',
  sourceRootId: 'root',
  destinationDisplayPath: 'Destination',
  outputFolder: 'Stillharbour organized fixture',
  files: [
    {
      id: 'file',
      sourcePath: 'source/photo.JPG',
      targetPath: 'Stillharbour organized fixture/after/photo.JPG',
      bytes: '42',
    },
  ],
  inspection: { reviewId: 'copy', states: ['needs_attention'], allPublishedBytesVerified: false },
  issues: [
    {
      fileId: 'file',
      error: {
        code: 'CONFLICT',
        message: 'The copied file length differs from the reviewed source.',
      },
    },
  ],
};
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(pickDirectory).mockResolvedValue({
    id: 'selection',
    displayPath: 'Destination',
    identityAvailable: true,
  });
});
it('requires a fresh native selection and reports retained issues without approving or starting work', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ available: true, reason: 'Available' })
    .mockResolvedValueOnce(result);
  render(<RetainedCopyInspection job={job} />);
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Choose original destination and inspect' }));
  await screen.findByText(/The copied file length differs/);
  expect(screen.getByText('source/photo.JPG')).toBeInTheDocument();
  expect(screen.getByText('Stillharbour organized fixture/after/photo.JPG')).toBeInTheDocument();
  expect(libraryData).toHaveBeenLastCalledWith(
    { kind: 'organization_copy_recovery_inspect', job_id: 'job', selection_id: 'selection' },
    'organization_copy_recovery_inspection',
  );
  expect(libraryData).toHaveBeenCalledTimes(2);
});
it('keeps an unavailable build out of the native picker', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce({
    available: false,
    reason: 'Native recovery gate remains open.',
  });
  render(<RetainedCopyInspection job={job} />);
  fireEvent.click(screen.getByRole('button', { name: 'Choose original destination and inspect' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Native recovery gate remains open.');
  expect(pickDirectory).not.toHaveBeenCalled();
});
it('rejects another job and ignores an inspection arriving after unmount', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ available: true, reason: 'Available' })
    .mockResolvedValueOnce({ ...result, jobId: 'other' });
  const view = render(<RetainedCopyInspection job={job} />);
  fireEvent.click(screen.getByRole('button', { name: 'Choose original destination and inspect' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('does not match this job');
  let complete!: (value: CopyRecoveryInspection) => void;
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ available: true, reason: 'Available' })
    .mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          complete = resolve;
        }),
    );
  fireEvent.click(screen.getByRole('button', { name: 'Choose original destination and inspect' }));
  await waitFor(() => expect(complete).toBeTypeOf('function'));
  view.unmount();
  complete(result);
  expect(screen.queryByText('source/photo.JPG')).not.toBeInTheDocument();
});
