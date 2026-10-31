import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { BundleConfirmation, CopyManifest, OrganizationPlanDetail } from '../bindings';
import { libraryData } from '../ipc';
import { CopyManifestReview } from './CopyManifestReview';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const detail: OrganizationPlanDetail = {
  stale: false,
  plan: {
    id: 'plan',
    policy: 'organization-plan-v1',
    createdAt: 'now',
    digest: 'draft',
    state: 'draft',
    templateReference: null,
    destinationPath: 'Destination',
    destinationFingerprint: 'destination',
    blockers: ['execution_and_recovery'],
    executionAvailable: false,
    preview: {
      policy: 'preview',
      fingerprint: 'preview',
      catalogRevision: '1',
      template: { pattern: '{original_stem}.{ext}', event: '', missingDate: 'block' },
      rows: [
        {
          assetId: 'photo',
          rootId: 'root',
          sourcePath: 'photo.JPG',
          sourceRevision: 'revision',
          targetRelativePath: 'after/photo.JPG',
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
    },
  },
};
const choice: BundleConfirmation = {
  id: 'decision',
  planId: 'plan',
  assetId: 'photo',
  inventoryFingerprint: 'inventory',
  fingerprint: 'confirmation',
  files: [],
  totalBytes: '42',
  executionAvailable: false,
};
const manifest: CopyManifest = {
  id: 'manifest',
  planId: 'plan',
  draftDigest: 'draft',
  digest: 'digest',
  createdAt: 'now',
  bundleConfirmationIds: ['decision'],
  files: [
    {
      id: 'file',
      rootId: 'root',
      sourceName: '<photo>.JPG',
      sourceRevision: 'revision',
      targetRelativePath: 'after/photo.JPG',
      bytes: '9007199254740993',
      bundleAssetIds: ['photo'],
    },
  ],
  issues: [],
  selectedBytes: '9007199254740993',
  executionAvailable: false,
};
beforeEach(() => vi.mocked(libraryData).mockReset());

it('requires explicit decisions and sends only retained IDs, preserving exact bytes and literal filenames', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce(manifest);
  const view = render(<CopyManifestReview detail={detail} choices={{}} />);
  expect(libraryData).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Compile copy review' })).toBeDisabled();
  view.rerender(<CopyManifestReview detail={detail} choices={{ photo: choice }} />);
  fireEvent.click(screen.getByRole('button', { name: 'Compile copy review' }));
  expect(await screen.findByRole('heading', { name: 'Compiled file manifest' })).toHaveFocus();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'organization_copy_manifest',
      input: { planId: 'plan', bundleConfirmationIds: ['decision'] },
    },
    'organization_copy_manifest',
  );
  expect(screen.getAllByText(/9,007,199,254,740,993/).length).toBe(2);
  expect(screen.getByText('<photo>.JPG')).toBeInTheDocument();
  expect(document.querySelector('photo')).toBeNull();
  view.rerender(
    <CopyManifestReview detail={detail} choices={{ photo: { ...choice, id: 'different' } }} />,
  );
  expect(screen.getByRole('alert')).toHaveTextContent('scope changed');
});

it('keeps conflicts blocked and pages all proposed outputs', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce({
    ...manifest,
    files: Array.from({ length: 65 }, (_, index) => ({
      ...manifest.files[0]!,
      id: `file-${index}`,
      sourceName: `photo-${index}.JPG`,
    })),
    issues: [{ kind: 'source_has_multiple_targets', fileIds: ['a', 'b'] }],
  } satisfies CopyManifest);
  render(<CopyManifestReview detail={detail} choices={{ photo: choice }} />);
  fireEvent.click(screen.getByRole('button', { name: 'Compile copy review' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('conflicts block');
  expect(screen.getByText(/different proposed destinations across bundles/)).toBeInTheDocument();
  expect(screen.getByRole('table')).toHaveAccessibleName('Compiled files, page 1 of 2');
  expect(screen.queryByText('photo-64.JPG')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Next compiled files' }));
  expect(screen.getByText('photo-64.JPG')).toBeInTheDocument();
});

it('reports errors without clearing choices, rejects the wrong draft and ignores late results after unmount', async () => {
  vi.mocked(libraryData)
    .mockRejectedValueOnce(new Error('Source changed'))
    .mockResolvedValueOnce({ ...manifest, draftDigest: 'different' });
  const view = render(<CopyManifestReview detail={detail} choices={{ photo: choice }} />);
  fireEvent.click(screen.getByRole('button', { name: 'Compile copy review' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Source changed');
  fireEvent.click(screen.getByRole('button', { name: 'Compile copy review' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('different draft');
  let resolve!: (value: CopyManifest) => void;
  vi.mocked(libraryData).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Compile copy review' }));
  view.unmount();
  await act(async () => resolve(manifest));
  expect(screen.queryByRole('table')).not.toBeInTheDocument();
});
