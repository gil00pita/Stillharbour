import { fireEvent, render, screen, waitFor, within } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import { InspectionScreen } from '@/components/InspectionScreen';
import { libraryData } from '../../ipc';
import type { InspectionJob, InspectionReport, LibraryStatus } from '../../bindings';
import { fixturePreferences } from '../../library/__tests__/fixtures';
import { storageFiles, storageSummary } from '../StorageExplorer/fixtures';
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
}));
const status: LibraryStatus = {
  roots: [
    {
      id: 'root',
      label: 'Generated root',
      displayPath: '/generated',
      mode: 'referenced',
      availability: 'online',
      filter: fixturePreferences().scanFilter,
      lastScan: null,
      fileCount: 0,
      isBase: false,
    },
  ],
  jobs: [],
  revision: '0',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
const job: InspectionJob = {
  id: 'job',
  input: { rootIds: ['root'], scope: 'all_eligible_files', task: { kind: 'storage' } },
  state: 'running',
  startedAt: '2026-11-01',
  updatedAt: '2026-11-01',
  finishedAt: null,
  filesObserved: 0,
  filesChecked: 0,
  bytesRead: '0',
  issues: 0,
  limited: false,
  message: 'Reading generated scope',
};
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'inspection_jobs') return [];
      if (request.kind === 'inspection_start') return { ...job, input: request.input };
      if (request.kind === 'inspection_job') return job;
      if (request.kind === 'inspection_cancel') return { ...job, state: 'cancel_requested' };
      throw new Error('Unexpected inspection request');
    });
});
function setup(section: 'Storage' | 'Integrity' | 'Duplicates' = 'Storage', value = status) {
  return render(
    <InspectionScreen
      section={section}
      status={value}
      navigationControl={null}
      jobsControl={null}
    />,
  );
}
it('starts a native job with selected IDs and explicit scope, then waits for native cancellation', async () => {
  setup();
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith({ kind: 'inspection_jobs' }, 'inspection_jobs'),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Start storage inspection' }));
  await screen.findByText('Reading generated scope');
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'inspection_start',
      input: { rootIds: ['root'], scope: 'all_eligible_files', task: { kind: 'storage' } },
    },
    'inspection_job',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Cancel inspection' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Cancel inspection' })).toBeDisabled(),
  );
  expect(screen.getByRole('heading', { name: 'cancel requested' })).toBeVisible();
});
it('requires a separate acknowledgement before establishing append-only baselines', async () => {
  setup('Integrity');
  fireEvent.change(screen.getByLabelText('Integrity mode'), {
    target: { value: 'establish_baseline' },
  });
  expect(screen.getByRole('button', { name: /Establish/ })).toBeDisabled();
  const acknowledgement = screen.getByRole('checkbox', { name: /baseline/i });
  fireEvent.click(acknowledgement);
  await waitFor(() => expect(screen.getByRole('button', { name: /Establish/ })).toBeEnabled());
  expect(libraryData).not.toHaveBeenCalledWith(
    expect.objectContaining({ kind: 'inspection_start' }),
    expect.anything(),
  );
});
it('blocks an offline root and renders native failures without fake results', async () => {
  vi.mocked(libraryData).mockRejectedValue(new Error('Catalog unavailable'));
  setup('Storage', {
    ...status,
    roots: status.roots.map((root) => ({ ...root, availability: 'offline' })),
  });
  expect(screen.getByRole('button', { name: 'Start storage inspection' })).toBeDisabled();
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
});

