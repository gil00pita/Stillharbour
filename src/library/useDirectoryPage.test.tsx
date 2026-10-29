import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { DirectoryPage } from '../bindings';
import { useDirectoryPage } from './useDirectoryPage';
import { libraryData } from '../ipc';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const location = { rootId: 'root', directoryId: 'd', cursor: null, search: '' };
const populated: DirectoryPage = {
  directory: { id: 'd', name: 'Library', state: 'complete' },
  ancestors: [],
  entries: [{ kind: 'folder', directory: { id: 'd01', name: 'Trip', state: 'complete' } }],
  total: 1,
  previousCursor: null,
  nextCursor: null,
  revision: '1',
};
beforeEach(() => vi.mocked(libraryData).mockReset());
it('replaces removed entries when the catalog revision changes', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce(populated)
    .mockResolvedValueOnce({ ...populated, entries: [], total: 0, revision: '2' });
  const { result, rerender } = renderHook(({ revision }) => useDirectoryPage(location, revision), {
    initialProps: { revision: '1' },
  });
  await waitFor(() => expect(result.current.page?.entries).toHaveLength(1));
  rerender({ revision: '2' });
  await waitFor(() => expect(result.current.page?.total).toBe(0));
  expect(result.current.page?.entries).toHaveLength(0);
});
it('recovers from a cursor beyond the new end without treating the whole folder as empty', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ ...populated, entries: [] })
    .mockResolvedValueOnce(populated);
  const { result } = renderHook(() => useDirectoryPage({ ...location, cursor: 'old-end' }, '2'));
  await waitFor(() => expect(result.current.page?.entries).toHaveLength(1));
  expect(libraryData).toHaveBeenCalledTimes(2);
  expect(libraryData).toHaveBeenLastCalledWith(
    expect.objectContaining({ kind: 'browse_directory', cursor: null, limit: 240 }),
    'directory',
  );
});
