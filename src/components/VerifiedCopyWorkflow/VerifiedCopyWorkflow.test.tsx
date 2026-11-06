import { act, fireEvent, render, screen, waitFor } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type {
  CopyActionReview,
  CopyActionApproval,
  CopyManifest,
  CopyWorkflowJob,
  ImportPlan,
  LibraryRequest,
} from '../../bindings';
import { library, libraryData, pickDirectory } from '../../ipc';
import { VerifiedCopyWorkflow, ReopenPreparedCopy } from '@/components/VerifiedCopyWorkflow';
import { fixturePreferences } from '../../library/__tests__/fixtures';
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
  library: vi.fn(),
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
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  vi.mocked(pickDirectory).mockReset().mockResolvedValue({
    id: 'native-selection',
    displayPath: 'Chosen destination',
    identityAvailable: true,
  });
});
it.each([false, true])(
  'requires a source choice and rejects cross-root reviews (%s)',
  async (foreign) => {
    const files = ['root', 'other'].map((rootId, index) => ({
      id: index ? 'other-photo' : 'photo',
      rootId,
      sourceName: 'photo.JPG',
      sourceRevision: 'revision',
      targetRelativePath: 'after/photo.JPG',
      bytes: '42',
      bundleAssetIds: [],
    }));
    const roots = ['root', 'other'].map((id) => ({
      id,
      label: id === 'root' ? 'First library' : 'Second library',
      displayPath: `Generated ${id}`,
      mode: 'managed' as const,
      isBase: false,
      availability: 'online',
      filter: fixturePreferences().scanFilter,
      lastScan: null,
      fileCount: 1,
    }));
    vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
      switch (request.kind) {
        case 'status':
          return {
            roots,
            jobs: [],
            revision: '1',
            previewGeneration: '1',
            metadataAvailable: false,
            previewAvailable: false,
          };
        case 'organization_copy_capabilities':
          return { available: true, reason: 'Sources retained' };
        case 'organization_copy_effect_match':
          return {
            planId: 'plan',
            manifestId: 'manifest',
            digest: 'a'.repeat(64),
            existing: [],
            canReuseReview: true,
          };
        case 'organization_copy_review_create':
          return foreign ? { ...preparation, sourceRootId: 'other' } : preparation;
        default:
          throw new Error(`Unexpected request: ${request.kind}`);
      }
    });
    render(<VerifiedCopyWorkflow manifest={{ ...manifest, files }} />);
    const select = await screen.findByRole('combobox', { name: 'Source library to copy' });
    expect(select).toHaveValue('');
    expect(
      screen.queryByRole('button', { name: 'Choose destination and review preparation' }),
    ).not.toBeInTheDocument();
    expect(pickDirectory).not.toHaveBeenCalled();
    fireEvent.change(select, { target: { value: 'root' } });
    fireEvent.click(
      await screen.findByRole('button', { name: 'Choose destination and review preparation' }),
    );
    await waitFor(() =>
      expect(libraryData).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'organization_copy_review_create',
          source_root_id: 'root',
        }),
        'organization_copy_review',
      ),
    );
    expect(libraryData).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'organization_copy_effect_match', source_root_id: 'root' }),
      'organization_copy_effect_match',
    );
    if (foreign) {
      expect(await screen.findByRole('alert')).toHaveTextContent(
        'does not match this source library',
      );
      expect(screen.queryByRole('button', { name: /Approve/ })).not.toBeInTheDocument();
    } else {
      await screen.findByRole('heading', { name: 'Output preparation review' });
      fireEvent.change(select, { target: { value: 'other' } });
      expect(
        screen.queryByRole('heading', { name: 'Output preparation review' }),
      ).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /Approve/ })).not.toBeInTheDocument();
      expect(pickDirectory).toHaveBeenCalledOnce();
    }
  },
);
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
it('retries source discovery and refuses a missing-root choice without opening the picker', async () => {
  const files = ['root', 'other'].map((rootId) => ({
    id: rootId,
    rootId,
    sourceName: 'photo.JPG',
    sourceRevision: 'revision',
    targetRelativePath: 'after/photo.JPG',
    bytes: '42',
    bundleAssetIds: [],
  }));
  vi.mocked(libraryData)
    .mockRejectedValueOnce(new Error('Catalog temporarily unavailable'))
    .mockResolvedValueOnce({
      roots: [],
      jobs: [],
      revision: '1',
      previewGeneration: '1',
      metadataAvailable: false,
      previewAvailable: false,
    });
  render(<VerifiedCopyWorkflow manifest={{ ...manifest, files }} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog temporarily unavailable');
  fireEvent.click(screen.getByRole('button', { name: 'Retry source libraries' }));
  const select = await screen.findByRole('combobox', { name: 'Source library to copy' });
  expect(screen.getAllByRole('option', { name: /Unavailable source library/ })).toHaveLength(2);
  for (const option of screen.getAllByRole('option', { name: /Unavailable source library/ }))
    expect(option).toBeDisabled();
  fireEvent.change(select, { target: { value: 'root' } });
  expect(screen.getByRole('alert')).toHaveTextContent('unavailable or has no managed grant');
  expect(
    screen.queryByRole('button', { name: 'Choose destination and review preparation' }),
  ).not.toBeInTheDocument();
  expect(pickDirectory).not.toHaveBeenCalled();
});
it('requires separate exact approvals and starts, then loads the prepared review and actual inspection', async () => {
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    switch (request.kind) {
      case 'organization_copy_capabilities':
        return { available: true, reason: 'Sources retained' };
      case 'organization_copy_effect_match':
        return {
          planId: 'plan',
          manifestId: 'manifest',
          digest: 'a'.repeat(64),
          existing: [],
          canReuseReview: true,
        };
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

it('shows retained output history and freshly inspects it without allocating another folder', async () => {
  const priorJob = { ...job('copy_files', true), planId: 'original-plan' };
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    switch (request.kind) {
      case 'organization_copy_capabilities':
        return { available: true, reason: 'Sources retained' };
      case 'organization_copy_effect_match':
        return {
          planId: 'plan',
          manifestId: 'manifest',
          digest: 'a'.repeat(64),
          canReuseReview: false,
          existing: [
            {
              planId: 'original-plan',
              manifestId: 'original-manifest',
              reviewId: 'original-preparation',
              outputFolder: 'Stillharbour organized original',
              preparationRecorded: true,
              job: priorJob,
            },
          ],
        };
      case 'organization_copy_recovery_inspect':
        return {
          jobId: 'copy-job',
          sourceRootId: 'root',
          destinationDisplayPath: 'Chosen destination',
          outputFolder: 'Stillharbour organized original',
          files: copy.files.map((file) => ({
            ...file,
            targetPath: `Stillharbour organized original/after/photo.JPG`,
          })),
          issues: [],
          inspection: {
            reviewId: 'copy',
            states: ['published_verified'],
            allPublishedBytesVerified: true,
          },
        };
      default:
        throw new Error(`Unexpected request ${request.kind}`);
    }
  });
  render(<VerifiedCopyWorkflow manifest={manifest} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and review preparation' }),
  );
  await screen.findByRole('heading', { name: 'Existing output for this mapping' });
  expect(
    screen.queryByRole('button', { name: 'Approve output preparation' }),
  ).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Choose original destination and inspect' }));
  await screen.findByText('Every selected output was freshly compared with its source.');
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(
        ([request]) =>
          request.kind === 'organization_copy_review_create' ||
          request.kind === 'organization_copy_start',
      ),
  ).toBe(false);
});
it('rejects foreign effect history before requesting a copy review', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ available: true, reason: 'test' })
    .mockResolvedValueOnce({
      planId: 'foreign',
      manifestId: 'manifest',
      digest: 'a'.repeat(64),
      existing: [],
      canReuseReview: true,
    });
  render(<VerifiedCopyWorkflow manifest={manifest} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and review preparation' }),
  );
  await screen.findByRole('alert');
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([request]) => request.kind === 'organization_copy_review_create'),
  ).toBe(false);
});

