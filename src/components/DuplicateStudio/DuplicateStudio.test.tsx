import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '../../test/render';
import { DuplicateStudio } from './DuplicateStudio';
import { libraryData } from '../../ipc';
import type { InspectionJob, LibraryStatus } from '../../bindings';
import { group, member } from './fixtures';
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
}));
const status: LibraryStatus = {
  roots: [],
  jobs: [],
  revision: '1',
  previewGeneration: '1',
  metadataAvailable: true,
  previewAvailable: true,
};
const job: InspectionJob = {
  id: 'job',
  input: { rootIds: [], scope: 'all_eligible_files', task: { kind: 'storage' } },
  state: 'completed',
  startedAt: '',
  updatedAt: '',
  finishedAt: '',
  filesObserved: 2,
  filesChecked: 2,
  bytesRead: '200',
  issues: 0,
  limited: false,
  message: 'Complete',
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
      if (request.kind === 'inspection_group_members')
        return { jobId: 'job', rows: [member('a'), member('b')], next: null } as never;
      throw new Error('Unexpected request');
    });
});
function setup() {
  return render(
    <DuplicateStudio
      job={job}
      groups={{
        jobId: 'job',
        groups: [
          group,
          { ...group, id: 'candidate', evidence: 'property_candidate', memberIds: ['c'] },
        ],
        next: null,
      }}
      findings={{
        jobId: 'job',
        rows: [member('a', { name: 'Coast.png' }), member('c', { name: 'Mountain.png' })],
        next: null,
      }}
      status={status}
      next={vi.fn()}
    />,
  );
}
it('searches observed files, separates exact and uncertain evidence, and selects only shown clusters', async () => {
  setup();
  await screen.findByRole('heading', { name: 'Choose what stays' });
  fireEvent.click(screen.getByRole('button', { name: 'Needs review' }));
  expect(screen.getByRole('button', { name: /Mountain.png/ })).toBeVisible();
  expect(screen.queryByRole('button', { name: /Coast.png/ })).not.toBeInTheDocument();
  fireEvent.click(screen.getByLabelText('Select shown groups'));
  expect(screen.getByText('1 selected')).toBeVisible();
  fireEvent.change(screen.getByRole('searchbox', { name: 'Search duplicate groups' }), {
    target: { value: 'missing' },
  });
  expect(screen.getByText('No groups match')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Can automate' }));
  expect(screen.getByText('Automatic cleanup is unavailable')).toBeVisible();
});
it('keeps live threshold preview and confirmation fail-closed, without simulating removal or Undo', async () => {
  setup();
  fireEvent.click(screen.getByRole('button', { name: 'Cleanup rules' }));
  const rules = screen.getByRole('dialog', { name: 'Cleanup rules' });
  fireEvent.change(within(rules).getByRole('slider', { name: 'Minimum visual confidence' }), {
    target: { value: '100' },
  });
  expect(within(rules).getByRole('status')).toHaveTextContent('0 scored groups meet 100%');
  fireEvent.click(within(rules).getByRole('button', { name: 'Preview cleanup' }));
  expect(
    within(rules).getByRole('button', { name: 'Move to Recently Deleted unavailable' }),
  ).toBeDisabled();
  fireEvent.change(within(rules).getByLabelText('Retain this version'), {
    target: { value: 'largest' },
  });
  fireEvent.click(within(rules).getByRole('button', { name: 'Close' }));
  expect(screen.getByRole('button', { name: 'Cleanup rules' })).toHaveFocus();
  expect(await screen.findByLabelText('Keeper suggestion')).toHaveValue('largest');
  fireEvent.click(screen.getByRole('button', { name: 'Cleanup rules' }));
  expect(screen.getByRole('slider', { name: 'Minimum visual confidence' })).toHaveValue('100');
  fireEvent.click(
    screen.getByRole('dialog', { name: 'Cleanup rules' }).querySelector('header button')!,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Recently Deleted' }));
  expect(screen.getByRole('heading', { name: 'No files have been moved' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Undo removal unavailable' })).toBeDisabled();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.every(([request]) => request.kind === 'inspection_group_members'),
  ).toBe(true);
});
it('suggests conservative keepers and displays lookup failures instead of an empty success', async () => {
  setup();
  fireEvent.click(await screen.findByRole('button', { name: 'Suggest keepers' }));
  expect(screen.getAllByText('Keep', { exact: true })).toHaveLength(2);
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Root disconnected'));
  fireEvent.click(screen.getByRole('button', { name: /Mountain.png/ }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Root disconnected');
  expect(screen.getByRole('button', { name: 'Retry group' })).toBeEnabled();
});

it('distinguishes an active inspection from a successful empty result', () => {
  render(
    <DuplicateStudio
      job={{ ...job, state: 'running' }}
      groups={null}
      findings={null}
      status={status}
      next={vi.fn()}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Finding duplicate clusters…' })).toBeVisible();
  expect(screen.getByText(/Progress and cancellation are available in Jobs/)).toBeVisible();
  expect(screen.queryByText('No completed duplicate groups')).not.toBeInTheDocument();
});
