import { act, renderHook, waitFor } from '../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import { libraryData } from '../ipc';
import { defaultQuery } from './model';
import { useAssetFeed } from './useAssetFeed';
import { fixtureAsset } from './__tests__/fixtures';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const onError = vi.fn();
function page(start: number) {
  return {
    startIndex: start,
    assets: Array.from({ length: 240 }, (_, index) => fixtureAsset(String(start + index))),
    nextCursor: start >= 960 ? null : String(start + 240),
    total: 1200,
    revision: '1',
  };
}
beforeEach(() => {
  onError.mockClear();
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'query') return page(Number(request.cursor ?? 0));
      if (request.kind === 'seek_asset')
        return page(
          request.position === 'before'
            ? Math.max(0, Number(request.asset_id) - 240)
            : Number(request.asset_id),
        );
      if (request.kind === 'seek_date') return page(480);
      if (request.kind === 'seek_index') return page(request.index);
      throw new Error('Unexpected request');
    });
});
it('keeps at most720 summaries while loading both directions, without duplicate IDs', async () => {
  const { result } = renderHook(() => useAssetFeed(defaultQuery, null, '1', onError));
  await waitFor(() => expect(result.current.page?.assets).toHaveLength(240));
  for (let index = 0; index < 3; index++) {
    act(() => result.current.next());
    await waitFor(() => expect(result.current.busy).toBe(false));
  }
  expect(result.current.page?.assets).toHaveLength(720);
  expect(result.current.page?.assets[0]?.id).toBe('240');
  expect(result.current.hasBefore).toBe(true);
  act(() => result.current.previous());
  await waitFor(() => expect(result.current.busy).toBe(false));
  expect(result.current.page?.assets[0]?.id).toBe('0');
  expect(result.current.page?.assets).toHaveLength(720);
  expect(new Set(result.current.page?.assets.map((asset) => asset.id)).size).toBe(720);
});
it('serializes repeated edge requests and ignores an old query response', async () => {
  let resolve!: (value: ReturnType<typeof page>) => void;
  const { result, rerender } = renderHook(({ query }) => useAssetFeed(query, null, '1', onError), {
    initialProps: { query: defaultQuery },
  });
  await waitFor(() => expect(result.current.page?.assets).toHaveLength(240));
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  act(() => {
    result.current.next();
    result.current.next();
  });
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.filter(([request]) => request.kind === 'query' && request.cursor === '240'),
  ).toHaveLength(1);
  rerender({ query: { ...defaultQuery, name: 'new' } });
  await waitFor(() => expect(result.current.busy).toBe(false));
  await act(async () => resolve(page(240)));
  expect(result.current.page?.assets[0]?.id).toBe('0');
  expect(result.current.page?.assets).toHaveLength(240);
});
it('keeps loaded photos on a failed continuation and retries explicitly', async () => {
  const { result } = renderHook(() => useAssetFeed(defaultQuery, null, '1', onError));
  await waitFor(() => expect(result.current.page?.assets).toHaveLength(240));
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Catalog busy'));
  act(() => result.current.next());
  await waitFor(() => expect(result.current.error?.message).toBe('Catalog busy'));
  expect(result.current.page?.assets).toHaveLength(240);
  act(() => result.current.retry());
  await waitFor(() => expect(result.current.page?.assets).toHaveLength(480));
  expect(result.current.error).toBeNull();
});
it('refreshes a window from its stable first identity when catalog revision changes', async () => {
  const { result, rerender } = renderHook(
    ({ revision }) => useAssetFeed(defaultQuery, '2026-03', revision, onError),
    { initialProps: { revision: '1' } },
  );
  await waitFor(() => expect(result.current.page?.assets).toHaveLength(240));
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'seek_asset' || request.kind === 'query')
      return { ...page(480), revision: '2' };
    throw new Error('Unexpected');
  });
  rerender({ revision: '2' });
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'seek_asset', asset_id: '480', position: 'at' }),
      'page',
    ),
  );
  await waitFor(() => expect(result.current.busy).toBe(false));
  expect(result.current.page?.assets[0]?.id).toBe('480');
});

it('refreshes the beginning to include new files above the previous first asset', async () => {
  const remember = vi.fn();
  const { result, rerender } = renderHook(
    ({ revision }) => useAssetFeed(defaultQuery, null, revision, onError, remember),
    { initialProps: { revision: '1' } },
  );
  await waitFor(() => expect(result.current.page?.assets).toHaveLength(240));
  vi.mocked(libraryData).mockImplementation(async () => ({ ...page(1000), revision: '2' }));
  rerender({ revision: '2' });
  await waitFor(() => expect(result.current.page?.assets[0]?.id).toBe('1000'));
  expect(remember).toHaveBeenCalledWith(true);
});

it('keeps the rail page mounted and coalesces rapid ordinal seeks to one active and the latest pending target', async () => {
  const { result, rerender } = renderHook(
    ({ index }) => useAssetFeed(defaultQuery, null, '1', onError, undefined, index),
    { initialProps: { index: null as number | null } },
  );
  await waitFor(() => expect(result.current.page?.startIndex).toBe(0));
  let finish!: (value: ReturnType<typeof page>) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  rerender({ index: 100 });
  expect(result.current.page?.startIndex).toBe(0);
  const beforeKey = result.current.key;
  rerender({ index: 200 });
  rerender({ index: 300 });
  expect(result.current.key).toBe(beforeKey);
  expect(
    vi.mocked(libraryData).mock.calls.filter(([request]) => request.kind === 'seek_index'),
  ).toHaveLength(1);
  await act(async () => finish(page(100)));
  await waitFor(() => expect(result.current.page?.startIndex).toBe(300));
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.filter(([request]) => request.kind === 'seek_index')
      .map(([request]) => (request.kind === 'seek_index' ? request.index : null)),
  ).toEqual([100, 300]);
  expect(result.current.page?.assets).toHaveLength(240);
});