const reopenedManifest: CopyManifest = {
  ...manifest,
  files: [
    {
      id: 'photo',
      rootId: 'root',
      sourceName: 'source/photo.JPG',
      sourceRevision: 'revision',
      targetRelativePath: 'after/photo.JPG',
      bytes: '42',
      bundleAssetIds: ['photo'],
    },
  ],
};
it('reopens stopped preparation as an unapproved copy review without replaying preparation', async () => {
  const stopped = {
    ...job('prepare_names', true),
    state: 'needs_attention' as const,
    readyReviewId: null,
  };
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    if (request.kind === 'organization_copy_capabilities')
      return { available: true, reason: 'test' };
    if (request.kind === 'organization_copy_preparation_reopen')
      return { jobId: stopped.id, manifest: reopenedManifest, review: copy };
    throw new Error(`Unexpected request ${request.kind}`);
  });
  render(<ReopenPreparedCopy job={stopped} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and reopen preparation' }),
  );
  await screen.findByRole('heading', { name: 'Prepared copy review' });
  expect(screen.getByRole('button', { name: 'Approve byte-verified copy' })).toBeDisabled();
  expect(screen.queryByRole('button', { name: 'Start verified copy' })).not.toBeInTheDocument();
  expect(
    screen.queryByRole('button', { name: 'Approve output preparation' }),
  ).not.toBeInTheDocument();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([request]) => request.kind === 'organization_copy_start'),
  ).toBe(false);
});
it('retains an incomplete preparation error without exposing any approval', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ available: true, reason: 'test' })
    .mockRejectedValueOnce(new Error('Preparation has no complete retained namespace proof.'));
  render(<ReopenPreparedCopy job={{ ...job('prepare_names', true), readyReviewId: null }} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and reopen preparation' }),
  );
  await screen.findByText('Preparation has no complete retained namespace proof.');
  expect(
    screen.queryByRole('button', { name: 'Approve byte-verified copy' }),
  ).not.toBeInTheDocument();
});
it('discards a late preparation response after the job view closes', async () => {
  const stopped = { ...job('prepare_names', true), readyReviewId: null };
  const response = { jobId: stopped.id, manifest: reopenedManifest, review: copy };
  let resolve!: (value: typeof response) => void;
  const pending = new Promise<typeof response>((done) => {
    resolve = done;
  });
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    if (request.kind === 'organization_copy_capabilities')
      return { available: true, reason: 'test' };
    if (request.kind === 'organization_copy_preparation_reopen') return pending;
    throw new Error(`Unexpected request ${request.kind}`);
  });
  const view = render(<ReopenPreparedCopy job={stopped} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and reopen preparation' }),
  );
  await waitFor(() =>
    expect(
      vi
        .mocked(libraryData)
        .mock.calls.some(([request]) => request.kind === 'organization_copy_preparation_reopen'),
    ).toBe(true),
  );
  view.unmount();
  await act(async () => {
    resolve(response);
  });
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({ kind: 'organization_copy_discard', review_id: 'copy' }),
  );
});

