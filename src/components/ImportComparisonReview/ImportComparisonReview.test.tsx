import { act, fireEvent, render, screen, waitFor } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type {
  CopyManifest,
  ImportComparisonJob,
  LibraryRequest,
  LibraryStatus,
} from '../../bindings';
import { library, libraryData } from '../../ipc';
import { defaultFilter } from '../../library/model';
import { ImportComparisonReview } from '@/components/ImportComparisonReview';
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  library: vi.fn(),
  libraryData: vi.fn(),
}));
const manifest: CopyManifest = {
  id: 'manifest',
  planId: 'plan',
  digest: 'manifest-digest',
  draftDigest: 'draft',
  createdAt: 'now',
  bundleConfirmationIds: [],
  files: [
    {
      id: 'source-file',
      rootId: 'source-root',
      sourceName: 'source/photo.JPG',
      sourceRevision: 'revision',
      targetRelativePath: 'after/photo.JPG',
      bytes: '42',
      bundleAssetIds: ['photo'],
    },
  ],
  issues: [],
  selectedBytes: '42',
  executionAvailable: false,
};
const status: LibraryStatus = {
  roots: ['source-root', 'keeper-root'].map((id, index) => ({
    id,
    label: index ? 'Existing indexed library' : 'Selected source',
    displayPath: id,
    mode: 'referenced',
    isBase: !index,
    availability: 'online',
    filter: defaultFilter,
    lastScan: 'now',
    fileCount: 1,
  })),
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
function queued(): ImportComparisonJob {
  return {
    id: 'comparison',
    planId: 'plan',
    manifestId: 'manifest',
    sourceRootId: 'source-root',
    scopeRootIds: ['keeper-root'],
    state: 'queued',
    stage: 'admitting',
    candidateFiles: 0,
    processedCandidates: 0,
    comparedSourceFiles: 0,
    totalSourceFiles: 1,
    bytesRead: '0',
    report: null,
    error: null,
  };
}
function completed(): ImportComparisonJob {
  return {
    ...queued(),
    state: 'completed',
    stage: 'finished',
    candidateFiles: 1,
    processedCandidates: 1,
    comparedSourceFiles: 1,
    bytesRead: '252',
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
      bundles: [
        {
          bundleAssetIds: ['photo'],
          fileIds: ['source-file'],
          disposition: 'review_existing_bundle',
          existingGroups: [
            {
              id: 'b'.repeat(64),
              rootId: 'keeper-root',
              members: [{ fileId: 'source-file', assetId: 'keeper' }],
            },
          ],
          reason: 'Complete observed group; fresh verification remains required.',
        },
      ],
      files: [
        {
          fileId: 'source-file',
          sourcePath: 'source/photo.JPG',
          bytes: '42',
          state: 'exact_bytes_compared',
          matches: [
            {
              assetId: 'keeper',
              rootId: 'keeper-root',
              relativePath: 'renamed/<img onerror=bad>.JPG',
              displayTruncated: false,
              digest: 'a'.repeat(64),
              sameNativeObject: false,
            },
          ],
          furtherCandidatesUncompared: false,
        },
      ],
      issues: [],
    },
  };
}
beforeEach(() => {
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  vi.mocked(libraryData).mockReset();
});
function mockRequests(result: ImportComparisonJob = completed()) {
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    switch (request.kind) {
      case 'organization_copy_capabilities':
        return { available: true, reason: 'test' };
      case 'status':
        return status;
      case 'organization_import_comparison_start':
        return queued();
      case 'organization_import_comparison_job':
        return result;
      case 'organization_import_comparison_cancel':
        return { ...queued(), state: 'canceled', stage: 'finished' };
      case 'organization_copy_io':
        return { aggregateBytesPerSecond: '10485760', byteIoPaused: false, burstBytes: 131072 };
      default:
        throw new Error(`Unexpected request ${request.kind}`);
    }
  });
}
async function chooseScope() {
  fireEvent.click(screen.getByRole('button', { name: 'Choose indexed comparison scope' }));
  await screen.findByRole('checkbox', { name: 'Existing indexed library · online' });
  expect(
    screen.getByRole('checkbox', { name: 'Selected source · online · selected source' }),
  ).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Start read-only comparison' })).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox', { name: 'Existing indexed library · online' }));
  fireEvent.click(screen.getByRole('button', { name: 'Start read-only comparison' }));
}
it.each([false, true])(
  'scopes a multiple-library comparison to one referenced source and refuses a foreign job (%s)',
  async (foreign) => {
    mockRequests();
    const normal = vi.mocked(libraryData).getMockImplementation()!;
    const second = { ...status.roots[0]!, id: 'second-source', label: 'Second source' };
    vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest, ...rest) => {
      if (request.kind === 'status') return { ...status, roots: [...status.roots, second] };
      if (request.kind === 'organization_import_comparison_start')
        return foreign ? { ...queued(), sourceRootId: 'second-source' } : queued();
      return normal(request, ...rest);
    });
    const multiple = {
      ...manifest,
      files: [
        ...manifest.files,
        {
          ...manifest.files[0]!,
          id: 'second-file',
          rootId: 'second-source',
          bytes: '49',
          bundleAssetIds: ['second-photo'],
        },
      ],
      selectedBytes: '91',
    };
    render(<ImportComparisonReview manifest={multiple} />);
    const source = await screen.findByRole('combobox', { name: 'Source library to compare' });
    expect(source).toHaveValue('');
    expect(screen.getByRole('option', { name: /Selected source/ })).toBeEnabled();
    expect(
      screen.queryByRole('button', { name: 'Choose indexed comparison scope' }),
    ).not.toBeInTheDocument();
    fireEvent.change(source, { target: { value: 'source-root' } });
    await chooseScope();
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'organization_import_comparison_start',
        input: {
          planId: 'plan',
          manifestId: 'manifest',
          scopeRootIds: ['keeper-root'],
          sourceRootId: 'source-root',
        },
      },
      'organization_import_comparison_job',
    );
    if (foreign) {
      expect(await screen.findByRole('alert')).toHaveTextContent('another source or library scope');
      expect(
        screen.queryByRole('heading', { name: 'Observed file comparison' }),
      ).not.toBeInTheDocument();
    } else {
      await screen.findByRole('heading', { name: 'Observed file comparison' });
      expect(source).toBeDisabled();
      fireEvent.click(screen.getByRole('button', { name: 'Close comparison report' }));
      await waitFor(() => expect(source).toBeEnabled());
      expect(library).toHaveBeenCalledWith({
        kind: 'organization_import_comparison_discard',
        job_id: 'comparison',
      });
      fireEvent.change(source, { target: { value: 'second-source' } });
      expect(
        screen.queryByRole('heading', { name: 'Observed file comparison' }),
      ).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Choose indexed comparison scope' })).toBeEnabled();
    }
  },
);
it('compares only an explicit root scope and renders byte evidence as text without skip/removal authority', async () => {
  mockRequests();
  render(<ImportComparisonReview manifest={manifest} />);
  await chooseScope();
  await screen.findByRole('heading', { name: 'Observed file comparison' });
  expect(screen.getByText('Complete bytes compared equal')).toBeInTheDocument();
  expect(screen.getByText('renamed/<img onerror=bad>.JPG')).toBeInTheDocument();
  expect(document.querySelector('img')).toBeNull();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'organization_import_comparison_start',
      input: { planId: 'plan', manifestId: 'manifest', scopeRootIds: ['keeper-root'] },
    },
    'organization_import_comparison_job',
  );
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([request]) => request.kind === 'organization_copy_start'),
  ).toBe(false);
});
it('keeps incomplete indexed coverage unknown rather than claiming a new file', async () => {
  const result = completed();
  result.state = 'completed_with_issues';
  result.report!.allScopeAvailable = false;
  result.report!.files[0]!.state = 'needs_attention';
  result.report!.files[0]!.matches = [];
  result.report!.bundles[0]!.disposition = 'needs_attention';
  result.report!.bundles[0]!.existingGroups = [];
  result.report!.issues = [
    {
      rootId: 'keeper-root',
      assetId: 'keeper',
      error: { code: 'IO', message: 'Indexed file is inaccessible.' },
    },
  ];
  mockRequests(result);
  render(<ImportComparisonReview manifest={manifest} />);
  await chooseScope();
  await screen.findByText('Unknown; review comparison errors');
  expect(screen.getByText(/Comparison coverage is incomplete/)).toBeInTheDocument();
  expect(screen.queryByText('No match in the indexed scope')).not.toBeInTheDocument();
});
it('rejects a foreign comparison response without showing its report', async () => {
  const result = completed();
  result.manifestId = 'foreign';
  mockRequests(result);
  render(<ImportComparisonReview manifest={manifest} />);
  await chooseScope();
  await screen.findByRole('alert');
  expect(
    screen.queryByRole('heading', { name: 'Observed file comparison' }),
  ).not.toBeInTheDocument();
});
it('shows whole-bundle evidence without granting skip permission', async () => {
  mockRequests();
  render(<ImportComparisonReview manifest={manifest} />);
  await chooseScope();
  await screen.findByText('Whole-bundle import observations (1)');
  expect(screen.getByText(/1 complete existing groups to review/)).toBeInTheDocument();
  expect(screen.getByText(/fresh complete-byte verification and/)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /skip|import now/i })).not.toBeInTheDocument();
});
it('rejects bundle evidence pointing at an unverified keeper', async () => {
  const result = completed();
  result.report!.bundles[0]!.existingGroups[0]!.members[0]!.assetId = 'foreign';
  mockRequests(result);
  render(<ImportComparisonReview manifest={manifest} />);
  await chooseScope();
  await screen.findByRole('alert');
  expect(screen.queryByText('Whole-bundle import observations (1)')).not.toBeInTheDocument();
});
it('cancels at a read boundary and discards a retained job when its view closes', async () => {
  mockRequests(queued());
  const view = render(<ImportComparisonReview manifest={manifest} />);
  await chooseScope();
  fireEvent.click(await screen.findByRole('button', { name: 'Cancel read-only comparison' }));
  await screen.findByText(
    'Comparison canceled at a safe read boundary. Rerun to obtain a complete report.',
  );
  view.unmount();
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({
      kind: 'organization_import_comparison_discard',
      job_id: 'comparison',
    }),
  );
});
it('discards a valid late queued response after unmounting', async () => {
  let resolve!: (job: ImportComparisonJob) => void;
  const pending = new Promise<ImportComparisonJob>((done) => {
    resolve = done;
  });
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    if (request.kind === 'organization_copy_capabilities')
      return { available: true, reason: 'test' };
    if (request.kind === 'status') return status;
    if (request.kind === 'organization_import_comparison_start') return pending;
    throw new Error(`Unexpected request ${request.kind}`);
  });
  const view = render(<ImportComparisonReview manifest={manifest} />);
  await chooseScope();
  await waitFor(() =>
    expect(
      vi
        .mocked(libraryData)
        .mock.calls.some(([request]) => request.kind === 'organization_import_comparison_start'),
    ).toBe(true),
  );
  view.unmount();
  await act(async () => {
    resolve(queued());
  });
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({
      kind: 'organization_import_comparison_discard',
      job_id: 'comparison',
    }),
  );
});
