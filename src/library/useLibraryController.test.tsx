import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { libraryData } from '../ipc';
import type { LibraryStatus, SelectionToken } from '../bindings';
import { useLibraryController } from './useLibraryController';
import { fixtureAsset } from './__tests__/fixtures';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const assets = ['a', 'b', 'c', 'd'].map((id) => fixtureAsset(id));
const status: LibraryStatus = {
  roots: [],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
function setup() {
  return renderHook(() => useLibraryController({ status, section: 'Photos', onError, onRefresh }));
}
const onError = vi.fn();
const onRefresh = vi.fn(async () => {});
beforeEach(() => {
  onError.mockClear();
  onRefresh.mockClear();
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'query' || request.kind === 'seek_date' || request.kind === 'seek_asset')
        return { assets, total: 500, nextCursor: 'next-page', revision: '1' };
      if (request.kind === 'date_buckets') return [];
      if (request.kind === 'freeze_selection') return { id: 'frozen', count: 500, revision: '1' };
      if (request.kind === 'annotate') return assets.length;
      throw new Error(`Unexpected request ${request.kind}`);
    });
});
it('retains the sought month while progressively loading subsequent rows', async () => {
  const { result } = setup();
  await waitFor(() => expect(result.current.loading).toBe(false));
  act(() => result.current.jump('2020-06'));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'seek_date', period: '2020-06' }),
      'page',
    ),
  );
  await waitFor(() => expect(result.current.feed.busy).toBe(false));
  act(() => result.current.next());
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'query', cursor: 'next-page' }),
      'page',
    ),
  );
  expect(result.current.browse.period).toBe('2020-06');
});
it('selects only the visible ordered range and keeps a failed batch explicit', async () => {
  const { result } = setup();
  await waitFor(() => expect(result.current.loading).toBe(false));
  act(() => result.current.select('b', false));
  act(() => result.current.select('d', true));
  expect([...result.current.selection.ids]).toEqual(['b', 'c', 'd']);
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Catalog write refused'));
  await act(async () => {
    await expect(
      result.current.annotate({ favorite: true, rating: null, tags: null }),
    ).rejects.toThrow('Catalog write refused');
  });
  expect(result.current.selection.count).toBe(3);
  expect(onRefresh).not.toHaveBeenCalled();
  expect(onError).toHaveBeenCalled();
});
it('discards a late all-results selection after the filters change', async () => {
  const { result } = setup();
  await waitFor(() => expect(result.current.loading).toBe(false));
  let resolve!: (token: SelectionToken) => void;
  const deferred = new Promise<SelectionToken>((done) => {
    resolve = done;
  });
  vi.mocked(libraryData).mockImplementationOnce(async () => deferred);
  let freeze!: Promise<boolean>;
  act(() => {
    freeze = result.current.freeze();
  });
  act(() => result.current.filter({ name: 'changed' }));
  await act(async () => {
    resolve({ id: 'old-query', count: 500, revision: '1' });
    await freeze;
  });
  expect(result.current.selection.frozen).toBe(false);
  expect(result.current.selection.count).toBe(0);
});
