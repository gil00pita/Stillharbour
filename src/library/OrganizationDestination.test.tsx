import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { OrganizationDestinationReview, OrganizationPreview } from '../bindings';
import { libraryData, pickDirectory } from '../ipc';
import { OrganizationDestination } from './OrganizationDestination';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
  pickDirectory: vi.fn(),
}));
const preview: OrganizationPreview = {
  policy: 'organization-preview-v1',
  fingerprint: 'preview',
  catalogRevision: '1',
  template: { pattern: '{original_stem}.{ext}', event: '', missingDate: 'undated' },
  rows: [
    {
      assetId: 'photo',
      rootId: 'root',
      sourcePath: 'photo.JPG',
      sourceRevision: 'revision',
      targetRelativePath: 'photo.JPG',
      sequence: 1,
      bytes: '42',
      issues: [],
      usedUndated: false,
      companionCandidates: 0,
    },
  ],
  proposed: 1,
  blocked: 0,
  selectedBytes: '42',
  executionAvailable: false,
};
const response: OrganizationDestinationReview = {
  previewFingerprint: 'preview',
  catalogRevision: '1',
  destinationPath: 'Fixture/<destination>',
  fingerprint: 'destination',
  executionAvailable: false,
  persisted: false,
};
beforeEach(() => {
  vi.mocked(pickDirectory).mockReset().mockResolvedValue({
    id: 'opaque-session',
    displayPath: 'Selected destination',
    identityAvailable: true,
  });
  vi.mocked(libraryData).mockReset().mockResolvedValue(response);
});
const choose = () => fireEvent.click(screen.getByRole('button', { name: 'Choose destination' }));
it('saves a native-bound draft only after preflight and prevents duplicate submission', async () => {
  render(<OrganizationDestination preview={preview} revision="1" />);
  expect(screen.queryByRole('button', { name: 'Save plan draft' })).not.toBeInTheDocument();
  choose();
  await screen.findByRole('status');
  vi.mocked(libraryData).mockResolvedValueOnce({
    plan: {
      id: 'saved',
      policy: 'organization-plan-v1',
      createdAt: 'now',
      digest: 'digest',
      state: 'draft',
      templateReference: null,
      preview,
      destinationPath: response.destinationPath,
      destinationFingerprint: response.fingerprint,
      blockers: ['execution_and_recovery'],
      executionAvailable: false,
    },
    stale: false,
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save plan draft' }));
  expect(await screen.findByText(/Plan draft saved/)).toBeVisible();
  expect(libraryData).toHaveBeenLastCalledWith(
    {
      kind: 'organization_plan_create',
      selection_id: 'opaque-session',
      draft: {
        assetIds: ['photo'],
        template: preview.template,
        previewFingerprint: 'preview',
        templateReference: null,
      },
    },
    'organization_plan',
  );
  expect(screen.getByRole('button', { name: 'Save plan draft' })).toBeDisabled();
});
it('checks only an explicit native selection and discloses unfinished planning', async () => {
  render(<OrganizationDestination preview={preview} revision="1" />);
  expect(pickDirectory).not.toHaveBeenCalled();
  choose();
  expect(await screen.findByRole('status')).toHaveTextContent(
    'identity and indexed-root overlap checked',
  );
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'organization_destination',
      selection_id: 'opaque-session',
      asset_ids: ['photo'],
      template: preview.template,
      preview_fingerprint: 'preview',
    },
    'organization_destination',
  );
  expect(screen.getByText('Fixture/<destination>')).toBeVisible();
  expect(document.querySelector('destination')).toBeNull();
  expect(screen.getByText(/without reserving target names/)).toBeVisible();
});
it('retains the previous observation on picker cancellation without another check', async () => {
  render(<OrganizationDestination preview={preview} revision="1" />);
  choose();
  await screen.findByRole('status');
  vi.mocked(pickDirectory).mockResolvedValueOnce(null);
  choose();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Choose destination' })).toBeEnabled(),
  );
  expect(libraryData).toHaveBeenCalledOnce();
  expect(screen.getByText('Fixture/<destination>')).toBeVisible();
});
it('refuses unsupported selections and allows retry after a backend conflict', async () => {
  vi.mocked(pickDirectory).mockResolvedValueOnce({
    id: 'unsupported',
    displayPath: 'Unavailable',
    identityAvailable: false,
  });
  render(<OrganizationDestination preview={preview} revision="1" />);
  choose();
  expect(await screen.findByRole('alert')).toHaveTextContent('native identity cannot be verified');
  expect(libraryData).not.toHaveBeenCalled();
  vi.mocked(libraryData).mockRejectedValueOnce({ message: 'Destination overlaps the source.' });
  choose();
  expect(await screen.findByRole('alert')).toHaveTextContent('Destination overlaps');
  fireEvent.click(screen.getByRole('button', { name: 'Check destination again' }));
  await screen.findByRole('status');
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});
it('marks a review stale when the catalog changes while the request is pending', async () => {
  let resolve!: (review: OrganizationDestinationReview) => void;
  vi.mocked(libraryData).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const { rerender } = render(<OrganizationDestination preview={preview} revision="1" />);
  choose();
  await waitFor(() => expect(libraryData).toHaveBeenCalledOnce());
  rerender(<OrganizationDestination preview={preview} revision="2" />);
  resolve(response);
  expect(await screen.findByRole('alert')).toHaveTextContent('Build a fresh preview');
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Choose destination' })).toBeDisabled();
});
it('discards a late picker result after the preview closes', async () => {
  let resolve!: (selection: Awaited<ReturnType<typeof pickDirectory>>) => void;
  vi.mocked(pickDirectory).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const { unmount } = render(<OrganizationDestination preview={preview} revision="1" />);
  choose();
  unmount();
  resolve({ id: 'late', displayPath: 'Late selection', identityAvailable: true });
  await Promise.resolve();
  expect(libraryData).not.toHaveBeenCalled();
});

it('accepts a fresh preview newer than the last polled status without weakening later invalidation', async () => {
  const fresh = { ...preview, catalogRevision: '2' };
  vi.mocked(libraryData).mockResolvedValueOnce({ ...response, catalogRevision: '2' });
  const { rerender } = render(<OrganizationDestination preview={fresh} revision="1" />);
  choose();
  expect(await screen.findByRole('status')).toHaveTextContent(
    'identity and indexed-root overlap checked',
  );
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  rerender(<OrganizationDestination preview={fresh} revision="3" />);
  expect(screen.getByRole('alert')).toHaveTextContent('Build a fresh preview');
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
});
