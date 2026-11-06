import { fireEvent, render, screen, waitFor } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type { CopyManifest, ImportPlan, LibraryRequest } from '../../bindings';
import { library, libraryData } from '../../ipc';
import { SavedImportPlans } from '@/components/SavedImportPlans';
import { defaultFilter } from '../../library/model';
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  library: vi.fn(),
  libraryData: vi.fn(),
}));
const hash = 'a'.repeat(64);
const manifest: CopyManifest = {
  id: 'manifest',
  planId: 'owner',
  draftDigest: hash,
  digest: hash,
  createdAt: 'before',
  bundleConfirmationIds: ['confirmation'],
  files: [
    {
      id: 'file',
      rootId: 'source',
      sourceName: 'Photo.JPG',
      sourceRevision: hash,
      targetRelativePath: 'new/Photo.JPG',
      bytes: '2',
      bundleAssetIds: ['asset'],
    },
  ],
  issues: [],
  selectedBytes: '2',
  executionAvailable: false,
};
const plan: ImportPlan = {
  id: 'import',
  planId: 'owner',
  manifestId: 'manifest',
  manifestDigest: hash,
  digest: hash,
  createdAt: 'before',
  sourceRootId: 'source',
  scopeRootIds: ['existing'],
  choices: [{ fileId: 'file', existingGroupId: 'b'.repeat(64) }],
  copyFileIds: [],
  skipFiles: [
    {
      fileId: 'file',
      keeperAssetId: 'keeper',
      keeperRootId: 'existing',
      keeperRelativePath: 'Renamed.JPG',
      digest: hash,
      sameNativeObject: false,
    },
  ],
  sourcesRetained: true,
  executionAvailable: true,
};
beforeEach(() => {
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request: LibraryRequest) => {
      switch (request.kind) {
        case 'organization_copy_capabilities':
          return { available: true, reason: '' };
        case 'organization_import_saved_plans':
          return [
            {
              id: 'import',
              manifestId: 'manifest',
              digest: hash,
              createdAt: 'before',
              copyFiles: 0,
              skipFiles: 1,
            },
          ];
        case 'organization_import_plan_open':
          return plan;
        case 'organization_copy_manifest_open':
          return manifest;
        default:
          throw new Error(`Unexpected history action ${request.kind}`);
      }
    });
});
async function list() {
  fireEvent.click(screen.getByRole('button', { name: 'Check retained import history' }));
  return screen.findByRole('button', { name: /Review retained import: 0 copy \/ 1 existing/ });
}
it('opens retained import evidence without a new companion decision or restored approval', async () => {
  render(<SavedImportPlans planId="owner" />);
  fireEvent.click(await list());
  await screen.findByText(/Saved import plan: 0 files to copy/);
  await waitFor(() => expect(screen.getByRole('checkbox')).toBeEnabled());
  expect(screen.getByRole('checkbox')).not.toBeChecked();
  expect(screen.getByRole('button', { name: 'Approve existing-bundle import' })).toBeDisabled();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.every(([request]) =>
        [
          'organization_copy_capabilities',
          'organization_import_saved_plans',
          'organization_import_plan_open',
          'organization_copy_manifest_open',
        ].includes(request.kind),
      ),
  ).toBe(true);
});
it('rejects a foreign retained plan before requesting its source manifest', async () => {
  const normal = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) =>
    request.kind === 'organization_import_plan_open'
      ? { ...plan, planId: 'foreign' }
      : normal(request, ...rest),
  );
  render(<SavedImportPlans planId="owner" />);
  fireEvent.click(await list());
  await screen.findByRole('alert');
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([request]) => request.kind === 'organization_copy_manifest_open'),
  ).toBe(false);
});
it('opens only the retained source partition and displays its library label from the full manifest', async () => {
  const normal = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) => {
    if (request.kind === 'organization_copy_manifest_open')
      return {
        ...manifest,
        files: [
          ...manifest.files,
          {
            ...manifest.files[0]!,
            id: 'other-file',
            rootId: 'other-source',
            bundleAssetIds: ['other-asset'],
          },
        ],
        selectedBytes: '4',
      };
    if (request.kind === 'status')
      return {
        roots: [
          {
            id: 'source',
            label: 'Original source library',
            displayPath: 'Generated source',
            mode: 'referenced',
            isBase: false,
            availability: 'online',
            filter: defaultFilter,
            lastScan: null,
            fileCount: 1,
          },
        ],
        jobs: [],
        revision: '1',
        previewGeneration: '0',
        metadataAvailable: false,
        previewAvailable: false,
      };
    return normal(request, ...rest);
  });
  render(<SavedImportPlans planId="owner" />);
  fireEvent.click(await list());
  await screen.findByText('Original source library');
  expect(screen.getByText(/Saved import plan: 0 files to copy/)).toHaveTextContent(
    '1 existing files',
  );
  expect(
    screen.queryByRole('button', { name: 'Start existing-bundle verification' }),
  ).not.toBeInTheDocument();
});
it('shows the unavailable-build reason without requesting import records', async () => {
  vi.mocked(libraryData).mockResolvedValue({
    available: false,
    reason: 'Import review is disabled in this build.',
  });
  render(<SavedImportPlans planId="owner" />);
  fireEvent.click(screen.getByRole('button', { name: 'Check retained import history' }));
  await screen.findByText('Import review is disabled in this build.');
  expect(screen.getByRole('button', { name: 'Check retained import history' })).toBeDisabled();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.every(([request]) => request.kind === 'organization_copy_capabilities'),
  ).toBe(true);
});