it('uses illustrated radio choices without starting an inspection until explicit approval', async () => {
  setup('Duplicates');
  const exact = screen.getByRole('radio', { name: 'Identical file contents' });
  expect(exact).toBeChecked();
  expect(screen.getByRole('radiogroup', { name: 'Match mode' })).toBeVisible();
  expect(screen.queryByRole('combobox', { name: 'Match mode' })).not.toBeInTheDocument();
  expect(document.querySelectorAll('.duplicate-match-options img')).toHaveLength(3);
  fireEvent.click(screen.getByRole('radio', { name: 'Matching file properties' }));
  expect(screen.getByRole('radio', { name: 'Matching file properties' })).toBeChecked();
  expect(exact).not.toBeChecked();
  expect(await screen.findByRole('group', { name: 'Advanced candidate rules' })).toBeVisible();
  fireEvent.click(screen.getByRole('radio', { name: 'Any selected criterion' }));
  await waitFor(() =>
    expect(
      screen.getByRole('radio', { name: 'Any selected criterion' }).closest('label'),
    ).toHaveAttribute('data-state', 'checked'),
  );
  fireEvent.click(screen.getByRole('radio', { name: 'All eligible files' }));
  await waitFor(() =>
    expect(
      screen.getByRole('radio', { name: 'All eligible files' }).closest('label'),
    ).toHaveAttribute('data-state', 'checked'),
  );
  fireEvent.click(screen.getByText('Advanced matching options'));
  fireEvent.click(screen.getByRole('radio', { name: 'Direct bytes' }));
  expect(libraryData).not.toHaveBeenCalledWith(
    expect.objectContaining({ kind: 'inspection_start' }),
    expect.anything(),
  );
  await waitFor(() =>
    expect(screen.getByRole('radio', { name: 'Direct bytes' }).closest('label')).toHaveAttribute(
      'data-state',
      'checked',
    ),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Start duplicate inspection' }));
  await screen.findByText('Reading generated scope');
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'inspection_start',
      input: {
        rootIds: ['root'],
        scope: 'all_eligible_files',
        task: {
          kind: 'duplicates',
          options: expect.objectContaining({
            mode: 'properties',
            algorithm: 'direct_bytes',
            rules: expect.objectContaining({ logic: 'any', filename: true }),
          }),
        },
      },
    },
    'inspection_job',
  );
  expect(screen.getByRole('radio', { name: 'Identical file contents' })).toBeDisabled();
  expect(screen.getByRole('radio', { name: 'All eligible files' })).toBeDisabled();
  expect(screen.getByRole('radio', { name: 'Direct bytes' })).toBeDisabled();
});

it('preserves PNG-only manual review and blocks empty property rules with card selection', async () => {
  setup('Duplicates');
  fireEvent.click(screen.getByRole('radio', { name: 'Static PNG8 pixels' }));
  expect(screen.getByText('Manual review only')).toBeVisible();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Start duplicate inspection' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('radio', { name: 'Matching file properties' }));
  fireEvent.click(await screen.findByRole('checkbox', { name: 'Base filename' }));
  expect(screen.getByRole('button', { name: 'Start duplicate inspection' })).toBeDisabled();
  expect(screen.getByText('Choose at least one property criterion.')).toBeVisible();
  fireEvent.click(screen.getByRole('checkbox', { name: 'Logical size' }));
  expect(screen.getByRole('button', { name: 'Start duplicate inspection' })).toBeEnabled();
});

it.each(['Duplicates', 'Integrity', 'Storage'] as const)(
  'opens %s with setup and no empty result workspace or automatic first scan',
  async (section) => {
    setup(section);
    expect(await screen.findByRole('region', { name: 'Scan introduction' })).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Duplicate groups' })).not.toBeInTheDocument();
    expect(
      vi.mocked(libraryData).mock.calls.some(([request]) => request.kind === 'inspection_start'),
    ).toBe(false);
    fireEvent.click(screen.getByRole('button', { name: 'Scan history' }));
    expect(await screen.findByText('Your scans will appear here.')).toBeVisible();
  },
);

const savedReport = (id: string): InspectionReport => ({
  jobId: id,
  volumes: [],
  coverage: {
    enumerationComplete: true,
    excludedEntries: 0,
    inaccessibleEntries: 0,
    incompleteDirectories: 0,
  },
  exactOrCandidateGroups: 0,
  aliasGroups: 0,
  comparedPairs: '0',
  unknownPairs: '0',
  storage: null,
  id3Applicability: 'Not applicable',
});

it('keeps saved results visible during actual reconciliation and lets history stay selected as the scan completes', async () => {
  const saved: InspectionJob = {
    ...job,
    id: 'saved',
    state: 'completed',
    filesChecked: 7,
    finishedAt: '2026-11-01',
  };
  let state: InspectionJob['state'] = 'running';
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_jobs') return [saved];
    if (request.kind === 'inspection_start') return { ...job, id: 'refresh' };
    if (request.kind === 'inspection_job') return { ...job, id: 'refresh', state };
    if (request.kind === 'inspection_report') return savedReport(request.job_id);
    if (request.kind === 'inspection_findings')
      return { jobId: request.job_id, rows: [], next: null };
    throw new Error('Unexpected request');
  });
  setup();
  expect(await screen.findByText(/Showing scan from/)).toHaveTextContent('completed');
  expect(await screen.findByText(/Checking for changes in the background/)).toBeVisible();
  expect(screen.getByText(/Showing scan from/)).toBeVisible();
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'inspection_start', input: saved.input },
    'inspection_job',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Scan history' }));
  const versions = await screen.findByLabelText('Saved scan versions');
  fireEvent.click(within(versions).getByRole('button', { name: /Storage usage.*completed/ }));
  state = 'completed';
  expect(
    await screen.findByText('Latest scan finished. Earlier versions remain available in history.'),
  ).toBeVisible();
  expect(screen.getByText(/Showing scan from/)).toHaveTextContent('Historical version');
  expect(libraryData).not.toHaveBeenCalledWith(
    { kind: 'inspection_report', job_id: 'refresh' },
    'inspection_report',
  );
});

