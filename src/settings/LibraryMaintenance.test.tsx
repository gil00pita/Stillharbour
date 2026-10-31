import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { LibraryMaintenance } from './LibraryMaintenance';
import { libraryData } from '../ipc';
import type { MaintenanceTarget } from '../bindings';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const target: MaintenanceTarget = { kind: 'library', root_id: 'root' };
beforeEach(() => {
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'prepare_maintenance')
        return {
          id: 'native-plan',
          target: request.target,
          rootLabel: 'Generated library',
          rootDisplayPath: 'Generated folder',
          assetCount: 20,
          cachedPreviews: 4,
          cacheBytes: '1000',
        };
      if (request.kind === 'apply_maintenance') return { removedRootId: 'root', clearedEntries: 4 };
      throw new Error('Unexpected request');
    });
});
function setup(value = target, onComplete = vi.fn(async () => {})) {
  const onClose = vi.fn();
  render(<LibraryMaintenance target={value} onClose={onClose} onComplete={onComplete} />);
  return { onClose, onComplete };
}
it('previews scope and warns about lost catalog notes without removing anything before confirmation', async () => {
  const c = setup();
  await screen.findByText('Generated library');
  expect(screen.getByText(/catalog favorites, ratings, tags and scan history/)).toBeVisible();
  expect(libraryData).not.toHaveBeenCalledWith(
    expect.objectContaining({ kind: 'apply_maintenance' }),
    expect.anything(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(c.onClose).toHaveBeenCalledOnce();
  expect(c.onComplete).not.toHaveBeenCalled();
});
it('shows stopping progress until the backend returns a review and explains that cancellation leaves jobs stopped', async () => {
  let resolve!: (value: import('../bindings').MaintenancePreview) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise<import('../bindings').MaintenancePreview>((done) => {
        resolve = done;
      }),
  );
  setup();
  expect(screen.getByRole('status')).toHaveTextContent('Stopping this library’s jobs');
  expect(screen.getByRole('button', { name: 'Stopping jobs…' })).toBeDisabled();
  resolve({
    id: 'native-plan',
    target,
    rootLabel: 'Stopped library',
    rootDisplayPath: 'Generated folder',
    assetCount: 1,
    cachedPreviews: 0,
    cacheBytes: '0',
  });
  await screen.findByText('Stopped library');
  expect(screen.getByText(/Canceling removal does not restart them/)).toBeVisible();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Remove library and its previews' })).toBeEnabled(),
  );
  expect(libraryData).toHaveBeenCalledOnce();
});
it('executes only the backend-issued reviewed plan and reports actual completion', async () => {
  const c = setup();
  await screen.findByText('Generated library');
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Remove library and its previews' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Remove library and its previews' }));
  await waitFor(() =>
    expect(c.onComplete).toHaveBeenCalledWith({ removedRootId: 'root', clearedEntries: 4 }),
  );
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'apply_maintenance', plan_id: 'native-plan' },
    'maintenance_done',
  );
  expect(await screen.findByRole('status')).toHaveTextContent(
    'Original photos and sidecars are unchanged',
  );
});
it('does not claim jobs have stopped when preparing removal fails', async () => {
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Jobs are still stopping. Try again.'));
  setup();
  expect(await screen.findByRole('alert')).toHaveTextContent('Jobs are still stopping');
  expect(screen.queryByText(/active jobs are stopped before review/)).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Remove library and its previews' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Review again' })).toBeEnabled();
});
it('rejects stale/failed execution without success and requires another review before retry', async () => {
  const c = setup();
  await screen.findByText('Generated library');
  vi.mocked(libraryData).mockRejectedValueOnce(
    new Error('The catalog changed. Review this action again.'),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Remove library and its previews' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('The catalog changed');
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
  expect(c.onComplete).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Remove library and its previews' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Review again' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Remove library and its previews' })).toBeEnabled(),
  );
});
it('keeps successful cleanup distinct from a subsequent refresh failure', async () => {
  setup(
    { kind: 'preview_cache' },
    vi.fn(async () => {
      throw new Error('Refresh unavailable');
    }),
  );
  await screen.findByText('Generated library');
  fireEvent.click(screen.getByRole('button', { name: 'Clear all cached previews' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('The action completed');
  expect(screen.getByRole('status')).toHaveTextContent('Libraries and catalog notes are unchanged');
  expect(
    screen.queryByRole('button', { name: 'Clear all cached previews' }),
  ).not.toBeInTheDocument();
});

it('cancels from the initially focused review heading with Escape without applying the plan', async () => {
  const c = setup();
  await screen.findByText('Generated library');
  await waitFor(() => expect(screen.getByRole('button', { name: 'Cancel' })).toBeEnabled());
  const heading = screen.getByRole('heading', { name: 'Remove this library?' });
  expect(heading).toHaveFocus();
  fireEvent.keyDown(heading, { key: 'Escape' });
  expect(c.onClose).toHaveBeenCalledOnce();
  expect(libraryData).not.toHaveBeenCalledWith(
    expect.objectContaining({ kind: 'apply_maintenance' }),
    expect.anything(),
  );
});
