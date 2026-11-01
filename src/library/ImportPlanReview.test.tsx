import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type {
  CopyManifest,
  ImportComparisonJob,
  ImportPlan,
  ImportSession,
  ImportSkipApproval,
  LibraryRequest,
} from '../bindings';
import { library, libraryData } from '../ipc';
import { ImportPlanReview } from './ImportPlanReview';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  library: vi.fn(),
  libraryData: vi.fn(),
}));
const hash = 'a'.repeat(64),
  group = 'b'.repeat(64);
const manifest: CopyManifest = {
  id: 'manifest',
  planId: 'owner',
  draftDigest: hash,
  digest: hash,
  createdAt: 'now',
  bundleConfirmationIds: ['confirmed'],
  files: [
    {
      id: 'photo',
      rootId: 'source',
      sourceName: 'Photo.JPG',
      sourceRevision: hash,
      targetRelativePath: 'new/Photo.JPG',
      bytes: '2',
      bundleAssetIds: ['photo-asset'],
    },
    {
      id: 'sidecar',
      rootId: 'source',
      sourceName: 'Photo.xmp',
      sourceRevision: hash,
      targetRelativePath: 'new/Photo.xmp',
      bytes: '4',
      bundleAssetIds: ['photo-asset'],
    },
  ],
  issues: [],
  selectedBytes: '6',
  executionAvailable: false,
};
const comparison: ImportComparisonJob = {
  id: 'comparison',
  planId: 'owner',
  manifestId: 'manifest',
  sourceRootId: 'source',
  scopeRootIds: ['existing'],
  state: 'completed',
  stage: 'finished',
  candidateFiles: 2,
  processedCandidates: 2,
  comparedSourceFiles: 2,
  totalSourceFiles: 2,
  bytesRead: '36',
  error: null,
  report: {
    policy: 'indexed-import-comparison-v1',
    samplePolicy: 'import-three-window-blake3-v1',
    algorithm: 'blake3',
    observedAt: 'now',
    indexedPresentFilesOnly: true,
    allScopeAvailable: true,
    excludedOrMissingFiles: 0,
    bundleReviewRequired: true,
    mutationAuthorized: false,
    files: manifest.files.map((file, index) => ({
      fileId: file.id,
      sourcePath: file.sourceName,
      bytes: file.bytes,
      state: 'exact_bytes_compared',
      matches: [
        {
          assetId: `keeper-${index}`,
          rootId: 'existing',
          relativePath: index === 0 ? 'Renamed.JPG' : 'Renamed.xmp',
          displayTruncated: false,
          digest: hash,
          sameNativeObject: false,
        },
      ],
      furtherCandidatesUncompared: false,
    })),
    bundles: [
      {
        bundleAssetIds: ['photo-asset'],
        fileIds: ['photo', 'sidecar'],
        disposition: 'review_existing_bundle',
        existingGroups: [
          {
            id: group,
            rootId: 'existing',
            members: [
              { fileId: 'photo', assetId: 'keeper-0' },
              { fileId: 'sidecar', assetId: 'keeper-1' },
            ],
          },
        ],
        reason: 'Complete coherent renamed group.',
      },
    ],
    issues: [],
  },
};
function saved(): ImportPlan {
  return {
    id: 'import-plan',
    planId: 'owner',
    manifestId: 'manifest',
    manifestDigest: hash,
    digest: hash,
    createdAt: 'now',
    sourceRootId: 'source',
    scopeRootIds: ['existing'],
    choices: [{ fileId: 'photo', existingGroupId: group }],
    copyFileIds: [],
    skipFiles: [
      {
        fileId: 'photo',
        keeperAssetId: 'keeper-0',
        keeperRootId: 'existing',
        keeperRelativePath: 'Renamed.JPG',
        digest: hash,
        sameNativeObject: false,
      },
      {
        fileId: 'sidecar',
        keeperAssetId: 'keeper-1',
        keeperRootId: 'existing',
        keeperRelativePath: 'Renamed.xmp',
        digest: hash,
        sameNativeObject: false,
      },
    ],
    sourcesRetained: true,
    executionAvailable: true,
  };
}
function session(state: ImportSession['state'] = 'queued'): ImportSession {
  return {
    id: 'session',
    planId: 'owner',
    importPlanId: 'import-plan',
    digest: hash,
    state,
    totalFiles: 2,
    verifiedFiles: state === 'completed' ? 2 : 0,
    bytesRead: state === 'completed' ? '12' : '0',
    sourcesRetained: true,
    outputCreated: false,
    createdAt: 'now',
    finishedAt: state === 'completed' ? 'later' : null,
    error: null,
  };
}
beforeEach(() => {
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request: LibraryRequest) => {
      switch (request.kind) {
        case 'organization_copy_capabilities':
          return { available: true, reason: '' };
        case 'organization_import_plan_create':
        case 'organization_import_plan_open':
          return saved();
        case 'organization_import_skip_approve':
          return {
            id: 'approval',
            importPlanId: 'import-plan',
            digest: hash,
            expiresInSeconds: 600,
          };
        case 'organization_import_skip_start':
          return session();
        case 'organization_import_session':
          return session('completed');
        case 'organization_import_session_cancel':
          return { ...session(), state: 'cancel_requested' };
        case 'organization_import_history':
          return {
            plans: [
              {
                id: 'import-plan',
                manifestId: 'manifest',
                digest: hash,
                createdAt: 'now',
                copyFiles: 0,
                skipFiles: 2,
              },
            ],
            sessions: [session('needs_recovery')],
          };
        default:
          throw new Error(`Unexpected request ${request.kind}`);
      }
    });
});
async function save() {
  fireEvent.change(screen.getByRole('combobox'), { target: { value: group } });
  fireEvent.click(screen.getByRole('button', { name: 'Save import decisions' }));
  await screen.findByText(/Saved import plan: 0 files to copy/);
}
async function approve() {
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Approve existing-bundle import' }));
  await screen.findByRole('button', { name: 'Verify and record skipped import' });
}
it('requires explicit bundle choice, separate approval and start before recording actual verification', async () => {
  const view = render(<ImportPlanReview manifest={manifest} comparison={comparison} />);
  expect(screen.getByRole('button', { name: 'Save import decisions' })).toBeDisabled();
  await save();
  expect(screen.getByRole('button', { name: 'Approve existing-bundle import' })).toBeDisabled();
  await approve();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([request]) => request.kind === 'organization_import_skip_start'),
  ).toBe(false);
  fireEvent.click(screen.getByRole('button', { name: 'Verify and record skipped import' }));
  await screen.findByText(/Import recorded: 2 existing files freshly compared/);
  expect(screen.getByText(/no output folder created/)).toBeInTheDocument();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'organization_import_plan_create',
      input: {
        comparisonJobId: 'comparison',
        choices: [{ fileId: 'photo', existingGroupId: group }],
      },
    },
    'organization_import_plan',
  );
  view.unmount();
  expect(library).toHaveBeenCalledWith({
    kind: 'organization_import_session_discard',
    plan_id: 'owner',
    session_id: 'session',
  });
  expect(library).toHaveBeenCalledWith({
    kind: 'organization_import_plan_release',
    plan_id: 'owner',
    import_plan_id: 'import-plan',
  });
});
it('rejects a saved response which splits a confirmed companion bundle', async () => {
  const normal = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) => {
    if (request.kind === 'organization_import_plan_create') {
      const invalid = saved();
      invalid.skipFiles.pop();
      invalid.copyFileIds = ['sidecar'];
      invalid.executionAvailable = false;
      return invalid;
    }
    return normal(request, ...rest);
  });
  render(<ImportPlanReview manifest={manifest} comparison={comparison} />);
  fireEvent.change(screen.getByRole('combobox'), { target: { value: group } });
  fireEvent.click(screen.getByRole('button', { name: 'Save import decisions' }));
  await screen.findByRole('alert');
  expect(
    screen.queryByRole('button', { name: 'Approve existing-bundle import' }),
  ).not.toBeInTheDocument();
});
it('shows stale keeper errors without claiming an import or issuing approval', async () => {
  const normal = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) => {
    if (request.kind === 'organization_import_plan_create')
      throw new Error('A keeper changed after comparison.');
    return normal(request, ...rest);
  });
  render(<ImportPlanReview manifest={manifest} comparison={comparison} />);
  fireEvent.change(screen.getByRole('combobox'), { target: { value: group } });
  fireEvent.click(screen.getByRole('button', { name: 'Save import decisions' }));
  await screen.findByText('A keeper changed after comparison.');
  expect(screen.queryByText(/Import recorded:/)).not.toBeInTheDocument();
});
it('opens durable incomplete history without replaying or restoring an approval', async () => {
  render(<ImportPlanReview manifest={manifest} />);
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Review saved import decisions' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Review saved import decisions' }));
  fireEvent.click(
    await screen.findByRole('button', { name: /Review import attempt: needs recovery/ }),
  );
  await screen.findByText(/This retained attempt has no complete outcome/);
  expect(screen.getByRole('button', { name: 'Approve existing-bundle import' })).toBeDisabled();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(
        ([request]) =>
          request.kind === 'organization_import_skip_start' ||
          request.kind === 'organization_import_skip_approve',
      ),
  ).toBe(false);
});
it('rejects a foreign session without showing its success', async () => {
  const normal = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) =>
    request.kind === 'organization_import_skip_start'
      ? { ...session('completed'), importPlanId: 'foreign' }
      : normal(request, ...rest),
  );
  render(<ImportPlanReview manifest={manifest} comparison={comparison} />);
  await save();
  await approve();
  fireEvent.click(screen.getByRole('button', { name: 'Verify and record skipped import' }));
  await screen.findByRole('alert');
  expect(screen.queryByText(/Import recorded:/)).not.toBeInTheDocument();
});
it('revokes a valid approval arriving after its view closes', async () => {
  const normal = vi.mocked(libraryData).getMockImplementation()!;
  let resolve!: (value: ImportSkipApproval) => void;
  const pending = new Promise<ImportSkipApproval>((done) => {
    resolve = done;
  });
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) =>
    request.kind === 'organization_import_skip_approve' ? pending : normal(request, ...rest),
  );
  const view = render(<ImportPlanReview manifest={manifest} comparison={comparison} />);
  await save();
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Approve existing-bundle import' }));
  view.unmount();
  vi.mocked(library).mockClear();
  await act(async () =>
    resolve({
      id: 'late-approval',
      importPlanId: 'import-plan',
      digest: hash,
      expiresInSeconds: 600,
    }),
  );
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({
      kind: 'organization_import_plan_release',
      plan_id: 'owner',
      import_plan_id: 'import-plan',
    }),
  );
});
it('cancels a valid start response arriving after its view closes', async () => {
  const normal = vi.mocked(libraryData).getMockImplementation()!;
  let resolve!: (value: ImportSession) => void;
  const pending = new Promise<ImportSession>((done) => {
    resolve = done;
  });
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) =>
    request.kind === 'organization_import_skip_start' ? pending : normal(request, ...rest),
  );
  const view = render(<ImportPlanReview manifest={manifest} comparison={comparison} />);
  await save();
  await approve();
  fireEvent.click(screen.getByRole('button', { name: 'Verify and record skipped import' }));
  await waitFor(() =>
    expect(
      vi
        .mocked(libraryData)
        .mock.calls.some(([request]) => request.kind === 'organization_import_skip_start'),
    ).toBe(true),
  );
  view.unmount();
  await act(async () => resolve(session()));
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({
      kind: 'organization_import_session_discard',
      plan_id: 'owner',
      session_id: 'session',
    }),
  );
});
it('releases an unused approval when source-retention acknowledgment is cleared', async () => {
  render(<ImportPlanReview manifest={manifest} comparison={comparison} />);
  await save();
  await approve();
  fireEvent.click(screen.getByRole('checkbox'));
  expect(
    screen.queryByRole('button', { name: 'Verify and record skipped import' }),
  ).not.toBeInTheDocument();
  expect(library).toHaveBeenCalledWith({
    kind: 'organization_import_plan_release',
    plan_id: 'owner',
    import_plan_id: 'import-plan',
  });
});
it('disables unavailable production import history with the backend reason', async () => {
  vi.mocked(libraryData).mockResolvedValue({
    available: false,
    reason: 'Experimental import workflow is disabled.',
  });
  render(<ImportPlanReview manifest={manifest} />);
  await screen.findByText('Experimental import workflow is disabled.');
  expect(screen.getByRole('button', { name: 'Review saved import decisions' })).toBeDisabled();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.every(([request]) => request.kind === 'organization_copy_capabilities'),
  ).toBe(true);
});
it('filters other source plans and attempts before validating the current root history', async () => {
  const normal = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) => {
    if (request.kind === 'organization_import_history')
      return {
        plans: [
          {
            id: 'current-plan',
            manifestId: manifest.id,
            digest: hash,
            createdAt: 'current-time',
            copyFiles: 0,
            skipFiles: manifest.files.length,
            sourceRootId: manifest.files[0]!.rootId,
          },
          {
            id: 'other-plan',
            manifestId: manifest.id,
            digest: hash,
            createdAt: 'other-time',
            copyFiles: 1,
            skipFiles: 0,
            sourceRootId: 'another-source',
          },
        ],
        sessions: [{ ...session(), id: 'another-session', importPlanId: 'other-plan' }],
      };
    return normal(request, ...rest);
  });
  render(<ImportPlanReview manifest={manifest} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Review saved import decisions' }));
  await screen.findByRole('button', { name: /Open import plan:.*current-time/ });
  expect(screen.queryByRole('button', { name: /other-time/ })).not.toBeInTheDocument();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});