it('reconciles baseline establishment as comparison only and retains offline reports without reading originals', async () => {
  const saved: InspectionJob = {
    ...job,
    state: 'completed',
    input: { ...job.input, task: { kind: 'health', mode: 'establish_baseline' } },
  };
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_jobs') return [saved];
    if (request.kind === 'inspection_start') return { ...job, input: request.input };
    if (request.kind === 'inspection_report') return savedReport(request.job_id);
    if (request.kind === 'inspection_findings')
      return { jobId: request.job_id, rows: [], next: null };
    if (request.kind === 'inspection_job') return job;
    throw new Error('Unexpected request');
  });
  const view = setup('Integrity');
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'inspection_start',
        input: { ...saved.input, task: { kind: 'health', mode: 'compare_baseline' } },
      },
      'inspection_job',
    ),
  );
  view.unmount();
  vi.mocked(libraryData).mockClear();
  setup('Integrity', {
    ...status,
    roots: status.roots.map((root) => ({ ...root, availability: 'offline' })),
  });
  expect(await screen.findByText(/Reconnect the selected libraries/)).toBeVisible();
  expect(
    vi.mocked(libraryData).mock.calls.some(([request]) => request.kind === 'inspection_start'),
  ).toBe(false);
});

it('requires explicitly removing a previously registered missing root from the next scan scope', async () => {
  const saved: InspectionJob = {
    ...job,
    state: 'completed',
    input: { ...job.input, rootIds: ['root', 'removed-root'] },
  };
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_jobs') return [saved];
    if (request.kind === 'inspection_report') return savedReport(request.job_id);
    if (request.kind === 'inspection_findings')
      return { jobId: request.job_id, rows: [], next: null };
    if (request.kind === 'inspection_start') return { ...job, input: request.input };
    if (request.kind === 'inspection_job') return job;
    throw new Error('Unexpected request');
  });
  setup();
  await screen.findByText(/Reconnect the selected libraries/);
  expect(
    vi.mocked(libraryData).mock.calls.some(([request]) => request.kind === 'inspection_start'),
  ).toBe(false);
  fireEvent.click(screen.getByRole('button', { name: 'Storage measurement' }));
  expect(screen.getByRole('button', { name: 'Start storage inspection' })).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox', { name: /Previously registered folder/ }));
  fireEvent.click(screen.getByRole('button', { name: 'Start storage inspection' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'inspection_start', input: job.input },
      'inspection_job',
    ),
  );
});

it('starts Storage results at the filters and keeps retained controls, issues and coverage in the header dialog', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_jobs')
      return [{ ...job, state: 'completed', message: 'Retained generated measurement' }];
    if (request.kind === 'inspection_start' || request.kind === 'inspection_job')
      return { ...job, id: 'refresh' };
    if (request.kind === 'inspection_report')
      return {
        jobId: job.id,
        storage: storageSummary,
        volumes: [],
        coverage: {
          enumerationComplete: false,
          excludedEntries: 1,
          inaccessibleEntries: 2,
          incompleteDirectories: 3,
        },
        exactOrCandidateGroups: 0,
        aliasGroups: 0,
        comparedPairs: '0',
        unknownPairs: '0',
        id3Applicability: '',
      };
    if (request.kind === 'inspection_findings')
      return { jobId: job.id, rows: storageFiles.filter((file) => !file.assetId), next: null };
    if (request.kind === 'inspection_issues') return { jobId: job.id, rows: [], next: null };
    throw new Error('Unexpected request');
  });
  const { container } = setup();
  await screen.findByRole('region', { name: 'Storage filters' });
  expect(container.querySelector('.inspection-content > .storage-explorer')).toBeInTheDocument();
  const content = container.querySelector('.inspection-page > .inspection-content');
  const explorer = content?.querySelector('.storage-explorer');
  expect(explorer?.parentElement).toBe(content);
  expect(explorer?.firstElementChild).toBe(screen.getByRole('region', { name: 'Storage filters' }));
  expect(content).toContainElement(screen.getByText(/Showing scan from/));
  expect(screen.getByText(/Showing scan from/)).toHaveTextContent('completed');
  expect(container.querySelector('.app-page-header')).not.toHaveTextContent(
    'Retained generated measurement',
  );
  expect(screen.queryByText('Storage measurement settings')).not.toBeInTheDocument();
  expect(screen.queryByLabelText('Retained inspection')).not.toBeInTheDocument();
  expect(screen.queryByRole('region', { name: 'Inspection job' })).not.toBeInTheDocument();
  expect(screen.queryByText(/Inventory:/)).not.toBeInTheDocument();
  const opener = screen.getByRole('button', { name: 'Storage measurement' });
  fireEvent.click(opener);
  const dialog = screen.getByRole('dialog', { name: 'Storage measurement' });
  expect(within(dialog).getByText('Storage measurement settings')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Scan history' })).toBeInTheDocument();
  expect(within(dialog).getByText('Reading generated scope')).toBeVisible();
  expect(within(dialog).getByText(/Inventory:/)).toHaveTextContent('3 incomplete directories');
  fireEvent.click(
    within(dialog).getByRole('button', { name: 'Review retained inspection issues' }),
  );
  await within(dialog).findByText('No retained issues for this report.');
  fireEvent.click(within(dialog).getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(opener).toHaveFocus();
  fireEvent.click(opener);
  fireEvent.keyDown(within(screen.getByRole('dialog')).getByRole('button', { name: 'Close' }), {
    key: 'Escape',
  });
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(opener).toHaveFocus();
  fireEvent.click(opener);
  fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(opener).toHaveFocus();
});

