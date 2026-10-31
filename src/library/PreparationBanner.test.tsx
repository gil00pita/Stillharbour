import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { LibraryRoot, LibraryStatus, ScanJob } from '../bindings';
import { library } from '../ipc';
import { defaultFilter } from './model';
import { PreparationBanner } from './PreparationBanner';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  library: vi.fn(),
}));
const root: LibraryRoot = {
  id: 'generated-root',
  label: 'Generated photos',
  displayPath: '/fixture',
  mode: 'referenced',
  isBase: true,
  availability: 'scanning',
  filter: defaultFilter,
  lastScan: null,
  fileCount: 20,
  preparingPreviews: true,
};
const scan: ScanJob = {
  id: 'scan',
  rootId: root.id,
  state: 'running',
  discovered: 20,
  indexed: 20,
  skipped: 0,
  errors: 0,
  message: 'Indexing',
  startedAt: '2026-10-30',
};
const status: LibraryStatus = {
  roots: [root],
  jobs: [scan],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
const refresh = vi.fn(async () => {});
beforeEach(() => {
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  refresh.mockReset().mockResolvedValue();
});

it('keeps discovery indeterminate even when every found file is indexed', () => {
  const { rerender } = render(
    <PreparationBanner status={status} rootId={null} onRefresh={refresh} />,
  );
  expect(screen.getByText('20 files scanned')).toBeVisible();
  expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
  expect(screen.getByRole('progressbar')).toHaveAccessibleDescription(
    'Scanning · total files not yet known',
  );
  expect(screen.getByText('Scanning · total files not yet known')).not.toBeVisible();
  expect(screen.queryByText('Generated photos')).not.toBeInTheDocument();
  expect(screen.queryByText(/0 remaining/)).not.toBeInTheDocument();
  rerender(
    <PreparationBanner
      status={{ ...status, jobs: [{ ...scan, discovered: 27 }] }}
      rootId={null}
      onRefresh={refresh}
    />,
  );
  expect(screen.getByText('27 files scanned')).toBeVisible();
  expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
});

it('shows actual processed previews after inventory, including unavailable outcomes', () => {
  const job = {
    ...scan,
    kind: 'thumbnails' as const,
    discovered: 100,
    indexed: 20,
    skipped: 15,
    errors: 5,
  };
  const { rerender } = render(
    <PreparationBanner status={{ ...status, jobs: [job] }} rootId={null} onRefresh={refresh} />,
  );
  expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '40');
  expect(screen.getByText('40 previews processed')).toBeVisible();
  expect(screen.getByText('5 previews unavailable · Details in Jobs')).toBeVisible();
  expect(screen.getByRole('progressbar')).toHaveAccessibleDescription(
    'Preparing thumbnails · 40 of 100',
  );
  rerender(
    <PreparationBanner
      status={{ ...status, jobs: [{ ...job, discovered: 0, indexed: 0, skipped: 0, errors: 0 }] }}
      rootId={null}
      onRefresh={refresh}
    />,
  );
  expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
  expect(screen.queryByText(/0 remaining/)).not.toBeInTheDocument();
  expect(screen.getByText('0 previews processed')).toBeVisible();
});

it('keeps queued inventory indeterminate and stop requests disabled until confirmed', () => {
  const job = { ...scan, kind: 'thumbnails' as const, state: 'queued' as const };
  const { rerender } = render(
    <PreparationBanner status={{ ...status, jobs: [job] }} rootId={null} onRefresh={refresh} />,
  );
  expect(screen.getByText('20 previews processed')).toBeVisible();
  expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
  rerender(
    <PreparationBanner
      status={{ ...status, jobs: [{ ...job, state: 'cancel_requested' }] }}
      rootId={null}
      onRefresh={refresh}
    />,
  );
  expect(screen.getByRole('button', { name: 'Stopping…' })).toBeDisabled();
  expect(screen.getByRole('progressbar')).toHaveAccessibleDescription(
    'Stopping thumbnail generation…',
  );
});

it('cancels with the native ID and resumes a paused library explicitly', async () => {
  const { rerender } = render(
    <PreparationBanner status={status} rootId={null} onRefresh={refresh} />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Stop scan' }));
  await waitFor(() => expect(library).toHaveBeenCalledWith({ kind: 'cancel', job_id: 'scan' }));
  await waitFor(() => expect(refresh).toHaveBeenCalledOnce());
  rerender(
    <PreparationBanner
      status={{ ...status, jobs: [{ ...scan, state: 'canceled' }] }}
      rootId={null}
      onRefresh={refresh}
    />,
  );
  expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Resume previews' }));
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({ kind: 'generate_thumbnails', root_id: root.id }),
  );
});

it('retains failures, suppresses stale errors after changing folders and hides after completion', async () => {
  let reject: ((reason: Error) => void) | undefined;
  vi.mocked(library).mockImplementation(
    () =>
      new Promise((_, fail) => {
        reject = fail;
      }),
  );
  const { rerender } = render(
    <PreparationBanner status={{ ...status, jobs: [] }} rootId={root.id} onRefresh={refresh} />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Resume previews' }));
  const other = { ...root, id: 'other', label: 'Another folder' };
  rerender(
    <PreparationBanner
      status={{ ...status, roots: [other], jobs: [] }}
      rootId={other.id}
      onRefresh={refresh}
    />,
  );
  await act(async () => {
    reject?.(new Error('Old folder failed'));
  });
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  vi.mocked(library).mockRejectedValue(new Error('Folder offline'));
  fireEvent.click(screen.getByRole('button', { name: 'Resume previews' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Folder offline');
  rerender(
    <PreparationBanner
      status={{ ...status, roots: [{ ...other, preparingPreviews: false }], jobs: [] }}
      rootId={null}
      onRefresh={refresh}
    />,
  );
  expect(screen.queryByRole('region', { name: 'Library preparation' })).not.toBeInTheDocument();
});
