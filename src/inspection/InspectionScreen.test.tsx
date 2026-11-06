import { fireEvent, render, screen, waitFor, within } from '../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import { InspectionScreen } from './InspectionScreen';
import { libraryData } from '../ipc';
import type { InspectionJob, LibraryStatus } from '../bindings';
import { fixturePreferences } from '../library/__tests__/fixtures';
import { storageFiles, storageSummary } from '../components/StorageExplorer/fixtures';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
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
  fireEvent.click(screen.getByRole('button', { name: 'Storage measurement' }));
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
  expect(screen.getByRole('button', { name: /Establish/ })).toBeEnabled();
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
  fireEvent.click(screen.getByRole('button', { name: 'Storage measurement' }));
  expect(screen.getByRole('button', { name: 'Start storage inspection' })).toBeDisabled();
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
});

it('starts Storage results at the filters and keeps retained controls, issues and coverage in the header dialog', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'inspection_jobs')
      return [{ ...job, state: 'completed', message: 'Retained generated measurement' }];
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
  expect(container.querySelector('.inspection-content')?.firstElementChild).toHaveClass(
    'storage-explorer',
  );
  expect(screen.queryByText('Storage measurement settings')).not.toBeInTheDocument();
  expect(screen.queryByLabelText('Retained inspection')).not.toBeInTheDocument();
  expect(screen.queryByRole('region', { name: 'Inspection job' })).not.toBeInTheDocument();
  expect(screen.queryByText(/Inventory:/)).not.toBeInTheDocument();
  const opener = screen.getByRole('button', { name: 'Storage measurement' });
  fireEvent.click(opener);
  const dialog = screen.getByRole('dialog', { name: 'Storage measurement' });
  expect(within(dialog).getByText('Storage measurement settings')).toBeVisible();
  expect(within(dialog).getByLabelText('Retained inspection')).toBeVisible();
  expect(within(dialog).getByText('Retained generated measurement')).toBeVisible();
  expect(within(dialog).getByText(/Inventory:/)).toHaveTextContent('3 incomplete directories');
  fireEvent.click(
    within(dialog).getByRole('button', { name: 'Review retained inspection issues' }),
  );
  await within(dialog).findByText('No retained issues for this report.');
  fireEvent.click(within(dialog).getByRole('button', { name: 'Close' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(opener).toHaveFocus();
  fireEvent.click(opener);
  fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(opener).toHaveFocus();
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
  expect(screen.getByRole('button', { name: 'Start duplicate inspection' })).toBeEnabled();
  fireEvent.click(screen.getByRole('radio', { name: 'Matching file properties' }));
  fireEvent.click(await screen.findByRole('checkbox', { name: 'Base filename' }));
  expect(screen.getByRole('button', { name: 'Start duplicate inspection' })).toBeDisabled();
  expect(screen.getByText('Choose at least one property criterion.')).toBeVisible();
  fireEvent.click(screen.getByRole('checkbox', { name: 'Logical size' }));
  expect(screen.getByRole('button', { name: 'Start duplicate inspection' })).toBeEnabled();
});