it('offers an explicit retry after a failed integrity run without replaying it on open', async () => {
  const failed: InspectionJob = {
    ...job,
    state: 'failed',
    message: 'Generated decoder failure',
    input: { ...job.input, task: { kind: 'health', mode: 'deep' } },
  };
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_jobs') return [failed];
    if (request.kind === 'inspection_report') return savedReport(request.job_id);
    if (request.kind === 'inspection_findings') return { jobId: failed.id, rows: [], next: null };
    if (request.kind === 'inspection_start') return { ...job, input: request.input };
    if (request.kind === 'inspection_job') return { ...job, input: failed.input };
    throw new Error('Unexpected request');
  });
  setup('Integrity');
  const retry = await screen.findByRole('button', { name: 'Retry integrity check' });
  await waitFor(() => expect(retry).toBeEnabled());
  expect(
    vi.mocked(libraryData).mock.calls.some(([request]) => request.kind === 'inspection_start'),
  ).toBe(false);
  fireEvent.click(retry);
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'inspection_start', input: failed.input },
      'inspection_job',
    ),
  );
  expect(screen.queryByRole('button', { name: 'Retry integrity check' })).not.toBeInTheDocument();
});

it('retries loading a failed saved report without starting another scan', async () => {
  let unavailable = true;
  const failed: InspectionJob = {
    ...job,
    state: 'failed',
    input: { ...job.input, task: { kind: 'health', mode: 'compare_baseline' } },
  };
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_jobs') return [failed];
    if (request.kind === 'inspection_report') {
      if (unavailable) throw new Error('Report temporarily unavailable');
      return savedReport(request.job_id);
    }
    if (request.kind === 'inspection_findings') return { jobId: failed.id, rows: [], next: null };
    throw new Error('Unexpected request');
  });
  setup('Integrity');
  expect(await screen.findByRole('alert')).toHaveTextContent('Report temporarily unavailable');
  unavailable = false;
  fireEvent.click(screen.getByRole('button', { name: 'Retry loading results' }));
  expect(await screen.findByText(/Showing scan from/)).toBeVisible();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  expect(
    vi.mocked(libraryData).mock.calls.some(([request]) => request.kind === 'inspection_start'),
  ).toBe(false);
});

it('requires a fresh acknowledgement when retrying failed baseline establishment', async () => {
  setup('Integrity');
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Start checksum comparison' })).toBeEnabled(),
  );
  fireEvent.change(screen.getByLabelText('Integrity mode'), {
    target: { value: 'establish_baseline' },
  });
  fireEvent.click(screen.getByRole('checkbox', { name: /baseline/i }));
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_start')
      return { ...job, state: 'failed', input: request.input };
    throw new Error('Unexpected request');
  });
  fireEvent.click(screen.getByRole('button', { name: 'Establish checksum baselines' }));
  const retry = await screen.findByRole('button', { name: 'Retry integrity check' });
  fireEvent.click(retry);
  expect(screen.getByRole('button', { name: 'Establish checksum baselines' })).toBeDisabled();
  expect(screen.getByRole('checkbox', { name: /baseline/i })).not.toBeChecked();
  expect(
    vi.mocked(libraryData).mock.calls.filter(([request]) => request.kind === 'inspection_start'),
  ).toHaveLength(1);
});