const mixedManifest: CopyManifest = {
  ...manifest,
  selectedBytes: '43',
  files: [
    {
      id: 'photo',
      rootId: 'root',
      sourceName: 'photo.JPG',
      sourceRevision: 'a'.repeat(64),
      targetRelativePath: 'after/photo.JPG',
      bytes: '42',
      bundleAssetIds: ['new-photo'],
    },
    {
      id: 'existing',
      rootId: 'root',
      sourceName: 'existing.JPG',
      sourceRevision: 'a'.repeat(64),
      targetRelativePath: 'after/existing.JPG',
      bytes: '1',
      bundleAssetIds: ['existing-photo'],
    },
  ],
};
const mixedPlan: ImportPlan = {
  id: 'import',
  planId: 'plan',
  manifestId: 'manifest',
  manifestDigest: manifest.digest,
  digest: 'a'.repeat(64),
  createdAt: 'now',
  sourceRootId: 'root',
  scopeRootIds: ['keeper-root'],
  choices: [
    { fileId: 'photo', existingGroupId: null },
    { fileId: 'existing', existingGroupId: 'b'.repeat(64) },
  ],
  copyFileIds: ['photo'],
  skipFiles: [
    {
      fileId: 'existing',
      keeperAssetId: 'keeper',
      keeperRootId: 'keeper-root',
      keeperRelativePath: 'old.JPG',
      digest: 'c'.repeat(64),
      sameNativeObject: false,
    },
  ],
  sourcesRetained: true,
  executionAvailable: false,
};
it.each([false, true])(
  'uses exact import review and requires matching receipt (foreign receipt: %s)',
  async (foreign) => {
    vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
      switch (request.kind) {
        case 'organization_copy_capabilities':
          return { available: true, reason: 'Sources retained' };
        case 'organization_import_copy_review':
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
            true,
          );
        case 'organization_import_copy_receipt':
          return {
            jobId: 'copy-job',
            planId: 'plan',
            manifestId: 'manifest',
            importPlanId: foreign ? 'foreign' : 'import',
            importDigest: mixedPlan.digest,
            copyReviewId: 'copy',
            copiedFileIds: ['photo'],
            skippedFiles: mixedPlan.skipFiles,
            keeperBytesRead: '4',
            verifiedAt: 'now',
            sourcesRetained: true,
          };
        default:
          throw new Error(`Unexpected request: ${request.kind}`);
      }
    });
    render(<VerifiedCopyWorkflow manifest={mixedManifest} importPlan={mixedPlan} />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Choose destination and review preparation' }),
    );
    await screen.findByRole('heading', { name: 'Output preparation review' });
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'organization_import_copy_review',
        plan_id: 'plan',
        import_plan_id: 'import',
        selection_id: 'native-selection',
      },
      'organization_copy_review',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Approve output preparation' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Start output preparation' }));
    await screen.findByRole('heading', { name: 'Prepared copy review' });
    expect(screen.getByRole('button', { name: 'Approve byte-verified copy' })).toBeDisabled();
    fireEvent.click(screen.getByRole('checkbox'));
    fireEvent.click(screen.getByRole('button', { name: 'Approve byte-verified copy' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Start verified copy' }));
    if (foreign) {
      await screen.findByText(
        'The completed import receipt does not match this job and saved decisions.',
      );
      expect(screen.queryByText(/^Import recorded:/)).not.toBeInTheDocument();
    } else
      await screen.findByText(
        'Import recorded: 1 new file copied and 1 existing file freshly verified. Sources retained.',
      );
    expect(
      vi
        .mocked(libraryData)
        .mock.calls.some(
          ([request]) =>
            request.kind === 'organization_copy_effect_match' ||
            request.kind === 'organization_copy_review_create',
        ),
    ).toBe(false);
  },
);
it('rejects an imported review containing a skipped file before exposing approval', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ available: true, reason: 'test' })
    .mockResolvedValueOnce({
      ...preparation,
      files: [...preparation.files, { ...preparation.files[0]!, id: 'existing' }],
    });
  render(<VerifiedCopyWorkflow manifest={mixedManifest} importPlan={mixedPlan} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and review preparation' }),
  );
  await screen.findByText(
    'The import copy review differs from the exact saved new-file decisions.',
  );
  expect(
    screen.queryByRole('button', { name: 'Approve output preparation' }),
  ).not.toBeInTheDocument();
});

