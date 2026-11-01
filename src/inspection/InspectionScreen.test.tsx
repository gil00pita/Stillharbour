import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { InspectionScreen } from './InspectionScreen';
import { libraryData } from '../ipc';
import type { InspectionJob, LibraryStatus } from '../bindings';
import { fixturePreferences } from '../library/__tests__/fixtures';
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
  expect(screen.getByRole('button', { name: 'Start storage inspection' })).toBeDisabled();
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
});
