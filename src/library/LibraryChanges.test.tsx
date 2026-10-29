import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { LibraryChange, LibraryChangePage, LibraryStatus } from '../bindings';
import { LibraryChanges } from './LibraryChanges';
import { libraryData } from '../ipc';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const status: LibraryStatus = {
  roots: [],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
const change: LibraryChange = {
  version: 1,
  sequence: '2',
  rootId: 'root',
  rootLabel: 'Photo library',
  assetId: 'photo',
  entryKind: 'file',
  kind: 'moved',
  previousNative: null,
  nativePath: null,
  previousPath: 'Old/<img onerror="alert(1)">.jpg',
  path: 'New/photo.jpg',
  previousRevision: 'old',
  revision: 'new',
  observedAt: '2026-10-29T12:00:00Z',
  scanId: 'scan',
};
beforeEach(() => vi.mocked(libraryData).mockReset());
function expand() {
  const details = screen.getByText('Library changes').closest('details')!;
  details.open = true;
  fireEvent(details, new Event('toggle'));
}
it('loads history only when opened and renders filenames as text', async () => {
  vi.mocked(libraryData).mockResolvedValue({ changes: [change], nextBefore: null });
  const { container } = render(<LibraryChanges status={status} />);
  expect(libraryData).not.toHaveBeenCalled();
  expand();
  expect(await screen.findByText(change.previousPath!)).toBeVisible();
  expect(screen.getByText('New/photo.jpg')).toBeVisible();
  expect(container.querySelector('img')).toBeNull();
  expect(libraryData).toHaveBeenCalledWith({ kind: 'changes', before: null, limit: 25 }, 'changes');
});
it('refreshes open history on catalog changes and pages without loading all records', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ changes: [change], nextBefore: '2' })
    .mockResolvedValueOnce({
      changes: [{ ...change, sequence: '1', kind: 'removed' }],
      nextBefore: null,
    })
    .mockResolvedValue({ changes: [], nextBefore: null });
  const { rerender } = render(<LibraryChanges status={status} />);
  expand();
  fireEvent.click(await screen.findByRole('button', { name: 'Older changes' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenLastCalledWith(
      { kind: 'changes', before: '2', limit: 25 },
      'changes',
    ),
  );
  expect(await screen.findByText('Removed')).toBeVisible();
  rerender(<LibraryChanges status={{ ...status, revision: '2' }} />);
  expect(await screen.findByText('No changes recorded yet.')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Latest changes' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenLastCalledWith(
      { kind: 'changes', before: null, limit: 25 },
      'changes',
    ),
  );
});
it('shows a failed history read and supports retry without claiming there are no changes', async () => {
  vi.mocked(libraryData)
    .mockRejectedValueOnce(new Error('History busy'))
    .mockResolvedValue({ changes: [], nextBefore: null });
  render(<LibraryChanges status={status} />);
  expand();
  expect(await screen.findByRole('alert')).toHaveTextContent('History busy');
  expect(screen.queryByText('No changes recorded yet.')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Retry changes' }));
  expect(await screen.findByText('No changes recorded yet.')).toBeVisible();
});
it('ignores a late history response after collapsing and discloses periodic or offline monitoring', async () => {
  let finish!: (page: LibraryChangePage) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise<LibraryChangePage>((resolve) => {
        finish = resolve;
      }),
  );
  render(
    <LibraryChanges
      status={{
        ...status,
        monitoring: [{ rootId: 'root', mode: 'periodic', message: 'Checking every 30 seconds.' }],
      }}
    />,
  );
  expect(screen.getByText(/Checking every 30 seconds/)).toBeVisible();
  expand();
  const details = screen.getByText('Library changes').closest('details')!;
  details.open = false;
  fireEvent(details, new Event('toggle'));
  await act(async () => finish({ changes: [change], nextBefore: null }));
  expect(screen.queryByText('Moved')).toBeNull();
});