it.each(['review', 'approval', 'start'] as const)(
  'releases a late mixed-import %s response after closing',
  async (phase) => {
    let resolve!: (value: CopyActionReview | CopyActionApproval | CopyWorkflowJob) => void;
    vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
      if (request.kind === 'organization_copy_capabilities')
        return { available: true, reason: 'test' };
      if (request.kind === 'organization_import_copy_review')
        return new Promise((done) => {
          resolve = done;
        });
      if (request.kind === 'organization_copy_approve')
        return phase === 'approval'
          ? new Promise((done) => {
              resolve = done;
            })
          : {
              id: 'approval',
              reviewId: 'preparation',
              action: 'prepare_names',
              expiresInSeconds: 600,
            };
      if (request.kind === 'organization_copy_start')
        return new Promise((done) => {
          resolve = done;
        });
      throw new Error(`Unexpected request: ${request.kind}`);
    });
    const view = render(
      <VerifiedCopyWorkflow
        manifest={mixedManifest}
        importPlan={mixedPlan}
        {...(phase === 'review'
          ? {}
          : {
              initial: {
                review: preparation,
                selection: {
                  id: 'native-selection',
                  displayPath: 'Chosen destination',
                  identityAvailable: true,
                },
              },
            })}
      />,
    );
    if (phase === 'review')
      fireEvent.click(
        screen.getByRole('button', { name: 'Choose destination and review preparation' }),
      );
    else {
      fireEvent.click(screen.getByRole('button', { name: 'Approve output preparation' }));
      if (phase === 'start')
        fireEvent.click(await screen.findByRole('button', { name: 'Start output preparation' }));
    }
    await waitFor(() => expect(resolve).toBeTypeOf('function'));
    view.unmount();
    vi.mocked(library).mockClear();
    await act(async () =>
      resolve(
        phase === 'review'
          ? preparation
          : phase === 'approval'
            ? {
                id: 'approval',
                reviewId: 'preparation',
                action: 'prepare_names',
                expiresInSeconds: 600,
              }
            : job('prepare_names', false),
      ),
    );
    await waitFor(() =>
      expect(library).toHaveBeenCalledWith(
        phase === 'start'
          ? { kind: 'organization_copy_cancel', job_id: 'prepare-job' }
          : { kind: 'organization_copy_discard', review_id: 'preparation' },
      ),
    );
  },
);

it('reopens a prepared mixed import using its full manifest and exact saved subset', async () => {
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    if (request.kind === 'organization_copy_capabilities')
      return { available: true, reason: 'test' };
    if (request.kind === 'organization_copy_preparation_reopen')
      return { jobId: 'prepare-job', manifest: mixedManifest, review: copy, importPlan: mixedPlan };
    throw new Error(`Unexpected request: ${request.kind}`);
  });
  render(<ReopenPreparedCopy job={job('prepare_names', true)} />);
  fireEvent.click(
    screen.getByRole('button', { name: 'Choose destination and reopen preparation' }),
  );
  await screen.findByRole('heading', { name: 'Prepared copy review' });
  expect(screen.getByRole('checkbox')).not.toBeChecked();
  expect(screen.getByRole('button', { name: 'Approve byte-verified copy' })).toBeDisabled();
  expect(screen.queryByRole('button', { name: 'Start verified copy' })).not.toBeInTheDocument();
  expect(screen.queryByText(/^Import recorded:/)).not.toBeInTheDocument();
});
