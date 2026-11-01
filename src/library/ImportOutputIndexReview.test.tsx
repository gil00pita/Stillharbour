import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { ImportCopyReceipt, ImportOutputIndex, LibraryRequest } from '../bindings';
import { library, libraryData, pickDirectory } from '../ipc';
import { ImportOutputIndexReview } from './ImportOutputIndexReview';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  library: vi.fn(),
  libraryData: vi.fn(),
  pickDirectory: vi.fn(),
}));
const receipt: ImportCopyReceipt = {
  jobId: 'copy-job',
  planId: 'plan',
  manifestId: 'manifest',
  importPlanId: 'import',
  importDigest: 'a'.repeat(64),
  copyReviewId: 'copy',
  copiedFileIds: ['new'],
  skippedFiles: [],
  keeperBytesRead: '0',
  verifiedAt: 'now',
  sourcesRetained: true,
};
const result: ImportOutputIndex = {
  jobId: 'copy-job',
  root: {
    id: 'output-root',
    label: 'Imported output',
    displayPath: 'Chosen destination/output',
    mode: 'referenced',
    isBase: false,
    availability: 'online',
    filter: {
      version: 1,
      includeGlobs: [],
      excludeGlobs: [],
      excludedFolders: [],
      extensions: [],
      minBytes: null,
      maxBytes: null,
      includeHiddenFiles: false,
      includeHiddenFolders: false,
      fromDate: null,
      throughDate: null,
    },
    lastScan: null,
    fileCount: 0,
  },
  scan: {
    kind: 'scan',
    id: 'scan',
    rootId: 'output-root',
    state: 'queued',
    discovered: 0,
    indexed: 0,
    skipped: 0,
    errors: 0,
    message: 'Queued',
    startedAt: 'now',
  },
  alreadyRegistered: false,
  originalMutationsEnabled: false,
};
beforeEach(() => {
  vi.mocked(libraryData).mockReset();
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  vi.mocked(pickDirectory).mockReset().mockResolvedValue({
    id: 'chosen',
    displayPath: 'Chosen destination',
    identityAvailable: true,
  });
});
it('uses a native selection, reports the real scan and cancels only that scan on request', async () => {
  vi.mocked(libraryData).mockImplementation(async (request: LibraryRequest) => {
    if (request.kind === 'organization_import_index_output') return result;
    if (request.kind === 'job_details')
      return { job: { ...result.scan!, state: 'running', indexed: 1 }, issues: [] };
    throw new Error(`Unexpected ${request.kind}`);
  });
  render(<ImportOutputIndexReview receipt={receipt} sourceRootId="source" />);
  fireEvent.click(screen.getByRole('button', { name: 'Choose destination and index output' }));
  await screen.findByText(/Output folder registered:/);
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'organization_import_index_output', job_id: 'copy-job', selection_id: 'chosen' },
    'organization_import_output_index',
  );
  expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuetext', 'Scan queued');
  await screen.findByText('1 indexed entries · 0 issues.');
  fireEvent.click(screen.getByRole('button', { name: 'Cancel output scan' }));
  await waitFor(() => expect(library).toHaveBeenCalledWith({ kind: 'cancel', job_id: 'scan' }));
});
it.each(['foreign', 'managed', 'thumbnail'] as const)(
  'rejects an unexpected indexing response: %s',
  async (variant) => {
    const value = structuredClone(result);
    if (variant === 'foreign') value.jobId = 'other';
    if (variant === 'managed') value.root.mode = 'managed';
    if (variant === 'thumbnail') value.scan!.kind = 'thumbnails';
    vi.mocked(libraryData).mockResolvedValue(value);
    render(<ImportOutputIndexReview receipt={receipt} sourceRootId="source" />);
    fireEvent.click(screen.getByRole('button', { name: 'Choose destination and index output' }));
    await screen.findByText('The output registration does not match this verified import.');
    expect(screen.queryByText(/Output folder registered:/)).not.toBeInTheDocument();
  },
);
it.each([false, true])(
  'closing a late registration cancels only a newly started scan (existing: %s)',
  async (existing) => {
    let resolve!: (value: ImportOutputIndex) => void;
    vi.mocked(libraryData).mockImplementation(
      async () =>
        new Promise<ImportOutputIndex>((done) => {
          resolve = done;
        }),
    );
    const view = render(<ImportOutputIndexReview receipt={receipt} sourceRootId="source" />);
    fireEvent.click(screen.getByRole('button', { name: 'Choose destination and index output' }));
    await waitFor(() => expect(resolve).toBeTypeOf('function'));
    view.unmount();
    await act(async () => resolve({ ...result, alreadyRegistered: existing }));
    if (existing) expect(library).not.toHaveBeenCalled();
    else expect(library).toHaveBeenCalledWith({ kind: 'cancel', job_id: 'scan' });
  },
);
