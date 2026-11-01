import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { CopyAssessment, CopyManifest } from '../bindings';
import { libraryData } from '../ipc';
import { CopyAssessmentReview } from './CopyAssessmentReview';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const manifest: CopyManifest = {
  id: 'manifest',
  planId: 'plan',
  digest: 'digest',
  draftDigest: 'draft',
  createdAt: 'now',
  bundleConfirmationIds: [],
  files: [],
  issues: [],
  selectedBytes: '9007199254740993',
  executionAvailable: false,
};
const report: CopyAssessment = {
  planId: 'plan',
  manifestId: 'manifest',
  manifestDigest: 'digest',
  observedAt: '2026-10-31T16:00:00Z',
  capacity: {
    policy: 'retained-source-copy-estimate-v1',
    status: 'sufficient_for_estimate',
    allocationUnitBytes: '4096',
    logicalBytes: '9007199254740993',
    roundedDataBytes: '9007199254745088',
    metadataAllowanceBytes: '131072',
    journalAllowanceBytes: '33554432',
    headroomBytes: '900719925474509',
    estimatedPeakAdditionalBytes: '9907919213905101',
    observedAvailableBytes: '10000000000000000',
    observedTotalBytes: '11000000000000000',
    outputDirectories: 1,
    spaceReserved: false,
  },
  fidelity: [
    { aspect: 'file_bytes', status: 'verification_required' },
    { aspect: 'alternate_data_streams', status: 'not_preserved_or_verified' },
  ],
  sourcesRetained: true,
  independentBackupVerified: false,
  executionAvailable: false,
};
beforeEach(() => vi.mocked(libraryData).mockReset());
it('checks only on request with retained IDs and reports exact totals and explicit fidelity limits', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce(report);
  render(<CopyAssessmentReview manifest={manifest} />);
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Check copy space and fidelity' }));
  await waitFor(() =>
    expect(screen.getByRole('heading', { name: 'Copy space estimate and fidelity' })).toHaveFocus(),
  );
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'organization_copy_assessment', plan_id: 'plan', manifest_id: 'manifest' },
    'organization_copy_assessment',
  );
  expect(
    screen.getByText(`${BigInt(report.capacity.logicalBytes).toLocaleString()} bytes`),
  ).toBeVisible();
  expect(screen.getByText('Not preserved or verified')).toBeVisible();
  expect(
    screen.getByText(/does not verify any copied content or independent backup/),
  ).toBeVisible();
  expect(screen.getByText(/Space is not reserved/)).toBeVisible();
});
it('rejects a foreign digest, supports retry and shows insufficient space without success authority', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ ...report, manifestDigest: 'foreign' })
    .mockResolvedValueOnce({
      ...report,
      capacity: { ...report.capacity, status: 'insufficient_for_estimate' },
    });
  render(<CopyAssessmentReview manifest={manifest} />);
  fireEvent.click(screen.getByRole('button', { name: 'Check copy space and fidelity' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('different review');
  expect(screen.queryByRole('heading')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Check copy space and fidelity' }));
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Insufficient observed space blocks copying.',
  );
  expect(screen.getByText(/does not approve copying/)).toBeVisible();
});
it('keeps errors retryable, disables duplicate requests and ignores an unmounted result', async () => {
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Destination offline'));
  const view = render(<CopyAssessmentReview manifest={manifest} />);
  fireEvent.click(screen.getByRole('button', { name: 'Check copy space and fidelity' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Destination offline');
  let resolve!: (value: CopyAssessment) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise<CopyAssessment>((done) => {
        resolve = done;
      }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Check copy space and fidelity' }));
  expect(screen.getByRole('button', { name: 'Checking copy space…' })).toBeDisabled();
  view.unmount();
  await act(async () => resolve(report));
  expect(screen.queryByRole('heading')).not.toBeInTheDocument();
});
