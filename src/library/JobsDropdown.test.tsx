import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { LibraryStatus, ScanJob } from '../bindings';
import { library, libraryData } from '../ipc';
import { JobsDropdown } from './JobsDropdown';
import { defaultFilter } from './model';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  library: vi.fn(),
  libraryData: vi.fn(),
}));
const job: ScanJob = {
  id: 'fixture-job',
  rootId: 'fixture-root',
  state: 'running',
  discovered: 40,
  indexed: 12,
  skipped: 1,
  errors: 2,
  message: 'Reading originals',
  startedAt: '2026-10-28',
};
const status: LibraryStatus = {
  roots: [],
  jobs: [job],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
const refresh = vi.fn<() => Promise<void>>();
beforeEach(() => {
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  vi.mocked(libraryData).mockReset();
  refresh.mockReset().mockResolvedValue();
});
function open() {
  fireEvent.click(screen.getByRole('button', { name: 'Jobs' }));
  return screen.getByRole('dialog', { name: 'Jobs' });
}

it('lists every active job with truthful counts and stopping state; keeps history secondary', () => {
  render(
    <JobsDropdown
      status={{
        ...status,
        jobs: [
          job,
          { ...job, id: 'stopping', state: 'cancel_requested' },
          { ...job, id: 'done', state: 'completed' },
        ],
      }}
      onRefresh={refresh}
    />,
  );
  expect(screen.getByRole('button', { name: 'Jobs' })).toHaveAccessibleDescription(
    '2 running jobs',
  );
  const panel = open();
  expect(
    within(panel)
      .getAllByRole('progressbar')
      .filter((bar) => !bar.hasAttribute('aria-valuenow')),
  ).toHaveLength(2);
  expect(within(panel).getByText('Stopping scan…')).toBeVisible();
  expect(within(panel).getByRole('button', { name: 'Stopping…' })).toBeDisabled();
  expect(within(panel).getAllByText('40 found · 12 indexed · 1 skipped · 2 issues')).toHaveLength(
    3,
  );
  expect(within(panel).getByText('Scan complete')).not.toBeVisible();
  fireEvent.click(within(panel).getByText('Recent jobs (1)'));
  expect(within(panel).getByText('Scan complete')).toBeVisible();
});
it('distinguishes connecting from no jobs, and moves completed work to history without navigation', () => {
  const { rerender } = render(<JobsDropdown status={null} onRefresh={refresh} />);
  const panel = open();
  expect(within(panel).getByText('Connecting to jobs…')).toBeVisible();
  rerender(<JobsDropdown status={status} onRefresh={refresh} />);
  expect(within(panel).getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
  rerender(
    <JobsDropdown
      status={{ ...status, jobs: [{ ...job, state: 'completed' }] }}
      onRefresh={refresh}
    />,
  );
  expect(within(panel).getByText('No running jobs.')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Jobs' })).toHaveAccessibleDescription(
    '0 running jobs',
  );
});
it('dismisses with Escape, outside pointer or focus, and restores focus only when appropriate', () => {
  render(
    <>
      <JobsDropdown status={status} onRefresh={refresh} />
      <button>Gallery</button>
    </>,
  );
  expect(within(open()).getByRole('heading', { name: 'Jobs' })).toHaveFocus();
  fireEvent.keyDown(document, { key: 'Escape' });
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Jobs' })).toHaveFocus();
  open();
  fireEvent.pointerDown(screen.getByRole('button', { name: 'Gallery' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  open();
  fireEvent.focusIn(screen.getByRole('button', { name: 'Gallery' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
it('cancels by validated job identity, refreshes status and exposes native failures without fake success', async () => {
  render(<JobsDropdown status={status} onRefresh={refresh} />);
  open();
  vi.mocked(library).mockRejectedValueOnce(new Error('Scan could not be stopped. Try again.'));
  fireEvent.click(screen.getByRole('button', { name: 'Cancel scan' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Scan could not be stopped');
  expect(refresh).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel scan' }));
  await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
  expect(library).toHaveBeenLastCalledWith({ kind: 'cancel', job_id: job.id });
});
it('loads issue details lazily, renders them as text and handles failure', async () => {
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Details unavailable.'));
  render(<JobsDropdown status={status} onRefresh={refresh} />);
  open();
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Details' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Details unavailable');
  fireEvent.click(screen.getByRole('button', { name: 'Hide details' }));
  vi.mocked(libraryData).mockResolvedValueOnce({
    job,
    issues: ['<script>untrusted</script>', '<script>untrusted</script>'],
  });
  fireEvent.click(screen.getByRole('button', { name: 'Details' }));
  expect(await screen.findByText('<script>untrusted</script>')).toBeVisible();
  expect(screen.getAllByRole('listitem')).toHaveLength(1);
  expect(libraryData).toHaveBeenCalledWith({ kind: 'job_details', job_id: job.id }, 'job_details');
});

it('starts a separate thumbnail job for a selected saved library and exposes thumbnail cancellation', async () => {
  render(
    <JobsDropdown
      status={{
        ...status,
        roots: [
          {
            id: 'fixture-root',
            label: 'Generated library',
            displayPath: '/generated',
            mode: 'referenced',
            isBase: true,
            availability: 'online',
            filter: defaultFilter,
            lastScan: null,
            fileCount: 40,
          },
        ],
        jobs: [{ ...job, kind: 'thumbnails', state: 'queued' }],
      }}
      onRefresh={refresh}
    />,
  );
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Generate thumbnails' }));
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({ kind: 'generate_thumbnails', root_id: 'fixture-root' }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Cancel thumbnails' }));
  await waitFor(() => expect(library).toHaveBeenCalledWith({ kind: 'cancel', job_id: job.id }));
});
