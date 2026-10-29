import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import type { ThumbnailKey, ThumbnailResult } from '../bindings';
import { libraryData } from '../ipc';
import { previewKey, usePreviews } from './previews';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const onError = vi.fn();
const keys = [32, 512].map((edge) => ({ assetId: 'photo', revision: 'same-revision', edge }));
function result(key: ThumbnailKey, state = 'ready'): ThumbnailResult {
  return { ...key, state, error: null };
}
afterEach(() => {
  vi.useRealTimers();
  vi.resetAllMocks();
});

it('uses one generation for tiny work and cache-only clear lookups, then enables clear work on settle', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind !== 'thumbnails') throw new Error('Unexpected request');
    return request.keys.map((key) => result(key));
  });
  const { result: hook, rerender } = renderHook(
    ({ threshold }: { threshold: number | undefined }) =>
      usePreviews(keys, onError, true, threshold),
    { initialProps: { threshold: 32 as number | undefined } },
  );
  await waitFor(() => expect(hook.current.results[previewKey(keys[1]!)]?.state).toBe('ready'));
  const requests = vi.mocked(libraryData).mock.calls.map(([request]) => request);
  expect(requests).toHaveLength(2);
  expect(requests[0]).toMatchObject({ kind: 'thumbnails', keys: [keys[0]] });
  expect(requests[1]).toMatchObject({ kind: 'thumbnails', keys: [keys[1]], cache_only: true });
  expect(requests[0]?.kind === 'thumbnails' && requests[0].generation).toBe(
    requests[1]?.kind === 'thumbnails' && requests[1].generation,
  );
  rerender({ threshold: undefined });
  await waitFor(() => expect(libraryData).toHaveBeenCalledTimes(3));
  expect(vi.mocked(libraryData).mock.calls[2]?.[0]).toMatchObject({ kind: 'thumbnails', keys });
  expect(hook.current.results[previewKey(keys[1]!)]?.state).toBe('ready');
});

it('observes a newly completed rendition at200ms rather than waiting750ms', async () => {
  vi.useFakeTimers();
  vi.mocked(libraryData)
    .mockResolvedValueOnce([result(keys[0]!, 'pending')])
    .mockResolvedValueOnce([result(keys[0]!)]);
  const { result: hook } = renderHook(() => usePreviews([keys[0]!], onError));
  await act(async () => {
    await Promise.resolve();
  });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(199);
  });
  expect(libraryData).toHaveBeenCalledTimes(1);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(1);
  });
  expect(libraryData).toHaveBeenCalledTimes(2);
  expect(hook.current.results[previewKey(keys[0]!)]?.state).toBe('ready');
});

it('keeps both cache lookup and work batches within64 keys and480 total', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind !== 'thumbnails') throw new Error('Unexpected request');
    expect(request.keys.length).toBeLessThanOrEqual(64);
    return request.keys.map((key) => result(key));
  });
  const many = Array.from({ length: 500 }, (_, i) => ({ ...keys[i % 2]!, assetId: String(i) }));
  const { result: hook } = renderHook(() => usePreviews(many, onError, true, 32));
  await waitFor(() => expect(Object.keys(hook.current.results)).toHaveLength(480));
  expect(libraryData).toHaveBeenCalledTimes(8);
  expect(onError).not.toHaveBeenCalled();
});
